//! The blocks of a preset: what sits in each position of its signal paths.

#[rustfmt::skip]
mod catalog;

use catalog::MODELS;
use openhx_i18n::fl;
use std::collections::HashMap;
use std::sync::LazyLock;

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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Model {
    /// The ID the device reports for the model.
    pub id: u16,
    pub category: BlockCategory,
    pub name: &'static str,
    /// The ID HX Edit uses for the model, such as `HD2_AmpLine6Badonk`.
    pub symbolic_id: &'static str,
    /// The model's parameters, in the order the device stores their values.
    pub params: &'static [ParamDef],
}

/// The models a block of `category` can hold, sorted by name.
///
/// An amp+cab block picks its model from the amps, so [`BlockCategory::AmpCab`]
/// lists those.
pub fn models_in(category: BlockCategory) -> &'static [&'static Model] {
    // Sorted once; the GUI asks on every redraw.
    static BY_CATEGORY: LazyLock<HashMap<BlockCategory, Vec<&'static Model>>> =
        LazyLock::new(|| {
            let mut by_category: HashMap<_, Vec<_>> = HashMap::new();
            for model in MODELS {
                by_category.entry(model.category).or_default().push(model);
            }
            for models in by_category.values_mut() {
                models.sort_unstable_by(|a, b| a.name.cmp(b.name).then(a.id.cmp(&b.id)));
            }
            by_category
        });

    let wanted = match category {
        BlockCategory::AmpCab => BlockCategory::Amp,
        other => other,
    };
    BY_CATEGORY.get(&wanted).map_or(&[], Vec::as_slice)
}

/// Looks up a model by the ID the device reports for it.
pub fn model_info(id: u16) -> Option<&'static Model> {
    MODELS
        .binary_search_by_key(&id, |m| m.id)
        .ok()
        .map(|i| &MODELS[i])
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
            Self::Bool(true) => f.write_str(&fl!("block-on")),
            Self::Bool(false) => f.write_str(&fl!("block-off")),
            Self::Text(s) => f.write_str(s.trim_end_matches('\0')),
        }
    }
}

/// The type and range of a parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamKind {
    Float { min: f64, max: f64, default: f64 },
    Int { min: i64, max: i64, default: i64 },
    Bool { default: bool },
}

/// Describes one parameter of a model, as HX Edit defines it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamDef {
    /// HX Edit's symbolic ID, such as `Drive` or `@mic`.
    pub id: &'static str,
    /// The name HX Edit shows.
    pub name: &'static str,
    /// How HX Edit displays the value (`percent`, `volume`, …); empty when
    /// it has no special display.
    pub display: &'static str,
    pub kind: ParamKind,
}

impl ParamDef {
    /// Whether `value` has this parameter's type.
    pub fn accepts(&self, value: &ParamValue) -> bool {
        matches!(
            (self.kind, value),
            (ParamKind::Float { .. }, ParamValue::Float(_))
                | (ParamKind::Int { .. }, ParamValue::Int(_))
                | (ParamKind::Bool { .. }, ParamValue::Bool(_))
        )
    }

    /// Where `value` lies within the parameter's range, from `0.0` to `1.0`;
    /// `None` for switches and values of the wrong type.
    pub fn fraction(&self, value: &ParamValue) -> Option<f32> {
        let (value, min, max) = match (self.kind, value) {
            (ParamKind::Float { min, max, .. }, ParamValue::Float(v)) => (*v, min, max),
            (ParamKind::Int { min, max, .. }, ParamValue::Int(v)) => {
                (*v as f64, min as f64, max as f64)
            }
            _ => return None,
        };
        (max > min).then(|| ((value - min) / (max - min)).clamp(0.0, 1.0) as f32)
    }

    /// Formats `value` the way HX Edit shows it, with its unit.
    ///
    /// Switches format as on/off in the user's language; selectors whose choices are named on
    /// the device (mic, note value, …) show their raw number.
    pub fn format(&self, value: &ParamValue) -> String {
        let display = self.display;
        match (self.kind, value) {
            (ParamKind::Float { min, max, .. }, ParamValue::Float(v)) => {
                let v = *v;
                // Cut filters at the end of their range are bypassed.
                let cut_off = (display.contains("low_cut") && v <= min)
                    || (display.contains("high_cut") && v >= max);
                if cut_off {
                    "Off".to_owned()
                } else if display.contains("cut") || display.contains("freq") {
                    format_frequency(v)
                } else if display.contains("time_ms") {
                    format!("{:.0} ms", v * 1000.0)
                } else if display.starts_with("percent") {
                    format!("{:.0}%", v * 100.0)
                } else if display.starts_with("volume") {
                    format!("{v:.1} dB")
                } else if display == "pan" {
                    format_pan(v)
                } else if display == "cab_mic_distance" {
                    format!("{v:.1}\"")
                } else if display == "CabMicIrs_Angle" {
                    format!("{v:.0}°")
                } else if (min, max) == (0.0, 1.0) {
                    // Knobs run from 0 to 10 on the device.
                    format!("{:.1}", v * 10.0)
                } else {
                    format!("{v:.2}")
                }
            }
            (ParamKind::Int { .. }, ParamValue::Int(v)) if display == "integer_slider_1based" => {
                (v + 1).to_string()
            }
            _ => value.to_string(),
        }
    }
}

