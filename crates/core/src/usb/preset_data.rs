//! Parses the active preset's data, as streamed over the edit channel.
//!
//! The stream holds the 8-byte inner header and a response envelope
//! `{102: 1012, 103: 0, 104: <blob>}`. The blob is a sequence of MessagePack
//! values:
//!
//! ```text
//! "l6-helix\0"          magic
//! <str, 48 bytes>       offset table (12 × u32 LE), not needed here
//! { 0: { 22: [20 slots], … }, 7: { 37: firmware version, … }, … }
//! ```
//!
//! The 20 slots are the two signal paths in order. In each path, slot 0 is
//! the input, slots 1–8 are the block positions and slot 9 is the output:
//!
//! ```text
//! { 19: type, 20: body }
//!   type 0–3   path input/output; not a block
//!   type 2     path 2's input: { 15: split }
//!   type 3     path 2's output: { 17: join }
//!   type 6     block: { 24: { 23: dual, 25: model, 26: second model },
//!                       10: enabled, 11: params, 12: second params }
//!   type 8     empty position (body is nil)
//!   other      not decoded yet (e.g. type 7, the looper); skipped
//! params:     { 2: count, 3: …, 4: [values] }
//! split/join: { 8: model, 13: slot, 10: enabled, 7: params }
//! ```
//!
//! A split or join `slot` is the index within path 1 of the slot it sits
//! before: `3` lies between the second and third position.

use crate::{
    blocks::{Block, BlockKind, POSITIONS_PER_PATH, ParamValue, PathSplit, PresetChain},
    error::HxError,
    models::ActivePreset,
    usb::{
        parser::{map_get as get, response_result},
        protocol::{RESPONSE_HEADER_SIZE, RESPONSE_PAYLOAD_OFFSET},
    },
};
use openhx_i18n::fl;
use rmpv::{Value, decode::read_value};
use std::io::Cursor;

/// Magic string at the start of the preset blob.
const PRESET_MAGIC: &[u8] = b"l6-helix\0";

/// Key of the map holding the signal paths in the preset body.
const KEY_FLOW: u64 = 0;
/// Key of the slot array within the flow map.
const KEY_SLOTS: u64 = 22;
/// Slot keys.
const KEY_SLOT_TYPE: u64 = 19;
const KEY_SLOT_BODY: u64 = 20;
/// Block body keys.
const KEY_MODELS: u64 = 24;
const KEY_DUAL: u64 = 23;
const KEY_MODEL_A: u64 = 25;
const KEY_MODEL_B: u64 = 26;
const KEY_ENABLED: u64 = 10;
const KEY_PARAMS_A: u64 = 11;
const KEY_PARAMS_B: u64 = 12;
/// Key of the split in path 2's input and of the join in its output.
const KEY_SPLIT: u64 = 15;
const KEY_JOIN: u64 = 17;
/// Key of the slot a split or join sits before.
const KEY_SPLIT_SLOT: u64 = 13;

/// Parameter map key holding the value array.
const KEY_PARAM_VALUES: u64 = 4;

/// Slot type of a regular block.
const SLOT_TYPE_BLOCK: u64 = 6;

/// Slots per signal path: input, the block positions, output.
const SLOTS_PER_PATH: usize = POSITIONS_PER_PATH as usize + 2;

/// Result keys of the active-preset-info reply.
const KEY_INFO_SETLIST: u64 = 107;
const KEY_INFO_PRESET: u64 = 108;

/// Parses the reply to [`READ_ACTIVE_PRESET_INFO`] into the active preset.
///
/// [`READ_ACTIVE_PRESET_INFO`]: crate::usb::protocol::READ_ACTIVE_PRESET_INFO
pub fn parse_active_preset_info(
    payload: &[u8],
    preset_count: u16,
) -> Result<ActivePreset, HxError> {
    let result = response_result(payload)?;
    let field = |key| {
        get(&result, key)
            .and_then(Value::as_u64)
            .and_then(|v| u8::try_from(v).ok())
    };

    let setlist = field(KEY_INFO_SETLIST).ok_or_else(|| invalid("active setlist"))?;
    let preset = field(KEY_INFO_PRESET)
        .filter(|p| u16::from(*p) < preset_count)
        .ok_or_else(|| invalid("active preset"))?;

    Ok(ActivePreset { setlist, preset })
}

