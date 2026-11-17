use crate::args::DeviceArg;
use openhx_core::{
    Block, ParamDef, ParamValue, PathSplit, PresetChain, connect_client, device::KnownDevice,
    error::HxError,
};
use openhx_i18n::fl;

/// Executes the `preset blocks` CLI command.
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

    let PresetChain { blocks, split } = client.read_active_preset_blocks()?;

    for block in &blocks {
        println!("{}", format_block(block));
        if !block.params.is_empty() {
            println!("      {}", format_params(&block.params, block.param_defs()));
        }
        if !block.cab_params.is_empty() {
            println!(
                "      {}",
                format_params(&block.cab_params, block.cab_param_defs())
            );
        }
    }

    // Path 2 runs in parallel to the positions between split and join.
    if let Some(PathSplit { split, join }) = split
        && blocks.iter().any(|b| b.path == 1)
        && join > split
    {
        let (from, to): (u8, u8) = (split + 1, join);
        println!("\n{}", fl!("cli-path-split", from = from, to = to));
    }

    println!("\n{}", fl!("cli-total-blocks", count = blocks.len()));

    Ok(())
}

/// Formats a block as `P1.3  [AMP] Name (off)`.
fn format_block(block: &Block) -> String {
    let tag = block.category().map_or("???", |c| c.abbreviation());
    let state = if block.enabled {
        String::new()
    } else {
        format!(" ({})", fl!("block-off"))
    };
    format!(
        "P{}.{}  [{tag:<3}] {}{state}",
        block.path + 1,
        block.position + 1,
        block.name()
    )
}

/// Formats values as `Drive 3.7, Level 0.0 dB` when their definitions are
/// known, otherwise as bare values.
fn format_params(params: &[ParamValue], defs: Option<&[ParamDef]>) -> String {
    params
        .iter()
        .enumerate()
        .map(|(i, value)| match defs {
            Some(defs) => format!("{} {}", defs[i].name, defs[i].format(value)),
            None => value.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
