/// Static metadata that fully describes a supported Line 6 device from the
/// perspective of the USB transport and the preset-read protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceProfile {
    pub name: &'static str,
    pub vendor_id: u16,
    pub product_id: u16,
    pub preset_count: u16,
}

impl DeviceProfile {
    /// Returns the 3-byte MessagePack `array16` marker that corresponds to
    /// [`Self::preset_count`].
    #[inline]
    pub fn array_marker(&self) -> [u8; 3] {
        let [hi, lo] = self.preset_count.to_be_bytes();
        [0xDC, hi, lo]
    }
}

impl std::fmt::Display for DeviceProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (VID={:#06X} PID={:#06X}, {} presets)",
            self.name, self.vendor_id, self.product_id, self.preset_count,
        )
    }
}
