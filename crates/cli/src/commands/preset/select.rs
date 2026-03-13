use crate::args::DeviceArg;
use openhx_core::{device::KnownDevice, error::HxError, usb::client::Client};
use openhx_i18n::fl;

/// Executes the `preset select` CLI command.
pub fn execute(device: Option<DeviceArg>, bank: u8, preset: u8) -> Result<(), HxError> {
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

    println!(
        "{}",
        fl!("cli-selecting-preset", bank = bank, preset = preset)
    );

    client.select_preset(bank, preset)?;

    println!("{}", fl!("cli-preset-selected-success"));

    Ok(())
}
