use crate::args::DeviceArg;
use openhx_core::{connect_client, device::KnownDevice, error::HxError};
use openhx_i18n::fl;

/// Executes the `preset select` CLI command.
pub fn execute(device: Option<DeviceArg>, bank: u8, preset: u8) -> Result<(), HxError> {
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

    println!(
        "{}",
        fl!("cli-selecting-preset", bank = bank, preset = preset)
    );

    client.select_preset(bank, preset)?;

    println!("{}", fl!("cli-preset-selected-success"));

    Ok(())
}
