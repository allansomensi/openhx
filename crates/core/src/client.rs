use crate::{device::profile::DeviceProfile, error::HxError, models::Preset};
use std::sync::{Mutex, OnceLock};

#[cfg(not(feature = "mock"))]
use crate::device::catalog::DEVICE_CATALOG;

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

/// Process-wide cache of the active [`DeviceClient`]. Operations are serialized
/// through the [`Mutex`], and the slot is cleared on USB error so the next
/// [`with_device`] call transparently reconnects.
static SHARED_CLIENT: OnceLock<Mutex<Option<Box<dyn DeviceClient + Send>>>> = OnceLock::new();

fn shared_slot() -> &'static Mutex<Option<Box<dyn DeviceClient + Send>>> {
    SHARED_CLIENT.get_or_init(|| Mutex::new(None))
}

/// Connects on first use and runs `f` against the cached client. Holding the
/// device across calls avoids the claim/release race that triggers
/// `LIBUSB_ERROR_ACCESS` when actions fire faster than the kernel can release
/// the previous interface claim.
///
/// Concurrency: serialised through a process-wide [`Mutex`] held for the
/// entire duration of `f`. Concurrent callers queue; `f` should therefore stay
/// short (no async waits, no blocking I/O beyond the device call itself), and
/// calling `with_device` from inside `f` will deadlock.
///
/// Errors: any `Err` clears the cached client so the next call reconnects;
/// callers don't need to invalidate manually.
pub fn with_device<F, R>(f: F) -> Result<R, HxError>
where
    F: FnOnce(&dyn DeviceClient) -> Result<R, HxError>,
{
    let mut slot = shared_slot().lock().unwrap_or_else(|p| p.into_inner());

    if slot.is_none() {
        *slot = Some(connect_client(None)?);
    }

    let result = f(slot.as_ref().expect("just initialized").as_ref());

    // Drop the cached client on *any* error. Reconnecting only costs one fresh
    // session handshake; the upside is automatic recovery from wedged sessions
    // that return parseable-but-wrong data, which would otherwise persist.
    // Errors caused by stable conditions (e.g. wrong `preset_count`) will keep
    // surfacing after reconnect, which is the right outcome — a recurring
    // visible error rather than silent corruption.
    if result.is_err() {
        *slot = None;
    }

    result
}

/// Drops the shared client, forcing the next [`with_device`] call to
/// reconnect.
pub fn reset_shared_device() {
    if let Some(lock) = SHARED_CLIENT.get() {
        let mut slot = lock.lock().unwrap_or_else(|p| p.into_inner());
        *slot = None;
    }
}

/// Returns `true` if a known device is currently enumerated on the USB bus,
/// without opening or claiming any interface. Suitable for periodic
/// disconnect-monitoring without disturbing an active session.
pub fn is_device_present() -> bool {
    #[cfg(feature = "mock")]
    {
        true
    }

    #[cfg(not(feature = "mock"))]
    {
        use rusb::UsbContext;
        let Ok(ctx) = rusb::Context::new() else {
            return false;
        };
        let Ok(devices) = ctx.devices() else {
            return false;
        };

        for dev in devices.iter() {
            if let Ok(desc) = dev.device_descriptor() {
                let vid = desc.vendor_id();
                let pid = desc.product_id();
                if DEVICE_CATALOG
                    .iter()
                    .any(|p| p.vendor_id == vid && p.product_id == pid)
                {
                    return true;
                }
            }
        }
        false
    }
}
