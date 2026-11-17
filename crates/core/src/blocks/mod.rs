//! The blocks of a preset: what sits in each position of its signal paths.

mod catalog;

use catalog::MODELS;

/// The kind of processing a block's model performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockCategory {
    Distortion,
    Dynamic,
    Eq,
    Modulation,
    Delay,
    Reverb,
    PitchSynth,
    Filter,
    Wah,
    Amp,
    AmpCab,
    Preamp,
    Cab,
    ImpulseResponse,
    VolumePan,
    SendReturn,
}

impl BlockCategory {
    /// Every category, sorted by label.
    pub const ALL: [Self; 16] = [
        Self::Amp,
        Self::AmpCab,
        Self::Cab,
        Self::Delay,
        Self::Distortion,
        Self::Dynamic,
        Self::Eq,
        Self::Filter,
        Self::ImpulseResponse,
        Self::Modulation,
        Self::PitchSynth,
        Self::Preamp,
        Self::Reverb,
        Self::SendReturn,
        Self::VolumePan,
        Self::Wah,
    ];

    /// The category's name as the device and HX Edit show it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Distortion => "Distortion",
            Self::Dynamic => "Dynamic",
            Self::Eq => "EQ",
            Self::Modulation => "Modulation",
            Self::Delay => "Delay",
            Self::Reverb => "Reverb",
            Self::PitchSynth => "Pitch/Synth",
            Self::Filter => "Filter",
            Self::Wah => "Wah",
            Self::Amp => "Amp",
            Self::AmpCab => "Amp+Cab",
            Self::Preamp => "Preamp",
            Self::Cab => "Cab",
            Self::ImpulseResponse => "Impulse Response",
            Self::VolumePan => "Volume/Pan",
            Self::SendReturn => "Send/Return",
        }
    }

    /// A short tag that fits under a block in the signal chain.
    pub fn abbreviation(self) -> &'static str {
        match self {
            Self::Distortion => "DIS",
            Self::Dynamic => "DYN",
            Self::Eq => "EQ",
            Self::Modulation => "MOD",
            Self::Delay => "DLY",
            Self::Reverb => "REV",
            Self::PitchSynth => "P/S",
            Self::Filter => "FLT",
            Self::Wah => "WAH",
            Self::Amp => "AMP",
            Self::AmpCab => "A+C",
            Self::Preamp => "PRE",
            Self::Cab => "CAB",
            Self::ImpulseResponse => "IR",
            Self::VolumePan => "VOL",
            Self::SendReturn => "S/R",
        }
    }
}

impl std::fmt::Display for BlockCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// A known block model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: u16,
    pub category: BlockCategory,
    pub name: &'static str,
}

/// The models a block of `category` can hold, sorted by name.
///
/// An amp+cab block picks its model from the amps, so [`BlockCategory::AmpCab`]
/// lists those.
pub fn models_in(category: BlockCategory) -> Vec<ModelInfo> {
    let wanted = match category {
        BlockCategory::AmpCab => BlockCategory::Amp,
        other => other,
    };
    let mut models: Vec<ModelInfo> = MODELS
        .iter()
        .filter(|&&(_, c, _)| c == wanted)
        .map(|&(id, category, name)| ModelInfo { id, category, name })
        .collect();
    models.sort_unstable_by(|a, b| a.name.cmp(b.name).then(a.id.cmp(&b.id)));
    models
}

/// Looks up a model by the ID the device reports for it.
pub fn model_info(id: u16) -> Option<ModelInfo> {
    MODELS
        .binary_search_by_key(&id, |&(model_id, _, _)| model_id)
        .ok()
        .map(|i| {
            let (id, category, name) = MODELS[i];
            ModelInfo { id, category, name }
        })
}

/// One parameter value as the device stores it.
///
/// Most parameters are floats; switches are booleans and selectors (mic
/// type, note values, …) are integers. Values are raw: a knob shown as `65%`
/// on the device is stored as `0.65`.
#[derive(Debug, Clone, PartialEq)]
pub enum ParamValue {
    Float(f64),
    Int(i64),
    Bool(bool),
    Text(String),
}

impl std::fmt::Display for ParamValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Float(v) => write!(f, "{v:.2}"),
            Self::Int(v) => write!(f, "{v}"),
            Self::Bool(true) => f.write_str("on"),
            Self::Bool(false) => f.write_str("off"),
            Self::Text(s) => f.write_str(s.trim_end_matches('\0')),
        }
    }
}

