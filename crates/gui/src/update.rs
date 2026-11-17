use crate::{app::App, message::Message, state::AppState};
use iced::Task;
use openhx_core::{
    HxError, PRESET_NAME_MAX_LEN, reset_shared_device, validate_preset_name, with_device,
};
use tracing::{debug, error, info};

pub fn handle_message(app: &mut App, message: Message) -> Task<Message> {
    debug!("Received message: {message:?}");

    match message {
        Message::DeviceDetected {
            name,
            setlists,
            presets,
        } => {
            info!(
                "Device connected successfully: {name} ({} setlist(s))",
                setlists.len()
            );
            app.device_name = name;
            app.setlists = setlists;
            app.setlist = 0;
            app.presets = presets;
            app.state = AppState::Connected;
            app.error_log = None;
            app.clear_selection();
            Task::none()
        }
        Message::DeviceDisconnected => {
            info!("Device disconnected");
            reset_shared_device();
            app.state = AppState::Waiting;
            app.device_name.clear();
            app.setlists.clear();
            app.setlist = 0;
            app.presets.clear();
            app.clear_selection();
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
            app.clear_selection();
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
            app.clear_selection();
            app.selected_preset = Some(index);
            if let Some(preset) = app.presets.iter().find(|p| p.index == index) {
                app.rename_input = preset.name.clone();
            }

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
        Message::RenameInputChanged(name) => {
            // Stop typing at the device's limit instead of failing on submit.
            if name.len() <= PRESET_NAME_MAX_LEN {
                app.rename_input = name;
                app.rename_error = None;
            }
            Task::none()
        }
        Message::RenameSubmitted => {
            let Some(preset) = app.selected_preset else {
                return Task::none();
            };
            if app.rename_pending {
                return Task::none();
            }

            let name = app.rename_input.trim_end().to_owned();
            if let Err(e) = validate_preset_name(&name) {
                app.rename_error = Some(e.to_string());
                return Task::none();
            }

            let setlist = app.setlist;
            info!("Renaming setlist {setlist}, slot {preset:03} to {name:?}");
            app.rename_pending = true;
            app.rename_error = None;

            Task::perform(
                async move {
                    let result = tokio::task::spawn_blocking({
                        let name = name.clone();
                        move || with_device(|client| client.rename_preset(setlist, preset, &name))
                    })
                    .await
                    .unwrap_or_else(|e| Err(HxError::Protocol(e.to_string())));
                    (name, result)
                },
                move |(name, result)| match result {
                    Ok(()) => Message::PresetRenamed {
                        setlist,
                        preset,
                        name,
                    },
                    Err(e) => Message::RenameFailed(e.to_string()),
                },
            )
        }
        Message::PresetRenamed {
            setlist,
            preset,
            name,
        } => {
            info!("Renamed setlist {setlist}, slot {preset:03} to {name:?}");
            app.rename_pending = false;

            // The user may have switched setlists while the rename ran.
            if setlist == app.setlist
                && let Some(entry) = app.presets.iter_mut().find(|p| p.index == preset)
            {
                entry.name = name.clone();
            }
            if app.selected_preset == Some(preset) && setlist == app.setlist {
                app.rename_input = name;
            }
            Task::none()
        }
        Message::RenameFailed(err) => {
            error!("Failed to rename preset: {err}");
            app.rename_pending = false;
            app.rename_error = Some(err);
            Task::none()
        }
    }
}
