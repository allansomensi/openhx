pub mod args;
pub mod commands;

use args::{Cli, Commands, PresetAction};
use clap::Parser;
use openhx_core::error::HxError;

pub fn run() -> Result<(), HxError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Preset { action } => match action {
            PresetAction::List { device } => commands::preset::list::execute(device),
            PresetAction::Select { index, device } => {
                commands::preset::select::execute(index, device)
            }
        },
    }
}
