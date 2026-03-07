pub mod args;
pub mod commands;

use args::{Cli, Commands};
use clap::Parser;
use openhx_core::error::HxError;

pub fn run() -> Result<(), HxError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ListPresets { device } => commands::list_presets::execute(device),
    }
}
