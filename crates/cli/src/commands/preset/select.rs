use crate::{args::DeviceArg, commands::connect};
use openhx_core::error::HxError;
use openhx_i18n::fl;

/// Executes the `preset select` CLI command.
pub fn execute(device: Option<DeviceArg>, setlist: u8, preset: u8) -> Result<(), HxError> {
    let client = connect(device)?;

    println!(
        "{}",
        fl!("cli-selecting-preset", setlist = setlist, preset = preset)
    );

    client.select_preset(setlist, preset)?;

    println!("{}", fl!("cli-preset-selected-success"));

    Ok(())
}
