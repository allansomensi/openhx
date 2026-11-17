use crate::args::DeviceArg;
use openhx_core::{connect_client, device::KnownDevice, error::HxError};
use openhx_i18n::fl;

/// Executes the `preset active` CLI command.
pub fn execute(device: Option<DeviceArg>) -> Result<(), HxError> {
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

    let active = client.read_active_preset()?;
    let name = client
        .read_setlist_presets(active.setlist)?
        .into_iter()
        .find(|p| p.index == active.preset)
        .map(|p| p.name)
        .unwrap_or_default();

    println!(
        "{}",
        fl!(
            "cli-active-preset",
            setlist = active.setlist,
            preset = format!("{:03}", active.preset),
            name = name
        )
    );

    Ok(())
}
