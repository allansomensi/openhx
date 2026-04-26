use crate::{app::App, message::Message, state::AppState};
use iced::Task;
use openhx_core::{HxError, reset_shared_device, with_device};
use tracing::{debug, error, info};

pub fn handle_message(app: &mut App, message: Message) -> Task<Message> {
    debug!("Received message: {message:?}");

    match message {
        Message::DeviceDetected(name, presets) => {
            info!("Device connected successfully: {name}");
            app.device_name = name;
            app.presets = presets;
            app.state = AppState::Connected;
            app.error_log = None;
            app.selected_preset = None;
            Task::none()
        }
        Message::DeviceDisconnected => {
            info!("Device disconnected");
            reset_shared_device();
            app.state = AppState::Waiting;
            app.device_name.clear();
            app.presets.clear();
            app.selected_preset = None;
            Task::none()
        }
        Message::ConnectionError(err) => {
            error!("Failed to connect to device: {err}");
            app.state = AppState::Error;
            app.error_log = Some(err);
            Task::none()
        }
        Message::PresetSelected(index) => {
            info!("Preset selected: {index:03}");
            app.selected_preset = Some(index);

            tokio::task::spawn_blocking(move || {
                match with_device(|client| client.select_preset(0, index)) {
                    Ok(()) => {}
                    Err(HxError::DeviceNotFound) => {
                        error!("Cannot select preset {index:03}: device not connected");
                    }
                    Err(e) => {
                        error!("Failed to select preset {index:03}: {e}");
                    }
                }
            });

            Task::none()
        }
    }
}
