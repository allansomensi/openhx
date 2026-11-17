use crate::args::DeviceArg;
use openhx_core::{DeviceClient, connect_client, device::KnownDevice, error::HxError};
use openhx_i18n::fl;

pub mod preset;
pub mod setlist;

/// Connects to `device`, or to the first supported device on the USB bus,
/// and reports the progress.
pub fn connect(device: Option<DeviceArg>) -> Result<Box<dyn DeviceClient + Send>, HxError> {
    #[cfg(feature = "mock")]
    eprintln!("{}", fl!("mock-mode-active"));

    let known_device = device.map(KnownDevice::from);

    match &known_device {
        Some(d) => println!(
            "{}",
            fl!(
                "cli-connecting-to",
                device_name = d.profile().name.to_string()
            )
        ),
        None => println!("{}", fl!("cli-probing-usb")),
    }

    let client = connect_client(known_device)?;

    println!(
        "{}\n",
        fl!("cli-connected-to", profile = client.profile().to_string())
    );

    Ok(client)
}
