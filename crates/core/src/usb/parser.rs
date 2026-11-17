use crate::{
    error::HxError,
    models::{Preset, Setlist},
    usb::protocol::{RESPONSE_HEADER_SIZE, RESPONSE_LEN_OFFSET, RESPONSE_PAYLOAD_OFFSET},
};
use openhx_i18n::fl;
use rmpv::decode::read_value;
use std::io::Cursor;

/// Integer key used by Line 6's MessagePack encoding for the preset name field.
const MSGPACK_KEY_PRESET_NAME: u64 = 109;

/// Response envelope key holding the command's status code (`0` on success).
const MSGPACK_KEY_STATUS: u64 = 103;

/// Response envelope key holding the command's result value.
const MSGPACK_KEY_RESULT: u64 = 104;

/// Extracts the MessagePack payload from the first packet of a response.
///
/// Responses are framed as a 16-byte transport header, an 8-byte inner header
/// whose last four bytes are the payload length, then the payload itself.
pub fn response_payload(buf: &[u8], n: usize) -> Result<&[u8], HxError> {
    if n < RESPONSE_PAYLOAD_OFFSET {
        return Err(HxError::protocol(fl!(
            "msgpack-response-too-short",
            len = n
        )));
    }

    let len_bytes: [u8; 4] = buf[RESPONSE_LEN_OFFSET..RESPONSE_PAYLOAD_OFFSET]
        .try_into()
        .map_err(|_| HxError::protocol(fl!("msgpack-response-too-short", len = n)))?;
    let len = u32::from_le_bytes(len_bytes) as usize;

    let end = RESPONSE_PAYLOAD_OFFSET
        .checked_add(len)
        .filter(|end| *end <= n)
        .ok_or_else(|| HxError::protocol(fl!("msgpack-response-truncated", len = len, got = n)))?;

    Ok(&buf[RESPONSE_PAYLOAD_OFFSET..end])
}

/// Returns the total length of a multi-packet response, measured like the
/// stream assembled from each packet's bytes after [`RESPONSE_HEADER_SIZE`].
///
/// Only the first packet carries the inner header: 8 bytes whose last four
/// are the little-endian payload length. The assembled stream therefore
/// holds those 8 bytes followed by the payload.
pub fn stream_len(buf: &[u8], n: usize) -> Result<usize, HxError> {
    if n < RESPONSE_PAYLOAD_OFFSET {
        return Err(HxError::protocol(fl!(
            "msgpack-response-too-short",
            len = n
        )));
    }

    let len_bytes: [u8; 4] = buf[RESPONSE_LEN_OFFSET..RESPONSE_PAYLOAD_OFFSET]
        .try_into()
        .map_err(|_| HxError::protocol(fl!("msgpack-response-too-short", len = n)))?;

    Ok(RESPONSE_PAYLOAD_OFFSET - RESPONSE_HEADER_SIZE + u32::from_le_bytes(len_bytes) as usize)
}

/// Looks up an integer key in a MessagePack map; `None` when `map` is not a
/// map or lacks the key.
pub(crate) fn map_get(map: &rmpv::Value, key: u64) -> Option<&rmpv::Value> {
    map.as_map()?
        .iter()
        .find(|(k, _)| k.as_u64() == Some(key))
        .map(|(_, v)| v)
}

/// Decodes a response envelope (`{102: txn, 103: status, 104: result}`).
fn decode_envelope(payload: &[u8]) -> Result<rmpv::Value, HxError> {
    let envelope = read_value(&mut Cursor::new(payload))?;
    if envelope.is_map() {
        Ok(envelope)
    } else {
        Err(HxError::protocol(fl!("msgpack-response-not-map")))
    }
}

/// Decodes a response envelope and returns its result value, rejecting a
/// non-zero status.
pub(crate) fn response_result(payload: &[u8]) -> Result<rmpv::Value, HxError> {
    let envelope = decode_envelope(payload)?;

    match map_get(&envelope, MSGPACK_KEY_STATUS).and_then(rmpv::Value::as_u64) {
        Some(0) | None => {}
        Some(status) => {
            return Err(HxError::protocol(fl!(
                "msgpack-response-status",
                status = status
            )));
        }
    }

    Ok(map_get(&envelope, MSGPACK_KEY_RESULT)
        .cloned()
        .unwrap_or(rmpv::Value::Nil))
}

