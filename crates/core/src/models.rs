use crate::error::HxError;
use openhx_i18n::fl;

/// Longest preset name the device stores, in bytes.
pub const PRESET_NAME_MAX_LEN: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Preset {
    pub index: u8,
    pub name: String,
}

impl Preset {
    #[inline]
    pub fn new(index: u8, name: impl Into<String>) -> Self {
        Self {
            index,
            name: name.into(),
        }
    }
}

/// The preset the device is currently playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActivePreset {
    pub setlist: u8,
    /// Slot within `setlist`.
    pub preset: u8,
}

/// A named setlist: a bank of [`Preset`]s held by the device.
///
/// HX Stomp-family devices have a single setlist; Helix-family devices hold
/// eight, named on the device itself (`FACTORY 1`, `USER 1`, …).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Setlist {
    pub index: u8,
    pub name: String,
}

impl Setlist {
    #[inline]
    pub fn new(index: u8, name: impl Into<String>) -> Self {
        Self {
            index,
            name: name.into(),
        }
    }
}

impl std::fmt::Display for Setlist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.index, self.name)
    }
}

/// Checks that `name` can be stored as a preset name on the device.
///
/// The device shows names on a character display that only renders printable
/// ASCII, and stores at most [`PRESET_NAME_MAX_LEN`] of them.
pub fn validate_preset_name(name: &str) -> Result<(), HxError> {
    if name.trim().is_empty() {
        return Err(HxError::protocol(fl!("preset-name-empty")));
    }

    if let Some(c) = name.chars().find(|c| !matches!(c, ' '..='~')) {
        return Err(HxError::protocol(fl!(
            "preset-name-invalid-char",
            character = c.to_string()
        )));
    }

    if name.len() > PRESET_NAME_MAX_LEN {
        return Err(HxError::protocol(fl!(
            "preset-name-too-long",
            len = name.len(),
            max = PRESET_NAME_MAX_LEN
        )));
    }

    Ok(())
}

impl std::fmt::Display for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:>3}: {}", self.index, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_name_accepts_printable_ascii_up_to_max_len() {
        assert!(validate_preset_name("Clean").is_ok());
        assert!(validate_preset_name("Lead #2 (Live)!").is_ok());
        assert!(validate_preset_name(&"X".repeat(PRESET_NAME_MAX_LEN)).is_ok());
    }

    #[test]
    fn preset_name_rejects_empty_and_blank() {
        assert!(validate_preset_name("").is_err());
        assert!(validate_preset_name("   ").is_err());
    }

    #[test]
    fn preset_name_rejects_names_over_max_len() {
        let name = "X".repeat(PRESET_NAME_MAX_LEN + 1);
        assert!(matches!(
            validate_preset_name(&name),
            Err(HxError::Protocol(_))
        ));
    }

    #[test]
    fn preset_name_rejects_non_ascii_and_control_chars() {
        assert!(validate_preset_name("Grüße").is_err());
        assert!(validate_preset_name("Tab\there").is_err());
        assert!(validate_preset_name("Nul\0").is_err());
    }
}
