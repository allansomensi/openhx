use crate::{app::App, message::Message, state::AppState};
use iced::Task;
use openhx_core::{HxError, reset_shared_device, with_device};
use tracing::{debug, error, info};

pub fn handle_message(app: &mut App, message: Message) -> Task<Message> {
    debug!("Received message: {message:?}");

    match message {
        Message::DeviceDetected {
            name,
            setlist_count,
            presets,
        } => {
            info!("Device connected successfully: {name} ({setlist_count} setlist(s))");
            app.device_name = name;
            app.setlist_count = setlist_count;
            app.setlist = 0;
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
            app.setlist_count = 1;
            app.setlist = 0;
            app.presets.clear();
            app.selected_preset = None;
            Task::none()
        }
        Message::SetlistChosen(setlist) => {
            if setlist == app.setlist {
                return Task::none();
            }
            info!("Loading setlist {setlist}");

            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        with_device(|client| client.read_setlist_presets(setlist))
                    })
                    .await
                    .unwrap_or_else(|e| Err(HxError::Protocol(e.to_string())))
                },
                move |result| match result {
                    Ok(presets) => Message::SetlistLoaded(setlist, presets),
                    Err(e) => Message::SetlistLoadFailed(setlist, e.to_string()),
                },
            )
        }
        Message::SetlistLoaded(setlist, presets) => {
            info!("Setlist {setlist} loaded ({} presets)", presets.len());
            app.setlist = setlist;
            app.presets = presets;
            app.selected_preset = None;
            Task::none()
        }
        Message::SetlistLoadFailed(setlist, err) => {
            error!("Failed to load setlist {setlist}: {err}");
            Task::none()
        }
        Message::ConnectionError(err) => {
            error!("Failed to connect to device: {err}");
            app.state = AppState::Error;
            app.error_log = Some(err);
            Task::none()
        }
        Message::PresetSelected(index) => {
            let setlist = app.setlist;
            info!("Preset selected: setlist {setlist}, slot {index:03}");
            app.selected_preset = Some(index);

            tokio::task::spawn_blocking(move || {
                match with_device(|client| client.select_preset(setlist, index)) {
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