/// Returns the status code (key `103`) of a reply envelope, if present.
///
/// The meaning of the code depends on the command: the setlist reply uses
/// `0` for success, while write commands (select, rename) report
/// [`ACK_STATUS_OK`]. The raw code is returned so each caller can judge it.
///
/// [`ACK_STATUS_OK`]: crate::usb::protocol::ACK_STATUS_OK
pub fn response_status(payload: &[u8]) -> Result<Option<u64>, HxError> {
    let envelope = decode_envelope(payload)?;
    Ok(map_get(&envelope, MSGPACK_KEY_STATUS).and_then(rmpv::Value::as_u64))
}

/// Parses the reply to the open-presets command into the device's setlists.
///
/// The result is an array of one-entry maps, each `{setlist_index: name}`.
/// Single-setlist devices report one entry.
pub fn parse_setlists(payload: &[u8]) -> Result<Vec<Setlist>, HxError> {
    let result = response_result(payload)?;

    let items = result
        .as_array()
        .ok_or_else(|| HxError::protocol(fl!("msgpack-setlists-not-array")))?;

    let mut setlists: Vec<Setlist> = items
        .iter()
        .map(|item| {
            let map = item
                .as_map()
                .ok_or_else(|| HxError::protocol(fl!("msgpack-setlist-not-map")))?;
            let (key, value) = map
                .first()
                .ok_or_else(|| HxError::protocol(fl!("msgpack-setlist-map-empty")))?;

            let index = key
                .as_u64()
                .and_then(|i| u8::try_from(i).ok())
                .ok_or_else(|| HxError::protocol(fl!("msgpack-setlist-index-not-int")))?;

            let name = value
                .as_str()
                .ok_or_else(|| {
                    HxError::protocol(fl!("msgpack-setlist-name-invalid", index = index))
                })?
                .trim_end_matches('\0')
                .trim_end()
                .to_owned();

            Ok(Setlist::new(index, name))
        })
        .collect::<Result<_, HxError>>()?;

    setlists.sort_unstable_by_key(|s| s.index);
    Ok(setlists)
}

/// Parses a raw MessagePack payload received from the device into a list of Presets.
///
/// The function scans the byte stream for a specific 3-byte array marker that indicates
/// the start of the preset list, then decodes the subsequent MessagePack structures.
///
/// A preset's slot is its **position** in the array: the position is what the
/// device displays and what [`build_select_preset_request`] addresses. Each
/// entry's outer key (`setlist × preset_count + slot` in the common case) can
/// go stale when presets are reordered, so it is only checked to fall within
/// `setlist`'s range.
///
/// [`build_select_preset_request`]: crate::usb::protocol::build_select_preset_request
pub fn parse_msgpack_stream(
    data: &[u8],
    preset_count: u16,
    setlist: u8,
) -> Result<Vec<Preset>, HxError> {
    let [hi, lo] = preset_count.to_be_bytes();
    let marker = [0xDC, hi, lo];

    let start = data
        .windows(3)
        .position(|w| w == marker)
        .ok_or(HxError::InvalidStreamMarker)?;

    let mut cursor = Cursor::new(&data[start..]);
    let root = read_value(&mut cursor)?;

    let items = root
        .as_array()
        .ok_or_else(|| HxError::protocol(fl!("msgpack-root-not-array")))?;

    // The marker encodes `preset_count`, so a decoded array of any other
    // length means the stream is corrupt.
    if items.len() != usize::from(preset_count) {
        return Err(HxError::protocol(fl!(
            "msgpack-preset-count-mismatch",
            expected = preset_count,
            got = items.len()
        )));
    }

    items
        .iter()
        .enumerate()
        .map(|(position, item)| {
            let slot = u8::try_from(position).map_err(|_| {
                HxError::protocol(fl!("msgpack-preset-slot-overflow", slot = position))
            })?;
            parse_preset(item, slot, preset_count, setlist)
        })
        .collect()
}