/// Parses the assembled active-preset stream into the preset's blocks.
///
/// Empty positions and the paths' inputs and outputs are left out; the
/// blocks are returned in signal order, path by path.
pub fn parse_active_preset(stream: &[u8]) -> Result<PresetChain, HxError> {
    let envelope = stream
        .get(RESPONSE_PAYLOAD_OFFSET - RESPONSE_HEADER_SIZE..)
        .ok_or_else(|| invalid("envelope"))?;
    let blob = response_result(envelope)?;
    let blob = blob.as_slice().ok_or_else(|| invalid("blob"))?;

    let mut cursor = Cursor::new(blob);
    let magic = read_value(&mut cursor)?;
    if magic.as_slice() != Some(PRESET_MAGIC) {
        return Err(invalid("magic"));
    }
    // Offset table; the body below is self-describing, so it is not needed.
    read_value(&mut cursor)?;
    let body = read_value(&mut cursor)?;

    let slots = get(&body, KEY_FLOW)
        .and_then(|flow| get(flow, KEY_SLOTS))
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("slots"))?;

    let mut blocks = Vec::new();
    for (index, slot) in slots.iter().enumerate() {
        let path = index / SLOTS_PER_PATH;
        let in_path = index % SLOTS_PER_PATH;
        // Slot 0 and the last slot of each path are its input and output.
        if !(1..=usize::from(POSITIONS_PER_PATH)).contains(&in_path) {
            continue;
        }
        let (Ok(path), Ok(position)) = (u8::try_from(path), u8::try_from(in_path - 1)) else {
            return Err(invalid("slot index"));
        };
        if let Some(block) = parse_slot(slot, path, position)? {
            blocks.push(block);
        }
    }

    Ok(PresetChain {
        blocks,
        split: path_split(slots),
    })
}

/// Reads where path 2 branches off path 1 and merges back.
fn path_split(slots: &[Value]) -> Option<PathSplit> {
    // The split sits in path 2's input slot, the join in its output slot.
    let gap = |slot: usize, key| {
        let index = slots
            .get(slot)
            .and_then(|s| get(s, KEY_SLOT_BODY))
            .and_then(|body| get(body, key))
            .and_then(|node| get(node, KEY_SPLIT_SLOT))
            .and_then(Value::as_u64)?;
        // Slot `n` of a path is position `n - 1`; the gap before it has the
        // same number as that position.
        let gap = index.saturating_sub(1).min(u64::from(POSITIONS_PER_PATH));
        u8::try_from(gap).ok()
    };

    Some(PathSplit {
        split: gap(SLOTS_PER_PATH, KEY_SPLIT)?,
        join: gap(2 * SLOTS_PER_PATH - 1, KEY_JOIN)?,
    })
}

/// Parses one block position; `None` when it is empty.
fn parse_slot(slot: &Value, path: u8, position: u8) -> Result<Option<Block>, HxError> {
    let slot_type = get(slot, KEY_SLOT_TYPE)
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid("slot type"))?;
    let body = get(slot, KEY_SLOT_BODY);

    let block = match (slot_type, body) {
        (SLOT_TYPE_BLOCK, Some(body)) => {
            let models = get(body, KEY_MODELS).ok_or_else(|| invalid("block models"))?;
            let model = |key| {
                get(models, key)
                    .and_then(Value::as_u64)
                    .and_then(|id| u16::try_from(id).ok())
            };
            let model_a = model(KEY_MODEL_A).ok_or_else(|| invalid("block model"))?;
            let dual = get(models, KEY_DUAL).and_then(Value::as_bool) == Some(true);

            let kind = match (dual, model(KEY_MODEL_B)) {
                (true, Some(cab)) => BlockKind::AmpCab { amp: model_a, cab },
                _ => BlockKind::Model(model_a),
            };

            Block {
                path,
                position,
                kind,
                enabled: enabled(body),
                params: params(get(body, KEY_PARAMS_A)),
                cab_params: params(get(body, KEY_PARAMS_B)),
            }
        }
        (slot_type, _) => {
            // Types 0–3 are path inputs/outputs and 8 is an empty position.
            // Anything else (e.g. 7, the looper) is a block type that has not
            // been decoded yet.
            if !matches!(slot_type, 0..=3 | 8) {
                tracing::warn!(path, position, slot_type, "skipping undecoded block type");
            }
            return Ok(None);
        }
    };

    Ok(Some(block))
}

/// Reads a block's on/off state; blocks without one are treated as on.
fn enabled(body: &Value) -> bool {
    get(body, KEY_ENABLED)
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

/// Reads the value array of a parameter map.
fn params(map: Option<&Value>) -> Vec<ParamValue> {
    map.and_then(|m| get(m, KEY_PARAM_VALUES))
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(param_value).collect())
        .unwrap_or_default()
}

fn param_value(value: &Value) -> Option<ParamValue> {
    match value {
        Value::F32(v) => Some(ParamValue::Float(f64::from(*v))),
        Value::F64(v) => Some(ParamValue::Float(*v)),
        Value::Boolean(v) => Some(ParamValue::Bool(*v)),
        Value::Integer(v) => v.as_i64().map(ParamValue::Int),
        Value::String(s) => Some(ParamValue::Text(
            String::from_utf8_lossy(s.as_bytes()).into_owned(),
        )),
        _ => None,
    }
}

