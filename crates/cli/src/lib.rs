pub mod args;
pub mod commands;

use args::{Cli, Commands, PresetAction, SetlistAction};
use clap::Parser;
use openhx_core::error::HxError;

pub fn run() -> Result<(), HxError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Setlist { action } => match action {
            SetlistAction::List { device } => commands::setlist::list::execute(device),
        },
        Commands::Preset { action } => match action {
            PresetAction::List { device, setlist } => {
                commands::preset::list::execute(device, setlist)
            }
            PresetAction::Select {
                device,
                setlist,
                preset,
            } => commands::preset::select::execute(device, setlist, preset),
        },
    }
}
