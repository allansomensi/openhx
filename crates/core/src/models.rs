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

impl std::fmt::Display for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:>3}: {}", self.index, self.name)
    }
}

/// Confirmed result of a `select_preset` operation.
///
/// The fields are populated from the device ACK packet returned immediately
/// after the host sends a `CHANGE_PRESET` command. All indices are **0-based**.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SelectedPreset {
    pub bank: u16,
    pub index: u16,
    pub name: String,
}

impl SelectedPreset {
    #[inline]
    pub fn new(bank: u16, index: u16, name: impl Into<String>) -> Self {
        Self {
            bank,
            index,
            name: name.into(),
        }
    }
}

impl std::fmt::Display for SelectedPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:>3}: {} (bank {})", self.index, self.name, self.bank)
    }
}
