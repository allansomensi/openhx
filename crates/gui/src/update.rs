use crate::{app::App, message::Message, state::AppState};
use iced::Task;

pub fn handle_message(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::DeviceDetected(name, presets) => {
            app.device_name = name;
            app.presets = presets;
            app.state = AppState::Connected;
            app.error_log = None;
            Task::none()
        }
        Message::DeviceDisconnected => {
            app.state = AppState::Waiting;
            app.device_name.clear();
            app.presets.clear();
            Task::none()
        }
        Message::ConnectionError(err) => {
            app.state = AppState::Error;
            app.error_log = Some(err);
            Task::none()
        }
    }
}
