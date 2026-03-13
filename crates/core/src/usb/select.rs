use super::protocol::RESPONSE_PAYLOAD_OFFSET;
use crate::{error::HxError, models::SelectedPreset};
use rmpv::decode::read_value;
use std::io::Cursor;

const MP_KEY_PRESET_INFO: i64 = 0x68; // 104
const MP_KEY_BANK: i64 = 0x6B; // 107
const MP_KEY_PRESET_INDEX: i64 = 0x6C; // 108
const MP_KEY_PRESET_NAME: i64 = 0x6D; // 109

/// Parses a `CHANGE_PRESET_ACK` response buffer.
pub fn parse_select_ack(buf: &[u8]) -> Result<SelectedPreset, HxError> {
    if buf.len() <= RESPONSE_PAYLOAD_OFFSET {
        return Err(HxError::protocol("ACK response too short"));
    }

    let payload = &buf[RESPONSE_PAYLOAD_OFFSET..];
    let mut cursor = Cursor::new(payload);

    let root =
        read_value(&mut cursor).map_err(|_| HxError::protocol("Invalid MessagePack in ACK"))?;

    let root_map = root
        .as_map()
        .ok_or_else(|| HxError::protocol("Root is not a map"))?;

    let mut preset_info = None;
    for (k, v) in root_map {
        if k.as_i64() == Some(MP_KEY_PRESET_INFO) {
            preset_info = Some(v);
            break;
        }
    }

    let info_map = preset_info
        .and_then(|v| v.as_map())
        .ok_or_else(|| HxError::protocol("Missing preset info map (key 104)"))?;

    let mut bank = 0;
    let mut index = 0;
    let mut name = String::new();

    for (k, v) in info_map {
        match k.as_i64() {
            Some(MP_KEY_BANK) => bank = v.as_u64().unwrap_or(0) as u16,
            Some(MP_KEY_PRESET_INDEX) => index = v.as_u64().unwrap_or(0) as u16,
            Some(MP_KEY_PRESET_NAME) => {
                if let Some(s) = v.as_str() {
                    name = s.trim_end_matches('\0').to_string(); // Strip Line6 quirk
                }
            }
            _ => {}
        }
    }

    Ok(SelectedPreset::new(bank, index, name))
}
