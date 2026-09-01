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

impl std::fmt::Display for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:>3}: {}", self.index, self.name)
    }
}
