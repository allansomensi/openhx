use crate::args::DeviceArg;
use openhx_core::{device::KnownDevice, error::HxError, usb::client::Client};
use openhx_i18n::fl;

pub fn execute(preset_index: u8, device: Option<DeviceArg>) -> Result<(), HxError> {
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
        fl!("cli-selecting-preset", index = preset_index.to_string())
    );

    let selected = client.select_preset(preset_index)?;

    println!(
        "{}",
        fl!(
            "cli-preset-selected",
            index = selected.index.to_string(),
            name = selected.name.clone()
        )
    );

    Ok(())
}
