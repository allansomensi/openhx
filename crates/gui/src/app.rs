use crate::{message::Message, state::AppState, update, view, worker};
use iced::{Subscription, Task};
use openhx_core::{Block, Preset, Setlist};

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
    /// Blocks of the device's active preset, in signal order.
    pub blocks: Vec<Block>,
    /// `true` while the active preset's blocks are being read.
    pub blocks_loading: bool,
    /// Why reading the blocks failed, if it did.
    pub blocks_error: Option<String>,
    /// Block whose settings are shown, as `(path, position)`.
    pub selected_block: Option<(u8, u8)>,
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
            blocks: Vec::new(),
            blocks_loading: false,
            blocks_error: None,
            selected_block: None,
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

    /// The block whose settings are shown, if it still exists.
    pub fn selected_block(&self) -> Option<&Block> {
        let (path, position) = self.selected_block?;
        self.blocks
            .iter()
            .find(|b| b.path == path && b.position == position)
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
