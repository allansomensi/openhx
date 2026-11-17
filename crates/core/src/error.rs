use thiserror::Error;

#[derive(Debug, Error)]
pub enum HxError {
    #[error(
        "No supported Line 6 device found. \
         Check the USB connection and OS permissions."
    )]
    DeviceNotFound,

    #[error(
        "Found '{device}' but access was denied. On Linux, install the udev \
         rule from pkg/linux/70-openhx.rules (see the README) and replug the device."
    )]
    PermissionDenied { device: &'static str },

    #[error("USB communication error: {0}")]
    Usb(#[from] rusb::Error),

    #[error("MessagePack decoding error: {0}")]
    MsgPack(#[from] rmpv::decode::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error(
        "MessagePack array marker not found in the device stream. \
         The device may be in an unexpected state."
    )]
    InvalidStreamMarker,
}

impl HxError {
    /// Constructs an [`HxError::Protocol`] from any [`Into<String>`] value.
    #[inline]
    pub(crate) fn protocol(msg: impl Into<String>) -> Self {
        Self::Protocol(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // From conversions

    #[test]
    fn rusb_timeout_converts_to_usb_variant() {
        let rusb_err = rusb::Error::Timeout;
        let hx_err: HxError = rusb_err.into();
        assert!(matches!(hx_err, HxError::Usb(rusb::Error::Timeout)));
    }

    #[test]
    fn rusb_no_device_converts_to_usb_variant() {
        let rusb_err = rusb::Error::NoDevice;
        let hx_err: HxError = rusb_err.into();
        assert!(matches!(hx_err, HxError::Usb(rusb::Error::NoDevice)));
    }

    #[test]
    fn permission_denied_display_names_device_and_udev_rule() {
        let s = HxError::PermissionDenied { device: "HX Stomp" }.to_string();
        assert!(s.contains("HX Stomp"));
        assert!(s.contains("udev"));
    }

    // Debug

    #[test]
    fn device_not_found_debug_is_non_empty() {
        let s = format!("{:?}", HxError::DeviceNotFound);
        assert!(!s.is_empty());
    }

    #[test]
    fn protocol_error_debug_includes_message() {
        let err = HxError::Protocol("debug content".to_string());
        let s = format!("{err:?}");
        assert!(s.contains("debug content"));
    }
}
