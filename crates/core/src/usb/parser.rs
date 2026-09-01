use crate::{error::HxError, models::Preset};
use openhx_i18n::fl;
use rmpv::decode::read_value;
use std::io::Cursor;

/// Integer key used by Line 6's MessagePack encoding for the preset name field.
const MSGPACK_KEY_PRESET_NAME: u64 = 109;

/// Parses a raw MessagePack payload received from the device into a list of Presets.
///
/// The function scans the byte stream for a specific 3-byte array marker that indicates
/// the start of the preset list, then decodes the subsequent MessagePack structures.
///
/// The device addresses presets globally as `setlist × preset_count + slot`;
/// every entry is checked to belong to `setlist` and returned with its slot
/// as [`Preset::index`].
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

    items
        .iter()
        .map(|item| parse_preset(item, preset_count, setlist))
        .collect()
}

/// Parses one preset slot.
///
/// Expected layout:
/// ```text
/// fixmap(1) { <index: uint> → fixmap { ..., 109: <name: str>, ... } }
/// ```
///
/// `index` is the device-global preset index; it is converted to a slot
/// within `setlist`.
fn parse_preset(item: &rmpv::Value, preset_count: u16, setlist: u8) -> Result<Preset, HxError> {
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
    let slot = index
        .checked_sub(base)
        .filter(|slot| *slot < u64::from(preset_count))
        .and_then(|slot| u8::try_from(slot).ok())
        .ok_or_else(|| {
            HxError::protocol(fl!(
                "msgpack-preset-index-out-of-setlist",
                index = index,
                setlist = setlist
            ))
        })?;

    let inner_map = value
        .as_map()
        .ok_or_else(|| HxError::protocol(fl!("msgpack-preset-inner-not-map", index = index)))?;

    let name = inner_map
        .iter()
        .find(|(k, _)| k.as_u64() == Some(MSGPACK_KEY_PRESET_NAME))
        .and_then(|(_, v)| v.as_str())
        .ok_or_else(|| HxError::protocol(fl!("msgpack-preset-name-not-found", index = index)))?
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

        let preset = parse_preset(&outer, 128, 0).unwrap();
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

        let preset = parse_preset(&outer, 128, 0).unwrap();
        assert_eq!(preset.name, "Jazz");
    }

    #[test]
    fn preset_item_not_a_map_returns_error() {
        let item = Value::Integer(42.into());
        let err = parse_preset(&item, 128, 0).unwrap_err();
        assert!(matches!(err, HxError::Protocol(_)));
    }

    #[test]
    fn preset_name_key_missing_returns_error() {
        let inner = Value::Map(vec![(
            Value::Integer(110u64.into()), // Wrong key
            Value::String("Name\0".into()),
        )]);
        let outer = Value::Map(vec![(Value::Integer(0u64.into()), inner)]);
        let err = parse_preset(&outer, 128, 0).unwrap_err();
        assert!(matches!(err, HxError::Protocol(_)));
    }
}
