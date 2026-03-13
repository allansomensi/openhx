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
pub fn parse_msgpack_stream(data: &[u8], preset_count: u16) -> Result<Vec<Preset>, HxError> {
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

    items.iter().map(parse_preset).collect()
}

/// Parses one preset slot.
///
/// Expected layout:
/// ```text
/// fixmap(1) { <index: uint> → fixmap { ..., 109: <name: str>, ... } }
/// ```
fn parse_preset(item: &rmpv::Value) -> Result<Preset, HxError> {
    let map = item
        .as_map()
        .ok_or_else(|| HxError::protocol(fl!("msgpack-preset-not-map")))?;

    let (key, value) = map
        .first()
        .ok_or_else(|| HxError::protocol(fl!("msgpack-preset-map-empty")))?;

    let index =
        key.as_u64()
            .ok_or_else(|| HxError::protocol(fl!("msgpack-preset-index-not-int")))? as u8;

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

    Ok(Preset::new(index, name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmpv::Value;

    #[test]
    fn test_parse_valid_preset() {
        let inner_map = Value::Map(vec![(
            Value::Integer(MSGPACK_KEY_PRESET_NAME.into()),
            Value::String("Clean Preset\0".into()),
        )]);

        let outer_map = Value::Map(vec![(Value::Integer(42.into()), inner_map)]);

        let result = parse_preset(&outer_map);
        assert!(result.is_ok());

        let preset = result.unwrap();

        assert_eq!(preset.index, 42);
        assert_eq!(preset.name, "Clean Preset");
    }

    #[test]
    fn test_parse_preset_missing_name() {
        let inner_map = Value::Map(vec![(
            Value::Integer(110.into()), // Wrong key
            Value::String("Clean Preset".into()),
        )]);

        let outer_map = Value::Map(vec![(Value::Integer(42.into()), inner_map)]);

        let result = parse_preset(&outer_map);
        assert!(result.is_err());
    }
}