fn format_frequency(hz: f64) -> String {
    if hz >= 1000.0 {
        format!("{:.1} kHz", hz / 1000.0)
    } else {
        format!("{hz:.0} Hz")
    }
}

/// Pan runs from `0.0` (left) over `0.5` (center) to `1.0` (right).
fn format_pan(v: f64) -> String {
    let amount = ((v - 0.5).abs() * 200.0).round();
    if amount == 0.0 {
        "C".to_owned()
    } else if v < 0.5 {
        format!("L{amount:.0}")
    } else {
        format!("R{amount:.0}")
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

    /// Definitions of [`Block::params`], when they are known and match the
    /// values the device reported.
    pub fn param_defs(&self) -> Option<&'static [ParamDef]> {
        matching_defs(self.model_id(), &self.params)
    }

    /// Definitions of [`Block::cab_params`] of an amp+cab block.
    pub fn cab_param_defs(&self) -> Option<&'static [ParamDef]> {
        match self.kind {
            BlockKind::AmpCab { cab, .. } => matching_defs(cab, &self.cab_params),
            BlockKind::Model(_) => None,
        }
    }

    /// Each of [`Block::params`] with its definition, when the definitions
    /// are known and match.
    pub fn params_with_defs(
        &self,
    ) -> impl Iterator<Item = (&ParamValue, Option<&'static ParamDef>)> {
        with_defs(&self.params, self.param_defs())
    }

    /// Each of [`Block::cab_params`] with its definition, like
    /// [`Block::params_with_defs`].
    pub fn cab_params_with_defs(
        &self,
    ) -> impl Iterator<Item = (&ParamValue, Option<&'static ParamDef>)> {
        with_defs(&self.cab_params, self.cab_param_defs())
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

/// Pairs `values` with `defs`, or with `None` when there are none.
fn with_defs<'a>(
    values: &'a [ParamValue],
    defs: Option<&'static [ParamDef]>,
) -> impl Iterator<Item = (&'a ParamValue, Option<&'static ParamDef>)> {
    let defs = defs.unwrap_or_default().iter().map(Some);
    values.iter().zip(defs.chain(std::iter::repeat(None)))
}

/// A model's definitions, if there is one of the right type for each of
/// `values`.
///
/// A mismatch means a firmware with other parameters or a parameter order
/// that differs from the definition; then no names are better than wrong
/// ones.
fn matching_defs(model: u16, values: &[ParamValue]) -> Option<&'static [ParamDef]> {
    model_info(model).map(|m| m.params).filter(|defs| {
        defs.len() == values.len()
            && defs
                .iter()
                .zip(values)
                .all(|(def, value)| def.accepts(value))
    })
}

/// Where path 2 branches off path 1 and where it merges back.
///
/// Both are gaps between positions: gap `n` lies right before position `n`
/// of path 1, and gap [`POSITIONS_PER_PATH`] lies before its output. Path 2
/// carries the signal for the positions from `split` up to, not including,
/// `join`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathSplit {
    pub split: u8,
    pub join: u8,
}

/// A preset's signal chain: its blocks and how its paths connect.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PresetChain {
    /// The blocks in signal order, path by path.
    pub blocks: Vec<Block>,
    /// Where path 2 branches off and merges back; `None` when the preset
    /// does not say.
    pub split: Option<PathSplit>,
}

impl PresetChain {
    /// Where path 2 runs, or `None` when it holds no blocks.
    ///
    /// A preset that does not say spans path 2 over the whole chain.
    pub fn path2_span(&self) -> Option<PathSplit> {
        self.blocks.iter().any(|b| b.path == 1).then(|| {
            self.split.unwrap_or(PathSplit {
                split: 0,
                join: POSITIONS_PER_PATH,
            })
        })
    }