/// Parses the preset at array position `slot`.
///
/// Expected layout:
/// ```text
/// fixmap(1) { <index: uint> → fixmap { ..., 109: <name: str>, ... } }
/// ```
///
/// `index` is a device-global preset index. It usually equals
/// `setlist × preset_count + slot`, but devices have been seen reporting
/// swapped indices after presets were reordered, so it only has to fall
/// within `setlist`'s range; the slot always comes from the position.
fn parse_preset(
    item: &rmpv::Value,
    slot: u8,
    preset_count: u16,
    setlist: u8,
) -> Result<Preset, HxError> {
    let map = item
        .as_map()
        .ok_or_else(|| HxError::protocol(fl!("msgpack-preset-not-map")))?;

    let (key, value) = map
        .first()
        .ok_or_else(|| HxError::protocol(fl!("msgpack-preset-map-empty")))?;

    let index = key
        .as_u64()
        .ok_or_else(|| HxError::protocol(fl!("msgpack-preset-index-not-int")))?;

    let base = u64::from(setlist) * u64::from(preset_count);
    let in_setlist = index
        .checked_sub(base)
        .is_some_and(|offset| offset < u64::from(preset_count));
    if !in_setlist {
        return Err(HxError::protocol(fl!(
            "msgpack-preset-index-out-of-setlist",
            index = index,
            setlist = setlist
        )));
    }

    if index - base != u64::from(slot) {
        tracing::debug!(
            slot,
            index,
            setlist,
            "preset index key differs from array position"
        );
    }

    let inner_map = value
        .as_map()
        .ok_or_else(|| HxError::protocol(fl!("msgpack-preset-inner-not-map", index = slot)))?;

    let name = inner_map
        .iter()
        .find(|(k, _)| k.as_u64() == Some(MSGPACK_KEY_PRESET_NAME))
        .and_then(|(_, v)| v.as_str())
        .ok_or_else(|| HxError::protocol(fl!("msgpack-preset-name-not-found", index = slot)))?
        .trim_end_matches('\0')
        .to_owned();

    Ok(Preset::new(slot, name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmpv::Value;

    // Helper functions

    fn encode_preset_bytes(index: u16, name: &str) -> Vec<u8> {
        let name_null = format!("{name}\0");
        let name_bytes = name_null.as_bytes();
        let name_len = name_bytes.len();

        let mut buf = vec![0x81]; // fixmap(1)
        match index {
            0..=127 => buf.push(index as u8), // positive fixint
            128..=255 => buf.extend_from_slice(&[0xCC, index as u8]), // uint8
            _ => {
                buf.push(0xCD); // uint16
                buf.extend_from_slice(&index.to_be_bytes());
            }
        }
        buf.push(0x81); // fixmap(1)
        buf.push(MSGPACK_KEY_PRESET_NAME as u8); // fixint 109 = 0x6D

        match name_len {
            0..=31 => buf.push(0xA0 | name_len as u8),
            32..=255 => {
                buf.push(0xD9); // str8
                buf.push(name_len as u8);
            }
            256..=65535 => {
                buf.push(0xDA); // str16
                buf.push((name_len >> 8) as u8);
                buf.push(name_len as u8);
            }
            _ => panic!("test preset name too long"),
        }
        buf.extend_from_slice(name_bytes);
        buf
    }

    fn build_stream(preamble: &[u8], presets: &[(u16, &str)]) -> Vec<u8> {
        let count = presets.len() as u16;
        let mut buf = preamble.to_vec();
        buf.push(0xDC); // array16 tag
        buf.extend_from_slice(&count.to_be_bytes());
        for &(index, name) in presets {
            buf.extend(encode_preset_bytes(index, name));
        }
        buf
    }

    // parse_msgpack_stream tests

    /// Reply payload to the open-presets command, captured from a Helix Floor.
    const SETLIST_REPLY: &[u8] = b"\x83\x66\xCD\x03\xE9\x67\x00\x68\x98\
\x81\xCD\x00\x00\xAAFACTORY 1\x00\
\x81\xCD\x00\x01\xAAFACTORY 2\x00\
\x81\xCD\x00\x02\xA7USER 1\x00\
\x81\xCD\x00\x03\xA7USER 2\x00\
\x81\xCD\x00\x04\xA7USER 3\x00\
\x81\xCD\x00\x05\xA7USER 4\x00\
\x81\xCD\x00\x06\xA7USER 5\x00\
\x81\xCD\x00\x07\xAATEMPLATES\x00";

    #[test]
    fn setlists_parse_from_captured_reply() {
        let setlists = parse_setlists(SETLIST_REPLY).unwrap();

        assert_eq!(setlists.len(), 8);
        assert_eq!(setlists[0], Setlist::new(0, "FACTORY 1"));
        assert_eq!(setlists[2], Setlist::new(2, "USER 1"));
        assert_eq!(setlists[7], Setlist::new(7, "TEMPLATES"));
    }

    #[test]
    fn setlists_reject_non_zero_status() {
        // {102: 1001, 103: 5, 104: []}
        let payload = b"\x83\x66\xCD\x03\xE9\x67\x05\x68\x90";
        let err = parse_setlists(payload).unwrap_err();
        assert!(matches!(err, HxError::Protocol(_)));
    }

    #[test]
    fn response_status_reads_code_from_captured_select_ack() {
        // HX Stomp ACK to a successful selection: {102: 1010, 103: 1, 104: nil}
        let payload = b"\x83\x66\xCD\x03\xF2\x67\x01\x68\xC0";
        assert_eq!(response_status(payload).unwrap(), Some(1));
    }

    #[test]
    fn response_status_reads_code_from_captured_rename_ack() {
        // HX Stomp ACK to a successful rename: {102: 1005, 103: 1, 104: nil}
        let payload = b"\x83\x66\xCD\x03\xED\x67\x01\x68\xC0";
        assert_eq!(
            response_status(payload).unwrap(),
            Some(crate::usb::protocol::ACK_STATUS_OK)
        );
    }

    #[test]
    fn response_status_is_none_without_status_key() {
        // {102: 1005}
        assert_eq!(response_status(b"\x81\x66\xCD\x03\xED").unwrap(), None);
    }

    #[test]
    fn response_status_rejects_non_map_envelope() {
        assert!(matches!(
            response_status(b"\x01"),
            Err(HxError::Protocol(_))
        ));
    }

    #[test]
    fn response_payload_slices_by_declared_length() {
        let mut buf = vec![0u8; 32];
        buf[RESPONSE_LEN_OFFSET..RESPONSE_PAYLOAD_OFFSET].copy_from_slice(&4u32.to_le_bytes());
        buf[RESPONSE_PAYLOAD_OFFSET..RESPONSE_PAYLOAD_OFFSET + 4]
            .copy_from_slice(b"\x01\x02\x03\x04");

        assert_eq!(response_payload(&buf, 32).unwrap(), b"\x01\x02\x03\x04");
    }

    #[test]
    fn stream_len_counts_inner_header_and_payload() {
        // First packet of the HX Stomp preset stream: payload length 0x0CBC.
        let mut buf = vec![0u8; 272];
        buf[RESPONSE_LEN_OFFSET..RESPONSE_PAYLOAD_OFFSET].copy_from_slice(&0x0CBCu32.to_le_bytes());

        assert_eq!(stream_len(&buf, 272).unwrap(), 8 + 0x0CBC);
    }

    #[test]
    fn stream_len_rejects_packet_without_inner_header() {
        assert!(stream_len(&[0u8; 16], 16).is_err());
    }

    #[test]
    fn response_payload_rejects_truncated_response() {
        let mut buf = vec![0u8; 32];
        buf[RESPONSE_LEN_OFFSET..RESPONSE_PAYLOAD_OFFSET].copy_from_slice(&999u32.to_le_bytes());

        assert!(response_payload(&buf, 32).is_err());
        assert!(response_payload(&buf, 8).is_err());
    }

    #[test]
    fn stream_multiple_presets_round_trip() {
        let presets = [(0, "Alpha"), (1, "Beta"), (2, "Gamma")];
        let stream = build_stream(&[], &presets);
        let result = parse_msgpack_stream(&stream, presets.len() as u16, 0).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "Alpha");
        assert_eq!(result[1].name, "Beta");
        assert_eq!(result[2].name, "Gamma");
    }

    #[test]
    fn stream_skips_preamble_bytes() {
        let preamble = &[0x00, 0xFF, 0x42, 0x13, 0x37];
        let stream = build_stream(preamble, &[(0, "Crunch")]);
        let result = parse_msgpack_stream(&stream, 1, 0).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].index, 0);
        assert_eq!(result[0].name, "Crunch");
    }

    #[test]
    fn stream_indices_are_global_across_setlists() {
        // Setlist 64 of a 2-preset device: entries carry indices 128 and 129.
        let stream = build_stream(&[], &[(128, "First"), (129, "Second")]);
        let result = parse_msgpack_stream(&stream, 2, 64).unwrap();

        assert_eq!(result[0].index, 0);
        assert_eq!(result[0].name, "First");
        assert_eq!(result[1].index, 1);
        assert_eq!(result[1].name, "Second");
    }

    #[test]
    fn stream_uint16_indices_decode() {
        // Setlist 2 of a 128-preset device: indices 256..=383 arrive as uint16.
        let names: Vec<String> = (0..128).map(|i| format!("P{i}")).collect();
        let presets: Vec<(u16, &str)> = names
            .iter()
            .enumerate()
            .map(|(i, name)| (256 + i as u16, name.as_str()))
            .collect();
        let stream = build_stream(&[], &presets);
        let result = parse_msgpack_stream(&stream, 128, 2).unwrap();

        assert_eq!(result.len(), 128);
        assert_eq!(result[0].index, 0);
        assert_eq!(result[0].name, "P0");
        assert_eq!(result[127].index, 127);
        assert_eq!(result[127].name, "P127");
    }

    #[test]
    fn stream_slot_comes_from_array_position_not_index_key() {
        // Captured from an HX Stomp after reordering: positions 1 and 2 carry
        // each other's index keys, while the array is in device slot order.
        let stream = build_stream(
            &[],
            &[
                (0, "TANK86"),
                (2, "DOOM"),
                (1, "DUSKHEAD"),
                (3, "New Preset"),
            ],
        );
        let result = parse_msgpack_stream(&stream, 4, 0).unwrap();

        assert_eq!(result[0], Preset::new(0, "TANK86"));
        assert_eq!(result[1], Preset::new(1, "DOOM"));
        assert_eq!(result[2], Preset::new(2, "DUSKHEAD"));
        assert_eq!(result[3], Preset::new(3, "New Preset"));
    }

    #[test]
    fn stream_slot_uses_position_in_later_setlists() {
        // Setlist 1 of a 2-preset device with swapped keys 3 and 2.
        let stream = build_stream(&[], &[(3, "First"), (2, "Second")]);
        let result = parse_msgpack_stream(&stream, 2, 1).unwrap();

        assert_eq!(result[0], Preset::new(0, "First"));
        assert_eq!(result[1], Preset::new(1, "Second"));
    }

    #[test]
    fn stream_truncated_array_returns_error() {
        // The marker announces 3 entries but the stream ends after 2.
        let mut stream = build_stream(&[], &[(0, "A"), (1, "B")]);
        stream[1..3].copy_from_slice(&3u16.to_be_bytes());
        assert!(parse_msgpack_stream(&stream, 3, 0).is_err());
    }

    #[test]
    fn stream_index_outside_requested_setlist_returns_error() {
        let stream = build_stream(&[], &[(5, "Stray")]);
        let err = parse_msgpack_stream(&stream, 1, 1).unwrap_err();
        assert!(matches!(err, HxError::Protocol(_)));
    }

    #[test]
    fn stream_null_terminator_is_stripped_from_name() {
        // Firmware often pads with nulls; we must strip them
        let stream = build_stream(&[], &[(0, "Solo\0\0")]);
        let result = parse_msgpack_stream(&stream, 1, 0).unwrap();

        assert!(!result[0].name.contains('\0'));
        assert_eq!(result[0].name, "Solo");
    }

    #[test]
    fn stream_marker_not_found_returns_error() {
        let garbage = vec![0x00u8, 0x01, 0x02, 0xFF, 0xAB];
        let err = parse_msgpack_stream(&garbage, 128, 0).unwrap_err();
        assert!(matches!(err, HxError::InvalidStreamMarker));
    }

    // parse_preset tests

    #[test]
    fn preset_valid_parses_correctly() {
        let inner = Value::Map(vec![(
            Value::Integer(MSGPACK_KEY_PRESET_NAME.into()),
            Value::String("Crunch\0".into()),
        )]);
        let outer = Value::Map(vec![(Value::Integer(42.into()), inner)]);

        let preset = parse_preset(&outer, 42, 128, 0).unwrap();
        assert_eq!(preset.index, 42);
        assert_eq!(preset.name, "Crunch");
    }

    #[test]
    fn preset_extra_keys_in_inner_map_are_ignored() {
        // Ensures resilience against future firmware adding new data fields
        let inner = Value::Map(vec![
            (Value::Integer(1u64.into()), Value::Integer(999.into())),
            (
                Value::Integer(MSGPACK_KEY_PRESET_NAME.into()),
                Value::String("Jazz\0".into()),
            ),
            (Value::Integer(200u64.into()), Value::Boolean(true)),
        ]);
        let outer = Value::Map(vec![(Value::Integer(0u64.into()), inner)]);

        let preset = parse_preset(&outer, 0, 128, 0).unwrap();
        assert_eq!(preset.name, "Jazz");
    }

    #[test]
    fn preset_item_not_a_map_returns_error() {
        let item = Value::Integer(42.into());
        let err = parse_preset(&item, 0, 128, 0).unwrap_err();
        assert!(matches!(err, HxError::Protocol(_)));
    }

    #[test]
    fn preset_name_key_missing_returns_error() {
        let inner = Value::Map(vec![(
            Value::Integer(110u64.into()), // Wrong key
            Value::String("Name\0".into()),
        )]);
        let outer = Value::Map(vec![(Value::Integer(0u64.into()), inner)]);
        let err = parse_preset(&outer, 0, 128, 0).unwrap_err();
        assert!(matches!(err, HxError::Protocol(_)));
    }
}
