use super::profile::DeviceProfile;

pub const PROFILE_HX_STOMP: DeviceProfile = DeviceProfile {
    name: "HX Stomp",
    vendor_id: 0x0E41,
    product_id: 0x4246,
    preset_count: 126,
    setlist_count: 1,
};

pub const PROFILE_HX_STOMP_XL: DeviceProfile = DeviceProfile {
    name: "HX Stomp XL",
    vendor_id: 0x0E41,
    product_id: 0x4253,
    preset_count: 128,
    setlist_count: 1,
};

// Helix Floor streams the active setlist's 128 presets per session; the device
// holds 8 setlists × 128 = 1024 presets total, but only the active one is
// reachable through the current preset-list protocol.
pub const PROFILE_HELIX_FLOOR: DeviceProfile = DeviceProfile {
    name: "Helix Floor",
    vendor_id: 0x0E41,
    product_id: 0x4248,
    preset_count: 128,
    setlist_count: 8,
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

#[cfg(test)]
mod tests {
    use super::*;

    // DEVICE_CATALOG

    #[test]
    fn catalog_has_three_entries() {
        assert_eq!(DEVICE_CATALOG.len(), 3);
    }

    #[test]
    fn catalog_contains_hx_stomp() {
        assert!(
            DEVICE_CATALOG.contains(&PROFILE_HX_STOMP),
            "catalog must contain HX Stomp profile"
        );
    }

    #[test]
    fn catalog_contains_hx_stomp_xl() {
        assert!(DEVICE_CATALOG.contains(&PROFILE_HX_STOMP_XL));
    }

    #[test]
    fn catalog_contains_helix_floor() {
        assert!(DEVICE_CATALOG.contains(&PROFILE_HELIX_FLOOR));
    }

    #[test]
    fn catalog_all_entries_share_vendor_id_0x0e41() {
        for profile in DEVICE_CATALOG {
            assert_eq!(
                profile.vendor_id, 0x0E41,
                "all Line 6 devices use VID 0x0E41; '{}' does not",
                profile.name
            );
        }
    }

    #[test]
    fn catalog_entries_have_correct_preset_counts() {
        assert_eq!(
            PROFILE_HX_STOMP.preset_count, 126,
            "HX Stomp should have 126 presets"
        );
        assert_eq!(
            PROFILE_HX_STOMP_XL.preset_count, 128,
            "HX Stomp XL should have 128 presets"
        );
        assert_eq!(
            PROFILE_HELIX_FLOOR.preset_count, 128,
            "Helix Floor should have 128 presets"
        );
    }

    #[test]
    fn catalog_entries_have_correct_setlist_counts() {
        assert_eq!(PROFILE_HX_STOMP.setlist_count, 1);
        assert_eq!(PROFILE_HX_STOMP_XL.setlist_count, 1);
        assert_eq!(
            PROFILE_HELIX_FLOOR.setlist_count, 8,
            "Helix Floor holds 8 setlists"
        );
    }

    #[test]
    fn catalog_product_ids_are_unique() {
        let pids: Vec<u16> = DEVICE_CATALOG.iter().map(|p| p.product_id).collect();
        let unique: std::collections::HashSet<u16> = pids.iter().copied().collect();
        assert_eq!(
            pids.len(),
            unique.len(),
            "every catalog entry must have a distinct PID"
        );
    }

    #[test]
    fn catalog_names_are_non_empty() {
        for profile in DEVICE_CATALOG {
            assert!(!profile.name.is_empty(), "profile name must not be empty");
        }
    }

    // Known profile constants

    #[test]
    fn hx_stomp_xl_vid_pid() {
        assert_eq!(PROFILE_HX_STOMP_XL.vendor_id, 0x0E41);
        assert_eq!(PROFILE_HX_STOMP_XL.product_id, 0x4253);
    }

    #[test]
    fn helix_floor_vid_pid() {
        assert_eq!(PROFILE_HELIX_FLOOR.vendor_id, 0x0E41);
        assert_eq!(PROFILE_HELIX_FLOOR.product_id, 0x4248);
    }

    #[test]
    fn hx_stomp_xl_name_matches_expected() {
        assert_eq!(PROFILE_HX_STOMP_XL.name, "HX Stomp XL");
    }

    // KnownDevice::profile

    #[test]
    fn hx_stomp_returns_correct_profile() {
        let profile = KnownDevice::HxStomp.profile();
        assert_eq!(profile.name, "HX Stomp");
        assert_eq!(profile.preset_count, 126);
    }

    #[test]
    fn hx_stomp_xl_returns_correct_profile() {
        let profile = KnownDevice::HxStompXl.profile();
        assert_eq!(profile.name, "HX Stomp XL");
        assert_eq!(profile.product_id, 0x4253);
    }

    #[test]
    fn helix_floor_returns_correct_profile() {
        let profile = KnownDevice::HelixFloor.profile();
        assert_eq!(profile.name, "Helix Floor");
    }

    #[test]
    fn profile_returns_static_reference() {
        // Calling profile() twice must return the same address.
        let a = KnownDevice::HxStompXl.profile() as *const _;
        let b = KnownDevice::HxStompXl.profile() as *const _;
        assert_eq!(a, b, "profile() should return the same static reference");
    }

    // Display

    #[test]
    fn display_uses_profile_name() {
        assert_eq!(KnownDevice::HxStomp.to_string(), "HX Stomp");
        assert_eq!(KnownDevice::HxStompXl.to_string(), "HX Stomp XL");
        assert_eq!(KnownDevice::HelixFloor.to_string(), "Helix Floor");
    }

    // From<KnownDevice> for &'static DeviceProfile

    #[test]
    fn from_conversion_matches_direct_profile_call() {
        for device in KnownDevice::all() {
            let via_from: &'static DeviceProfile = device.into();
            let direct = device.profile();
            assert_eq!(via_from, direct);
        }
    }
}
