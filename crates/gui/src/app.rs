use crate::{message::Message, state::AppState, update, view, worker};
use iced::{Subscription, Task};
use openhx_core::{Block, BlockCategory, Preset, PresetChain, Setlist};

pub struct App {
    pub state: AppState,
    pub device_name: String,
    /// The connected device's setlists, named as on the device.
    pub setlists: Vec<Setlist>,
    /// Setlist whose presets are currently shown.
    pub setlist: u8,
    pub presets: Vec<Preset>,
    pub error_log: Option<String>,
    pub selected_preset: Option<u8>,
    /// Name being edited for the selected preset.
    pub rename_input: String,
    /// `true` while a rename is in flight; blocks a second submit.
    pub rename_pending: bool,
    /// Why the last rename failed, shown under the name field.
    pub rename_error: Option<String>,
    /// The active preset's blocks and the block view's state.
    pub blocks: BlocksState,
}

/// The active preset's signal chain as shown in the block view.
#[derive(Debug, Default)]
pub struct BlocksState {
    /// The device's active preset.
    pub chain: PresetChain,
    /// `true` while the active preset's blocks are being read.
    pub loading: bool,
    /// Why reading the blocks failed, if it did.
    pub error: Option<String>,
    /// Block whose settings are shown, as `(path, position)`.
    pub selected: Option<(u8, u8)>,
    /// Device type picked in the type list; `None` follows the selected
    /// block's own type.
    pub browse_category: Option<BlockCategory>,
}

impl BlocksState {
    /// The block whose settings are shown, if it still exists.
    pub fn selected_block(&self) -> Option<&Block> {
        let (path, position) = self.selected?;
        self.chain.block_at(path, position)
    }

    /// The device type whose models the module list shows: the one picked
    /// in the type list, else `block`'s own.
    pub fn browsed_category(&self, block: &Block) -> Option<BlockCategory> {
        self.browse_category.or_else(|| block.category())
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            state: AppState::Waiting,
            device_name: String::new(),
            setlists: Vec::new(),
            setlist: 0,
            presets: Vec::new(),
            error_log: None,
            selected_preset: None,
            rename_input: String::new(),
            rename_pending: false,
            rename_error: None,
            blocks: BlocksState::default(),
        }
    }
}

impl App {
    /// Clears the selection together with any rename in progress.
    pub fn clear_selection(&mut self) {
        self.selected_preset = None;
        self.rename_input.clear();
        self.rename_pending = false;
        self.rename_error = None;
    }

    /// Selects preset slot `index` of the shown setlist and offers its name
    /// for renaming.
    pub fn select_preset(&mut self, index: u8) {
        self.clear_selection();
        self.selected_preset = Some(index);
        if let Some(preset) = self.presets.iter().find(|p| p.index == index) {
            self.rename_input = preset.name.clone();
        }
    }

    pub fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        update::handle_message(self, message)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match self.state {
            AppState::Waiting => Subscription::run(worker::usb_poll),
            AppState::Connected => Subscription::run(worker::usb_check_disconnect),
            _ => Subscription::none(),
        }
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        view::view(self)
    }
}
