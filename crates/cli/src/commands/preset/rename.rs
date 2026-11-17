use crate::args::DeviceArg;
use openhx_core::{connect_client, device::KnownDevice, error::HxError, validate_preset_name};
use openhx_i18n::fl;

/// Executes the `preset rename` CLI command.
pub fn execute(
    device: Option<DeviceArg>,
    setlist: u8,
    preset: u8,
    name: &str,
) -> Result<(), HxError> {
    #[cfg(feature = "mock")]
    eprintln!("{}", fl!("mock-mode-active"));

    // Reject an unusable name before touching the USB bus.
    validate_preset_name(name)?;

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
        fl!(
            "cli-renaming-preset",
            setlist = setlist,
            preset = preset,
            name = name
        )
    );

    client.rename_preset(setlist, preset, name)?;

    println!("{}", fl!("cli-preset-renamed-success"));

    Ok(())
}
