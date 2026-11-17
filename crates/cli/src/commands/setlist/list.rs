use crate::{args::DeviceArg, commands::connect};
use openhx_core::error::HxError;
use openhx_i18n::fl;

/// Executes the `setlist list` CLI command.
pub fn execute(device: Option<DeviceArg>) -> Result<(), HxError> {
    let client = connect(device)?;

    let setlists = client.list_setlists()?;

    for setlist in &setlists {
        println!("{setlist}");
    }

    println!("\n{}", fl!("cli-total-setlists", count = setlists.len()));

    Ok(())
}
