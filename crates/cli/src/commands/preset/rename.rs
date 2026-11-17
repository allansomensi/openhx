use crate::{args::DeviceArg, commands::connect};
use openhx_core::{error::HxError, validate_preset_name};
use openhx_i18n::fl;

/// Executes the `preset rename` CLI command.
pub fn execute(
    device: Option<DeviceArg>,
    setlist: u8,
    preset: u8,
    name: &str,
) -> Result<(), HxError> {
    // Reject an unusable name before touching the USB bus.
    validate_preset_name(name)?;

    let client = connect(device)?;

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
