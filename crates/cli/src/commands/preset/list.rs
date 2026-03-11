use crate::args::DeviceArg;
use openhx_core::{device::KnownDevice, error::HxError, usb::client::Client};
use openhx_i18n::fl;

/// Executes the `preset list` CLI command.
pub fn execute(device: Option<DeviceArg>) -> Result<(), HxError> {
    let client = match device {
        Some(d) => {
            let known = KnownDevice::from(d);

            println!(
                "{}",
                fl!(
                    "cli-connecting-to",
                    device_name = known.profile().name.to_string()
                )
            );
            Client::connect(known)?
        }
        None => {
            println!("{}", fl!("cli-probing-usb"));
            Client::detect()?
        }
    };

    println!(
        "{}\n",
        fl!("cli-connected-to", profile = client.profile().to_string())
    );

    let presets = client.read_presets()?;

    for preset in &presets {
        println!("{preset}");
    }

    println!("\n{}", fl!("cli-total-presets", count = presets.len()));

    Ok(())
}
