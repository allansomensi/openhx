use crate::{args::DeviceArg, commands::connect};
use openhx_core::{Block, ParamDef, ParamValue, PathSplit, error::HxError};
use openhx_i18n::fl;

/// Executes the `preset blocks` CLI command.
pub fn execute(device: Option<DeviceArg>) -> Result<(), HxError> {
    let client = connect(device)?;

    let chain = client.read_active_preset_blocks()?;

    for block in &chain.blocks {
        println!("{}", format_block(block));
        if !block.params.is_empty() {
            println!("      {}", format_params(block.params_with_defs()));
        }
        if !block.cab_params.is_empty() {
            println!("      {}", format_params(block.cab_params_with_defs()));
        }
    }

    // Path 2 runs in parallel to the positions between split and join.
    if let Some(PathSplit { split, join }) = chain.path2_span()
        && join > split
    {
        let (from, to): (u8, u8) = (split + 1, join);
        println!("\n{}", fl!("cli-path-split", from = from, to = to));
    }

    println!("\n{}", fl!("cli-total-blocks", count = chain.blocks.len()));

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
fn format_params<'a>(
    params: impl Iterator<Item = (&'a ParamValue, Option<&'static ParamDef>)>,
) -> String {
    params
        .map(|(value, def)| match def {
            Some(def) => format!("{} {}", def.name, def.format(value)),
            None => value.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
