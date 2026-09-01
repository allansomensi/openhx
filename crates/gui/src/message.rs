use openhx_core::Preset;

#[derive(Debug, Clone)]
pub enum Message {
    DeviceDetected {
        name: String,
        setlist_count: u8,
        presets: Vec<Preset>,
    },
    DeviceDisconnected,
    ConnectionError(String),
    /// The user picked another setlist in the sidebar.
    SetlistChosen(u8),
    /// A setlist's presets finished loading.
    SetlistLoaded(u8, Vec<Preset>),
    /// Loading a setlist failed; the previous list stays on screen.
    SetlistLoadFailed(u8, String),
    PresetSelected(u8),
}