fn invalid(field: &str) -> HxError {
    HxError::protocol(fl!("preset-data-invalid", field = field))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::BlockCategory;

    /// Active-preset stream captured from an HX Stomp (firmware 3.71).
    const HX_STOMP_STREAM: &[u8] = include_bytes!("fixtures/active_preset_hx_stomp.bin");

    /// The factory preset "Cathedral", captured from an HX Stomp (3.71):
    /// path 2 branches off after the second block and merges back before
    /// the reverb in position 5.
    const CATHEDRAL_STREAM: &[u8] = include_bytes!("fixtures/active_preset_cathedral_hx_stomp.bin");

    #[test]
    fn captured_streams_carry_split_and_join() {
        let cathedral = parse_active_preset(CATHEDRAL_STREAM).unwrap();
        assert_eq!(cathedral.split, Some(PathSplit { split: 2, join: 4 }));
        let path2: Vec<_> = cathedral
            .blocks
            .iter()
            .filter(|b| b.path == 1)
            .map(|b| b.position)
            .collect();
        assert_eq!(path2, [3]);

        // Delays and reverbs end with their trails switch; Pitch Echo stores
        // tempo sync before the note value.
        for block in &cathedral.blocks {
            assert!(block.param_defs().is_some(), "{}", block.name());
        }
        let pitch_echo = cathedral.blocks[2].param_defs().unwrap();
        let tail: Vec<_> = pitch_echo.iter().rev().take(3).map(|d| d.id).collect();
        assert_eq!(tail, ["@trails", "SyncSelect1", "TempoSync1"]);

        // Two cabs in parallel after the amp in position 4.
        let dual_cab = parse_active_preset(HX_STOMP_STREAM).unwrap();
        assert_eq!(dual_cab.split, Some(PathSplit { split: 4, join: 6 }));
    }

    #[test]
    fn captured_stream_yields_its_blocks_in_signal_order() {
        let blocks = parse_active_preset(HX_STOMP_STREAM).unwrap().blocks;

        let summary: Vec<_> = blocks
            .iter()
            .map(|b| (b.path, b.position, b.kind.clone(), b.enabled))
            .collect();
        assert_eq!(
            summary,
            vec![
                (0, 0, BlockKind::Model(582), true),
                (0, 3, BlockKind::Model(349), true),
                (0, 5, BlockKind::Model(72), true),
                (1, 4, BlockKind::Model(72), true),
            ]
        );
    }

    #[test]
    fn captured_stream_carries_parameter_values() {
        let blocks = parse_active_preset(HX_STOMP_STREAM).unwrap().blocks;

        let drive = &blocks[0];
        assert_eq!(drive.category(), Some(BlockCategory::Distortion));
        assert_eq!(drive.params.len(), 6);
        assert_eq!(drive.params[0], ParamValue::Float(0.0));
        assert_eq!(drive.params[1], ParamValue::Int(1));
        assert!(matches!(drive.params[2], ParamValue::Float(v) if (v - 0.37).abs() < 1e-3));
        assert_eq!(drive.params[4], ParamValue::Bool(false));

        let amp = &blocks[1];
        assert_eq!(amp.name(), "Line 6 Badonk (mono)");
        assert_eq!(amp.params.len(), 11);
        assert!(amp.cab_params.is_empty());
    }

    /// Payload of the active-preset-info reply, captured from an HX Stomp:
    /// `{102: 1028, 103: 0, 104: {107: 0, 108: 31, 109: "New Preset", …}}`.
    const ACTIVE_INFO_REPLY: &[u8] = &[
        0x83, 0x66, 0xCD, 0x04, 0x04, 0x67, 0x00, 0x68, 0x86, 0x6B, 0xCD, 0x00, 0x00, 0x6C, 0xCD,
        0x00, 0x1F, 0x6D, 0xAB, b'N', b'e', b'w', b' ', b'P', b'r', b'e', b's', b'e', b't', 0x00,
        0x75, 0xC2, 0x53, 0x92, 0xCD, 0x1E, 0xF8, 0x00, 0x5C, 0x00,
    ];

    #[test]
    fn captured_blocks_match_their_parameter_definitions() {
        let blocks = parse_active_preset(HX_STOMP_STREAM).unwrap().blocks;

        for block in &blocks {
            assert!(block.param_defs().is_some(), "{}", block.name());
        }
        let cab = blocks[2].param_defs().unwrap();
        assert_eq!(cab[2].format(&blocks[2].params[2]), "8.0 kHz");
    }

    #[test]
    fn active_preset_info_reads_setlist_and_slot() {
        let active = parse_active_preset_info(ACTIVE_INFO_REPLY, 126).unwrap();
        assert_eq!(
            active,
            ActivePreset {
                setlist: 0,
                preset: 31
            }
        );
    }

    #[test]
    fn active_preset_info_rejects_slot_beyond_preset_count() {
        assert!(matches!(
            parse_active_preset_info(ACTIVE_INFO_REPLY, 31),
            Err(HxError::Protocol(_))
        ));
    }

    #[test]
    fn stream_without_magic_is_rejected() {
        let mut stream = HX_STOMP_STREAM.to_vec();
        let at = stream
            .windows(PRESET_MAGIC.len())
            .position(|w| w == PRESET_MAGIC)
            .unwrap();
        stream[at] = b'x';

        assert!(matches!(
            parse_active_preset(&stream),
            Err(HxError::Protocol(_))
        ));
    }

    #[test]
    fn truncated_stream_is_rejected() {
        assert!(parse_active_preset(&HX_STOMP_STREAM[..64]).is_err());
        assert!(parse_active_preset(&[]).is_err());
    }
}
