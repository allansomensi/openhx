use crate::{
    app::App,
    blocks_view::{DEVICE_TYPE_LIST_ID, MODULE_LIST_ID},
    message::Message,
    state::AppState,
    view::PRESET_LIST_ID,
};
use iced::Task;
use iced::widget::operation::{RelativeOffset, snap_to};
use openhx_core::{
    ActivePreset, Block, BlockCategory, HxError, PRESET_NAME_MAX_LEN, blocks::models_in,
    reset_shared_device, validate_preset_name, with_device,
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
            load_blocks(app, None)
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
            app.blocks.clear();
            app.blocks_loading = false;
            app.blocks_error = None;
            app.selected_block = None;
            app.browse_category = None;
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

            // The selected preset becomes the active one, so its blocks are
            // read right after switching.
            load_blocks(app, Some((setlist, index)))
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
        Message::BlocksRequested => load_blocks(app, None),
        Message::ActivePresetLoaded { active, blocks } => {
            info!(
                "Active preset: setlist {}, slot {:03}, {} block(s)",
                active.setlist,
                active.preset,
                blocks.len()
            );
            let scroll = show_active_preset(app, active);
            app.blocks = blocks;
            app.blocks_loading = false;
            app.blocks_error = None;
            // Keep the selection only if that position still holds a block.
            if app.selected_block().is_none() {
                app.selected_block = None;
            }
            app.browse_category = None;
            scroll
        }
        Message::BlocksLoadFailed(err) => {
            error!("Failed to read the active preset's blocks: {err}");
            app.blocks_loading = false;
            app.blocks_error = Some(err);
            Task::none()
        }
        Message::BlockSelected(path, position) => {
            app.selected_block = Some((path, position));
            app.browse_category = None;
            scroll_lists_to_block(app)
        }
        Message::DeviceTypeChosen(category) => {
            app.browse_category = Some(category);
            scroll_lists_to_block(app)
        }
    }
}

/// Scrolls the device type and module lists so the browsed type and the
/// selected block's model are in view.
fn scroll_lists_to_block(app: &App) -> Task<Message> {
    let Some(block) = app.selected_block() else {
        return Task::none();
    };
    let Some(category) = app.browse_category.or_else(|| block.category()) else {
        return Task::none();
    };

    let type_index = BlockCategory::ALL
        .iter()
        .position(|&c| c == category)
        .unwrap_or(0);
    let models = models_in(category);
    // Show the block's model when browsing its type, else the list's top.
    let model_index = models
        .iter()
        .position(|m| m.id == block.model_id())
        .unwrap_or(0);

    Task::batch([
        snap_to(
            DEVICE_TYPE_LIST_ID,
            relative(type_index, BlockCategory::ALL.len()),
        ),
        snap_to(MODULE_LIST_ID, relative(model_index, models.len())),
    ])
}

/// The scroll offset that brings row `index` of `len` rows into view.
fn relative(index: usize, len: usize) -> RelativeOffset {
    let last = len.saturating_sub(1).max(1);
    RelativeOffset {
        x: 0.0,
        y: index as f32 / last as f32,
    }
}

/// Marks `active` as the selected preset, as on the device itself, and
/// scrolls the preset list to it.
///
/// Only presets of the shown setlist can be marked, and a rename in progress
/// keeps its selection.
fn show_active_preset(app: &mut App, active: ActivePreset) -> Task<Message> {
    if active.setlist != app.setlist
        || app.rename_pending
        || app.selected_preset == Some(active.preset)
    {
        return Task::none();
    }

    app.clear_selection();
    app.selected_preset = Some(active.preset);
    let Some(position) = app.presets.iter().position(|p| p.index == active.preset) else {
        return Task::none();
    };
    app.rename_input = app.presets[position].name.clone();

    snap_to(PRESET_LIST_ID, relative(position, app.presets.len()))
}

/// Reads which preset is active and its blocks, first switching to `select`
/// (a `(setlist, slot)` pair) when given.
fn load_blocks(app: &mut App, select: Option<(u8, u8)>) -> Task<Message> {
    app.blocks_loading = true;
    app.blocks_error = None;

    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || -> Result<(ActivePreset, Vec<Block>), HxError> {
                with_device(|client| {
                    if let Some((setlist, preset)) = select {
                        client.select_preset(setlist, preset)?;
                    }
                    let active = client.read_active_preset()?;
                    let blocks = client.read_active_preset_blocks()?;
                    Ok((active, blocks))
                })
            })
            .await
            .unwrap_or_else(|e| Err(HxError::Protocol(e.to_string())))
        },
        |result| match result {
            Ok((active, blocks)) => Message::ActivePresetLoaded { active, blocks },
            Err(e) => Message::BlocksLoadFailed(e.to_string()),
        },
    )
}
