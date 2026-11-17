use openhx_core::{Preset, Setlist};

#[derive(Debug, Clone)]
pub enum Message {
    DeviceDetected {
        name: String,
        setlists: Vec<Setlist>,
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
    /// The user edited the name field of the selected preset.
    RenameInputChanged(String),
    /// The user asked to store the edited name on the device.
    RenameSubmitted,
    /// The device stored a new preset name.
    PresetRenamed {
        setlist: u8,
        preset: u8,
        name: String,
    },
    /// Storing a new preset name failed.
    RenameFailed(String),
}
