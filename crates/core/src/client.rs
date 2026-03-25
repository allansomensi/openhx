use crate::{device::profile::DeviceProfile, error::HxError, models::Preset};

#[cfg(feature = "mock")]
use crate::mock::MockClient;

#[cfg(not(feature = "mock"))]
use crate::usb::client::Client;

/// Abstracts over any object that can communicate with a Line 6 HX series device.
pub trait DeviceClient: Send {
    /// Returns a reference to the static metadata profile of the connected device.
    fn profile(&self) -> &'static DeviceProfile;

    /// Reads all presets stored on the device and returns them sorted by index.
    fn read_presets(&self) -> Result<Vec<Preset>, HxError>;

    /// Sends a preset selection command to the device.
    ///
    /// `bank` and `preset` are zero-indexed values whose valid range depends
    /// on the connected device model.
    fn select_preset(&self, bank: u8, preset: u8) -> Result<(), HxError>;
}

/// Creates a [`DeviceClient`] for the given device, or auto-detects one.
pub fn connect_client(
    device: Option<crate::device::KnownDevice>,
) -> Result<Box<dyn DeviceClient + Send>, HxError> {
    #[cfg(feature = "mock")]
    {
        let _ = device;
        return Ok(Box::new(MockClient::new()));
    }

    #[cfg(not(feature = "mock"))]
    {
        match device {
            Some(d) => Client::connect(d).map(|c| Box::new(c) as Box<dyn DeviceClient + Send>),
            None => Client::detect().map(|c| Box::new(c) as Box<dyn DeviceClient + Send>),
        }
    }
}

/// Returns `true` if a [`DeviceClient`] can currently be created.
pub fn is_device_available() -> bool {
    #[cfg(feature = "mock")]
    {
        true
    }

    #[cfg(not(feature = "mock"))]
    {
        Client::detect().is_ok()
    }
}
