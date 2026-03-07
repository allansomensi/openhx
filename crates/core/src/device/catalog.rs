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

pub const DEVICE_CATALOG: &[DeviceProfile] = &[PROFILE_HX_STOMP, PROFILE_HX_STOMP_XL];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KnownDevice {
    HxStomp,
    HxStompXl,
}

impl KnownDevice {
    #[inline]
    pub fn profile(self) -> &'static DeviceProfile {
        match self {
            Self::HxStomp => &PROFILE_HX_STOMP,
            Self::HxStompXl => &PROFILE_HX_STOMP_XL,
        }
    }

    #[inline]
    pub fn all() -> impl Iterator<Item = Self> {
        [Self::HxStomp, Self::HxStompXl].into_iter()
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
