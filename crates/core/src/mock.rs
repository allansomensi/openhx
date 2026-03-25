use crate::{
    client::DeviceClient,
    device::{PROFILE_HX_STOMP_XL, profile::DeviceProfile},
    error::HxError,
    models::Preset,
};

/// A simulated HX Stomp XL.
pub struct MockClient;

impl MockClient {
    /// Creates a new `MockClient` targeting the HX Stomp XL profile.
    #[inline]
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockClient {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceClient for MockClient {
    fn profile(&self) -> &'static DeviceProfile {
        &PROFILE_HX_STOMP_XL
    }

    /// Returns a static list of 128 realistic preset names without any I/O.
    fn read_presets(&self) -> Result<Vec<Preset>, HxError> {
        let presets = MOCK_PRESET_NAMES
            .iter()
            .enumerate()
            .map(|(i, &name)| Preset::new(i as u8, name))
            .collect();

        Ok(presets)
    }

    /// Simulates a preset selection command.
    ///
    /// Validates that `bank` and `preset` fall within the range supported by
    /// the HX Stomp XL (4 banks × 32 presets) and returns `Ok(())`. No I/O
    /// is performed.
    fn select_preset(&self, bank: u8, preset: u8) -> Result<(), HxError> {
        const MAX_BANKS: u8 = 4;
        const MAX_PRESETS_PER_BANK: u8 = 32;

        if bank >= MAX_BANKS {
            return Err(HxError::protocol(format!(
                "bank {bank} out of range (0–{})",
                MAX_BANKS - 1
            )));
        }

        if preset >= MAX_PRESETS_PER_BANK {
            return Err(HxError::protocol(format!(
                "preset {preset} out of range (0–{})",
                MAX_PRESETS_PER_BANK - 1
            )));
        }

        eprintln!("[mock] select_preset(bank={bank}, preset={preset}) — OK");

        Ok(())
    }
}

static MOCK_PRESET_NAMES: [&str; 128] = [
    "Clean Tweed",
    "Warm Jazz",
    "Vintage AC",
    "Blackface Clean",
    "Boutique Clean",
    "Crystal Clear",
    "Funk Machine",
    "Sparkle Top",
    "Tweed Crunch",
    "Plexi Crunch",
    "Edge Breakup",
    "Blues Drive",
    "Smooth OD",
    "Dirty Thirty",
    "Texas Blues",
    "Chicago Blues",
    "Hi-Gain Mesa",
    "Double Recto",
    "Modern Metal",
    "Heavy Rhythm",
    "Djent Machine",
    "Thrash Lead",
    "Doom Riff",
    "Death Chunk",
    "Sustain Lead",
    "Screaming Lead",
    "Singing Lead",
    "Violin Lead",
    "Pink Lead",
    "Arena Solo",
    "AOR Lead",
    "Country Lead",
    "Chorus Pad",
    "Vintage Chorus",
    "Lush Chorus",
    "CE-1 Vibe",
    "Rotary Sim",
    "Tremolo Pad",
    "Vibrato Warm",
    "Ring Mod",
    "Long Echo",
    "Ping Pong",
    "Slapback",
    "Tape Echo",
    "Vintage Echo",
    "Reverse Dly",
    "Lofi Delay",
    "Stereo Echo",
    "Hall Reverb",
    "Room Reverb",
    "Plate Reverb",
    "Spring Reverb",
    "Church Verb",
    "Cave Reverb",
    "Arena Verb",
    "Shimmer Pad",
    "Wah Drive",
    "Auto Wah",
    "Envelope Fltr",
    "Tron Vibe",
    "Q Filter",
    "LFO Filter",
    "Step Filter",
    "Cocked Wah",
    "Fuzz Face",
    "Big Muff",
    "OCD Drive",
    "Klon Drive",
    "Tubescreamer",
    "Dist Plus",
    "RAT Drive",
    "Metal Zone",
    "Comp Sustain",
    "Orange Squish",
    "LA-2A Style",
    "1176 Style",
    "Optical Comp",
    "Dyna Comp",
    "Smooth Comp",
    "Studio Limit",
    "Octave Down",
    "Whammy Up",
    "Pitch Harm",
    "Smart Harm",
    "Detune Pad",
    "Drop Tune",
    "Bass Octave",
    "Synth Glide",
    "Synth Bass",
    "Poly Synth",
    "Organ Tones",
    "Keyboard Sim",
    "Acoustic Sim",
    "12-String",
    "Nashville Dbl",
    "Baritone Dn",
    "Studio DI",
    "Broadcast Dry",
    "Lofi Radio",
    "Telephone FX",
    "Megaphone",
    "Cassette Warm",
    "Bass Direct",
    "Amp Sim",
    "Pedalboard 1",
    "Pedalboard 2",
    "Pedalboard 3",
    "Pedalboard 4",
    "Live Rhythm",
    "Live Lead",
    "Live Clean",
    "Live Solo",
    "Studio A",
    "Studio B",
    "Tracking 1",
    "Tracking 2",
    "Mix Ref",
    "Reamp 1",
    "Reamp 2",
    "Scratch Pad",
    "Empty 120",
    "Empty 121",
    "Empty 122",
    "Empty 123",
    "Empty 124",
    "Empty 125",
    "Empty 126",
    "Empty 127",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_client_returns_128_presets() {
        let client = MockClient::new();
        let presets = client.read_presets().unwrap();
        assert_eq!(presets.len(), 128);
    }

    #[test]
    fn mock_preset_indices_are_sequential() {
        let client = MockClient::new();
        let presets = client.read_presets().unwrap();
        for (expected, preset) in presets.iter().enumerate() {
            assert_eq!(
                preset.index, expected as u16,
                "preset at position {expected} has wrong index"
            );
        }
    }

    #[test]
    fn mock_preset_names_are_non_empty() {
        let client = MockClient::new();
        let presets = client.read_presets().unwrap();
        for preset in &presets {
            assert!(
                !preset.name.is_empty(),
                "preset {} has an empty name",
                preset.index
            );
        }
    }

    #[test]
    fn mock_profile_is_hx_stomp_xl() {
        let client = MockClient::new();
        assert_eq!(client.profile().name, "HX Stomp XL");
    }

    #[test]
    fn mock_select_preset_valid_range() {
        let client = MockClient::new();
        assert!(client.select_preset(0, 0).is_ok());
        assert!(client.select_preset(3, 31).is_ok());
    }

    #[test]
    fn mock_select_preset_bank_out_of_range() {
        let client = MockClient::new();
        assert!(client.select_preset(4, 0).is_err());
    }

    #[test]
    fn mock_select_preset_preset_out_of_range() {
        let client = MockClient::new();
        assert!(client.select_preset(0, 32).is_err());
    }
}
