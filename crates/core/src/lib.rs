pub mod client;
pub mod device;
pub mod error;
pub mod models;
pub mod usb;

#[cfg(feature = "mock")]
pub mod mock;

pub use client::{
    DeviceClient, connect_client, is_device_present, reset_shared_device, with_device,
};
pub use device::{DEVICE_CATALOG, DeviceProfile, KnownDevice};
pub use device::{PROFILE_HX_STOMP, PROFILE_HX_STOMP_XL};
pub use error::HxError;
pub use models::Preset;
pub use usb::client::Client;
