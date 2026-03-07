use clap::{Parser, Subcommand};
use openhx_core::device::KnownDevice;
use openhx_i18n::fl;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(
        about = fl!("cli-list-presets-about"),
        long_about = fl!("cli-list-presets-long")
    )]
    ListPresets {
        #[arg(long, value_name = "DEVICE", help = fl!("cli-device-help"))]
        device: Option<DeviceArg>,
    },
}

/// CLI-friendly wrapper around [`KnownDevice`] that implements [`clap::ValueEnum`].
#[derive(Clone, clap::ValueEnum)]
pub enum DeviceArg {
    HxStomp,
    HxStompXl,
}

impl From<DeviceArg> for KnownDevice {
    fn from(arg: DeviceArg) -> Self {
        match arg {
            DeviceArg::HxStomp => KnownDevice::HxStomp,
            DeviceArg::HxStompXl => KnownDevice::HxStompXl,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli_structure() {
        Cli::command().debug_assert();
    }

    #[test]
    fn test_device_arg_conversion() {
        let dev: KnownDevice = DeviceArg::HxStomp.into();
        assert_eq!(dev, KnownDevice::HxStomp);

        let dev_xl: KnownDevice = DeviceArg::HxStompXl.into();
        assert_eq!(dev_xl, KnownDevice::HxStompXl);
    }
}
