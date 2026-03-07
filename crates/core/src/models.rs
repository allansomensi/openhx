#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Preset {
    pub index: u16,
    pub name: String,
}

impl Preset {
    #[inline]
    pub fn new(index: u16, name: impl Into<String>) -> Self {
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
