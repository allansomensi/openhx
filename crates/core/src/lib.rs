pub mod blocks;
pub mod client;
pub mod device;
pub mod error;
pub mod models;
pub mod usb;

#[cfg(feature = "mock")]
pub mod mock;

pub use blocks::{
    Block, BlockCategory, BlockKind, Model, ParamDef, ParamKind, ParamValue, PathSplit, PresetChain,
};
pub use client::{
    DeviceClient, connect_client, is_device_present, reset_shared_device, with_device,
};
pub use device::{DEVICE_CATALOG, DeviceProfile, KnownDevice};
pub use device::{PROFILE_HELIX_FLOOR, PROFILE_HX_STOMP, PROFILE_HX_STOMP_XL};
pub use error::HxError;
pub use models::{ActivePreset, PRESET_NAME_MAX_LEN, Preset, Setlist, validate_preset_name};
pub use usb::client::Client;