/// What a block position holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    /// A single model.
    Model(u16),
    /// An amp with its cab, sharing one block.
    AmpCab { amp: u16, cab: u16 },
}

/// A processing block in one of the preset's signal paths.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// Signal path, `0` or `1`.
    pub path: u8,
    /// Position within the path, `0`–`7`.
    pub position: u8,
    pub kind: BlockKind,
    /// Whether the block is switched on.
    pub enabled: bool,
    /// Parameter values of the block's model, in device order.
    pub params: Vec<ParamValue>,
    /// Parameter values of the cab of a [`BlockKind::AmpCab`] block.
    pub cab_params: Vec<ParamValue>,
}

impl Block {
    /// The block's category, if its model is known.
    pub fn category(&self) -> Option<BlockCategory> {
        match self.kind {
            BlockKind::Model(id) => model_info(id).map(|m| m.category),
            BlockKind::AmpCab { .. } => Some(BlockCategory::AmpCab),
        }
    }

    /// The ID of the model that defines the block: the model itself, or the
    /// amp of an amp+cab block.
    pub fn model_id(&self) -> u16 {
        match self.kind {
            BlockKind::Model(id) | BlockKind::AmpCab { amp: id, .. } => id,
        }
    }

    /// The block's display name; unknown models show their ID.
    pub fn name(&self) -> String {
        let model_name =
            |id: u16| model_info(id).map_or_else(|| format!("Model {id}"), |m| m.name.to_owned());
        match self.kind {
            BlockKind::Model(id) => model_name(id),
            BlockKind::AmpCab { amp, cab } => format!("{} + {}", model_name(amp), model_name(cab)),
        }
    }
}

/// Number of signal paths in a preset.
pub const PATH_COUNT: u8 = 2;

/// Number of block positions in each signal path.
pub const POSITIONS_PER_PATH: u8 = 8;

#[cfg(test)]
mod tests {
    use super::*;

    fn block(kind: BlockKind) -> Block {
        Block {
            path: 0,
            position: 0,
            kind,
            enabled: true,
            params: vec![],
            cab_params: vec![],
        }
    }

    #[test]
    fn all_categories_are_listed_once_and_sorted() {
        let labels: Vec<_> = BlockCategory::ALL.iter().map(|c| c.label()).collect();
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(labels, sorted);
    }

    #[test]
    fn models_in_lists_a_category_sorted_by_name() {
        let cabs = models_in(BlockCategory::Cab);
        assert!(cabs.iter().all(|m| m.category == BlockCategory::Cab));
        assert!(cabs.windows(2).all(|w| w[0].name <= w[1].name));
        assert!(cabs.iter().any(|m| m.id == 72));
    }

    #[test]
    fn amp_cab_lists_the_amps() {
        let amps = models_in(BlockCategory::AmpCab);
        assert!(!amps.is_empty());
        assert!(amps.iter().all(|m| m.category == BlockCategory::Amp));
        assert_eq!(
            block(BlockKind::AmpCab { amp: 349, cab: 72 }).model_id(),
            349
        );
    }

    #[test]
    fn model_table_is_sorted_and_unique() {
        assert!(MODELS.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn model_info_finds_known_models() {
        let horizon = model_info(582).unwrap();
        assert_eq!(horizon.category, BlockCategory::Distortion);
        assert_eq!(horizon.name, "Horizon Drive (mono)");

        assert_eq!(model_info(72).unwrap().category, BlockCategory::Cab);
        assert!(model_info(u16::MAX).is_none());
    }

    #[test]
    fn amp_cab_block_names_both_models() {
        let b = block(BlockKind::AmpCab { amp: 349, cab: 72 });
        assert_eq!(b.category(), Some(BlockCategory::AmpCab));
        assert_eq!(b.name(), "Line 6 Badonk (mono) + 4x12 XXL V30 (mono)");
    }

    #[test]
    fn unknown_model_shows_its_id() {
        let b = block(BlockKind::Model(9999));
        assert_eq!(b.category(), None);
        assert_eq!(b.name(), "Model 9999");
    }

    #[test]
    fn param_values_display_compactly() {
        assert_eq!(ParamValue::Float(0.654).to_string(), "0.65");
        assert_eq!(ParamValue::Int(8000).to_string(), "8000");
        assert_eq!(ParamValue::Bool(true).to_string(), "on");
        assert_eq!(ParamValue::Text("ab\0".into()).to_string(), "ab");
    }
}
