use crate::{args::DeviceArg, commands::connect};
use openhx_core::error::HxError;
use openhx_i18n::fl;

/// Executes the `preset list` CLI command.
pub fn execute(device: Option<DeviceArg>, setlist: u8) -> Result<(), HxError> {
    let client = connect(device)?;

    if client.profile().setlist_count > 1 {
        println!("{}", fl!("cli-reading-setlist", setlist = setlist));
    }

    let presets = client.read_setlist_presets(setlist)?;

    for preset in &presets {
        println!("{preset}");
    }

    println!("\n{}", fl!("cli-total-presets", count = presets.len()));

    Ok(())
}
