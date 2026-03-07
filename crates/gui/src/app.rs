use crate::{message::Message, state::AppState, update, view, worker};
use iced::{Subscription, Task};
use openhx_core::Preset;

pub struct App {
    pub state: AppState,
    pub device_name: String,
    pub presets: Vec<Preset>,
    pub error_log: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            state: AppState::Waiting,
            device_name: String::new(),
            presets: Vec::new(),
            error_log: None,
        }
    }
}

impl App {
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