    /// The block at `position` of `path`, if there is one.
    pub fn block_at(&self, path: u8, position: u8) -> Option<&Block> {
        self.blocks
            .iter()
            .find(|b| b.path == path && b.position == position)
    }
}

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
        assert!(MODELS.windows(2).all(|w| w[0].id < w[1].id));
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
    fn definitions_name_parameters_in_device_order() {
        let params = |id| model_info(id).unwrap().params;
        let horizon: Vec<_> = params(582).iter().map(|p| p.name).collect();
        assert_eq!(
            horizon,
            ["Drive", "Attack", "Bright", "Gate", "GateRange", "Level"]
        );
        assert_eq!(model_info(582).unwrap().symbolic_id, "HD2_DistHorizonDrive");

        // A cab's mic selector is stored after its regular parameters.
        let cab = params(72);
        assert_eq!(cab.first().unwrap().id, "Distance");
        assert_eq!(cab.last().unwrap().id, "@mic");
    }

    #[test]
    fn tempo_sync_order_follows_the_device() {
        // Captured from an HX Stomp: Simple Delay (80) stores the note value
        // before the tempo sync switch, Pitch Echo (334) the other way round.
        let ids = |id| -> Vec<_> {
            model_info(id)
                .unwrap()
                .params
                .iter()
                .map(|p| p.id)
                .collect()
        };
        let simple = ids(80);
        let pitch = ids(334);
        let pos = |ids: &[&str], id| ids.iter().position(|&p| p == id).unwrap();
        assert!(pos(&simple, "SyncSelect1") < pos(&simple, "TempoSync1"));
        assert!(pos(&pitch, "TempoSync1") < pos(&pitch, "SyncSelect1"));
    }

    #[test]
    fn stereo_variants_carry_their_stereo_only_parameters() {
        // Ducked Delay: mono 77, stereo 85; "Scale" exists in stereo only.
        let has_scale = |id| {
            model_info(id)
                .unwrap()
                .params
                .iter()
                .any(|p| p.id == "Scale")
        };
        assert!(!has_scale(77));
        assert!(has_scale(85));
    }

    #[test]
    fn param_defs_require_a_matching_value_count() {
        let mut b = block(BlockKind::Model(582));
        b.params = vec![ParamValue::Float(0.0); 6];
        b.params[1] = ParamValue::Int(1);
        b.params[4] = ParamValue::Bool(false);
        assert_eq!(b.param_defs().map(<[_]>::len), Some(6));

        b.params.pop();
        assert!(b.param_defs().is_none());
    }

    #[test]
    fn param_defs_require_matching_value_types() {
        let mut b = block(BlockKind::Model(582));
        b.params = vec![ParamValue::Float(0.0); 6];
        // Horizon Drive's second parameter is the integer "Attack".
        b.params[1] = ParamValue::Int(1);
        b.params[4] = ParamValue::Bool(false);
        assert!(b.param_defs().is_some());

        b.params[1] = ParamValue::Bool(true);
        assert!(b.param_defs().is_none());
    }

    #[test]
    fn params_pair_with_their_definitions_when_they_match() {
        let mut b = block(BlockKind::Model(582));
        b.params = vec![ParamValue::Float(0.0); 6];
        b.params[1] = ParamValue::Int(1);
        b.params[4] = ParamValue::Bool(false);
        let names: Vec<_> = b.params_with_defs().map(|(_, d)| d.map(|d| d.id)).collect();
        assert_eq!(names[0], Some("Drive"));
        assert_eq!(names.len(), 6);

        b.params.pop();
        assert!(b.params_with_defs().all(|(_, d)| d.is_none()));
        assert_eq!(b.params_with_defs().count(), 5);
    }

    #[test]
    fn path2_spans_the_whole_chain_unless_the_preset_says_otherwise() {
        let mut lower = block(BlockKind::Model(72));
        lower.path = 1;
        let mut chain = PresetChain {
            blocks: vec![block(BlockKind::Model(582))],
            split: Some(PathSplit { split: 2, join: 4 }),
        };
        assert_eq!(chain.path2_span(), None);

        chain.blocks.push(lower);
        assert_eq!(chain.path2_span(), Some(PathSplit { split: 2, join: 4 }));
        chain.split = None;
        assert_eq!(chain.path2_span(), Some(PathSplit { split: 0, join: 8 }));
        assert!(chain.block_at(1, 0).is_some());
    }

    #[test]
    fn param_defs_format_values_with_units() {
        let def = |display, min, max| ParamDef {
            id: "x",
            name: "x",
            display,
            kind: ParamKind::Float {
                min,
                max,
                default: min,
            },
        };
        let f = ParamValue::Float;
        assert_eq!(def("generic_knob", 0.0, 1.0).format(&f(0.37)), "3.7");
        assert_eq!(def("percent", 0.0, 1.0).format(&f(0.5)), "50%");
        assert_eq!(def("volume", -60.0, 6.0).format(&f(-3.0)), "-3.0 dB");
        assert_eq!(def("time_ms", 0.0, 2.0).format(&f(0.47)), "470 ms");
        assert_eq!(def("cab_low_cut", 19.9, 500.0).format(&f(19.9)), "Off");
        assert_eq!(def("cab_low_cut", 19.9, 500.0).format(&f(80.0)), "80 Hz");
        assert_eq!(
            def("cab_high_cut", 500.0, 20100.0).format(&f(8000.0)),
            "8.0 kHz"
        );
        assert_eq!(def("pan", 0.0, 1.0).format(&f(0.5)), "C");
        assert_eq!(def("pan", 0.0, 1.0).format(&f(0.25)), "L50");

        let volume = def("volume", -60.0, 6.0);
        assert_eq!(volume.fraction(&f(6.0)), Some(1.0));
        assert_eq!(volume.fraction(&ParamValue::Bool(true)), None);
    }

    #[test]
    fn param_values_display_compactly() {
        assert_eq!(ParamValue::Float(0.654).to_string(), "0.65");
        assert_eq!(ParamValue::Int(8000).to_string(), "8000");
        assert_eq!(ParamValue::Bool(true).to_string(), "on");
        assert_eq!(ParamValue::Text("ab\0".into()).to_string(), "ab");
    }
}
