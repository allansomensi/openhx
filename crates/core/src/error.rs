use thiserror::Error;

#[derive(Debug, Error)]
pub enum HxError {
    #[error(
        "No supported Line 6 device found. \
         Check the USB connection and OS permissions."
    )]
    DeviceNotFound,

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
    /// Constructs an [`HxError::Protocol`] from any `Into<String>` value.
    #[inline]
    pub(crate) fn protocol(msg: impl Into<String>) -> Self {
        Self::Protocol(msg.into())
    }
}
