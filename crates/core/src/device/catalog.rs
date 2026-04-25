use super::profile::DeviceProfile;

pub const PROFILE_HX_STOMP: DeviceProfile = DeviceProfile {
    name: "HX Stomp",
    vendor_id: 0x0E41,
    product_id: 0x4252, // unverified
    preset_count: 128,
};

pub const PROFILE_HX_STOMP_XL: DeviceProfile = DeviceProfile {
    name: "HX Stomp XL",
    vendor_id: 0x0E41,
    product_id: 0x4253,
    preset_count: 128,
};

// Helix Floor streams the active setlist's 128 presets per session; the device
// holds 8 setlists × 128 = 1024 presets total, but only the active one is
// reachable through the current preset-list protocol.
pub const PROFILE_HELIX_FLOOR: DeviceProfile = DeviceProfile {
    name: "Helix Floor",
    vendor_id: 0x0E41,
    product_id: 0x4248,
    preset_count: 128,
};

pub const DEVICE_CATALOG: &[DeviceProfile] =
    &[PROFILE_HX_STOMP, PROFILE_HX_STOMP_XL, PROFILE_HELIX_FLOOR];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KnownDevice {
    HxStomp,
    HxStompXl,
    HelixFloor,
}

impl KnownDevice {
    #[inline]
    pub fn profile(self) -> &'static DeviceProfile {
        match self {
            Self::HxStomp => &PROFILE_HX_STOMP,
            Self::HxStompXl => &PROFILE_HX_STOMP_XL,
            Self::HelixFloor => &PROFILE_HELIX_FLOOR,
        }
    }

    #[inline]
    pub fn all() -> impl Iterator<Item = Self> {
        [Self::HxStomp, Self::HxStompXl, Self::HelixFloor].into_iter()
    }
}

impl std::fmt::Display for KnownDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.profile().name)
    }
}

impl From<KnownDevice> for &'static DeviceProfile {
    #[inline]
    fn from(device: KnownDevice) -> Self {
        device.profile()
    }
}
