use openhx_core::Preset;

#[derive(Debug, Clone)]
pub enum Message {
    DeviceDetected(String, Vec<Preset>),
    DeviceDisconnected,
    ConnectionError(String),
    PresetSelected(u8),
}
