//! The active preset's signal chain and the settings of the selected block.
//!
//! Laid out like the block view of helix_usb: a strip per signal path from
//! input to output with one colored box per position, and below it the
//! selected block's type, model and settings.

use crate::{app::App, message::Message};
use iced::widget::{Space, button, column, container, progress_bar, row, scrollable, space, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use openhx_core::{
    Block, BlockCategory, ParamValue,
    blocks::{PATH_COUNT, POSITIONS_PER_PATH},
};
use openhx_i18n::fl;

/// Size of one block box in the chain.
const BLOCK_WIDTH: f32 = 46.0;
const BLOCK_HEIGHT: f32 = 52.0;
/// Width of the connector drawn between two boxes.
const CONNECTOR_WIDTH: f32 = 10.0;

/// Color of the signal line between blocks.
const LINE_COLOR: Color = Color::from_rgb(0.839, 0.573, 0.235);
const EMPTY_COLOR: Color = Color::from_rgb(0.18, 0.19, 0.21);
const UNKNOWN_COLOR: Color = Color::from_rgb(0.427, 0.427, 0.427);
const MUTED_TEXT: Color = Color::from_rgb(0.6, 0.6, 0.6);
const ERROR_TEXT: Color = Color::from_rgb(0.9, 0.3, 0.3);

/// The color a block of `category` is drawn in.
fn category_color(category: Option<BlockCategory>) -> Color {
    let rgb = |hex: u32| Color::from_rgb8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8);
    match category {
        Some(BlockCategory::Distortion) => rgb(0xD6923C),
        Some(BlockCategory::Dynamic | BlockCategory::Eq) => rgb(0xB9A03C),
        Some(BlockCategory::Modulation) => rgb(0x4F74BB),
        Some(BlockCategory::Delay) => rgb(0x4DA05F),
        Some(BlockCategory::Reverb) => rgb(0xBF6B2D),
        Some(BlockCategory::PitchSynth | BlockCategory::Filter | BlockCategory::Wah) => {
            rgb(0x8564BF)
        }
        Some(
            BlockCategory::Amp | BlockCategory::AmpCab | BlockCategory::Preamp | BlockCategory::Cab,
        ) => rgb(0xBE4D4D),
        Some(BlockCategory::ImpulseResponse) => rgb(0xB96695),
        Some(BlockCategory::VolumePan | BlockCategory::SendReturn) => rgb(0x3F9C9A),
        None => UNKNOWN_COLOR,
    }
}

pub fn panel(app: &App) -> Element<'_, Message> {
    let refresh = button(text(fl!("blocks-refresh")).size(13))
        .padding([5, 12])
        .on_press_maybe((!app.blocks_loading).then_some(Message::BlocksRequested));

    let header = row![
        text(fl!("blocks-title")).size(18),
        space::horizontal(),
        refresh
    ]
    .align_y(Alignment::Center);

    let mut content = column![header].spacing(14).width(Length::Fill);

    if app.blocks_loading {
        content = content.push(muted(fl!("blocks-loading")));
    }
    if let Some(err) = &app.blocks_error {
        content = content.push(
            text(fl!("blocks-load-failed", error = err.clone()))
                .size(13)
                .style(|_theme: &Theme| text::Style {
                    color: Some(ERROR_TEXT),
                }),
        );
    }

    if !app.blocks_loading && app.blocks_error.is_none() && app.blocks.is_empty() {
        content = content.push(muted(fl!("blocks-empty")));
    } else if !app.blocks.is_empty() {
        content = content.push(chain(app));
        content = content.push(details(app));
    }

    content.into()
}

/// One strip per signal path; the second path only when it holds blocks.
fn chain(app: &App) -> Element<'_, Message> {
    let mut strips = column![].spacing(10);

    for path in 0..PATH_COUNT {
        let used = app.blocks.iter().any(|b| b.path == path);
        if path > 0 && !used {
            continue;
        }
        strips = strips.push(path_strip(app, path));
    }

    container(strips)
        .padding(12)
        .width(Length::Fill)
        .style(panel_style)
        .into()
}

fn path_strip(app: &App, path: u8) -> Element<'_, Message> {
    let mut strip = row![endpoint()].align_y(Alignment::Center);

    for position in 0..POSITIONS_PER_PATH {
        let block = app
            .blocks
            .iter()
            .find(|b| b.path == path && b.position == position);
        let selected = app.selected_block == Some((path, position));

        strip = strip.push(connector());
        strip = strip.push(block_box(block, path, position, selected));
    }

    strip.push(connector()).push(endpoint()).into()
}

/// The ○ marking a path's input or output.
fn endpoint<'a>() -> Element<'a, Message> {
    container(Space::new().width(12).height(12))
        .style(|_theme: &Theme| container::Style {
            border: Border {
                color: LINE_COLOR,
                width: 2.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn connector<'a>() -> Element<'a, Message> {
    container(Space::new().width(CONNECTOR_WIDTH).height(2))
        .style(|_theme: &Theme| container::Style {
            background: Some(LINE_COLOR.into()),
            ..Default::default()
        })
        .into()
}

/// A position in the chain: a colored box for a block, a dark one when empty.
fn block_box(
    block: Option<&Block>,
    path: u8,
    position: u8,
    selected: bool,
) -> Element<'_, Message> {
    let (label, background) = match block {
        Some(block) => {
            let category = block.category();
            let mut color = category_color(category);
            if !block.enabled {
                color.a = 0.35;
            }
            (category.map_or("???", |c| c.abbreviation()), color)
        }
        None => ("", EMPTY_COLOR),
    };

    let content = container(text(label).size(12))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    let mut tile = button(content)
        .width(BLOCK_WIDTH)
        .height(BLOCK_HEIGHT)
        .padding(0)
        .style(move |_theme: &Theme, status| {
            let hovered = matches!(status, button::Status::Hovered);
            button::Style {
                background: Some(Background::Color(background)),
                text_color: Color::WHITE,
                border: Border {
                    color: if selected || hovered {
                        Color::WHITE
                    } else {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.12)
                    },
                    width: if selected { 2.0 } else { 1.0 },
                    radius: 6.0.into(),
                },
                ..Default::default()
            }
        });
    if block.is_some() {
        tile = tile.on_press(Message::BlockSelected(path, position));
    }

    tile.into()
}

/// Type, model and settings of the selected block.
fn details(app: &App) -> Element<'_, Message> {
    let Some(block) = app.selected_block() else {
        return muted(fl!("block-hint"));
    };

    let category = block
        .category()
        .map_or_else(|| fl!("block-unknown-category"), |c| c.label().to_owned());
    let state = if block.enabled {
        fl!("block-on")
    } else {
        fl!("block-off")
    };

    // Shown 1-based, like the device does.
    let path_number: u32 = u32::from(block.path) + 1;
    let position_number: u32 = u32::from(block.position) + 1;

    let info = column![
        field(fl!("block-device-type"), category),
        field(fl!("block-module-name"), block.name()),
        field(
            fl!(
                "block-position",
                path = path_number,
                position = position_number
            ),
            state
        ),
    ]
    .spacing(12)
    .width(Length::FillPortion(2));

    let mut settings = column![].spacing(6);
    for (i, value) in block.params.iter().enumerate() {
        let number: usize = i + 1;
        settings = settings.push(param_row(fl!("block-param", number = number), value));
    }
    for (i, value) in block.cab_params.iter().enumerate() {
        let number: usize = i + 1;
        settings = settings.push(param_row(fl!("block-cab-param", number = number), value));
    }

    let settings_panel = column![
        text(fl!("block-settings")).size(14),
        muted(fl!("block-read-only")),
        scrollable(settings).height(Length::Fill),
    ]
    .spacing(8)
    .width(Length::FillPortion(5));

    container(row![info, settings_panel].spacing(20))
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(panel_style)
        .into()
}

/// A labeled value in the block info column.
fn field<'a>(label: String, value: String) -> Element<'a, Message> {
    column![muted(label), text(value).size(14)]
        .spacing(2)
        .into()
}

/// One parameter: its label, a bar for values on the 0–1 knob scale, and
/// the raw value.
fn param_row<'a>(label: String, value: &ParamValue) -> Element<'a, Message> {
    let bar: Element<'a, Message> = match value {
        ParamValue::Float(v) if (0.0..=1.0).contains(v) => progress_bar(0.0..=1.0, *v as f32)
            .girth(8)
            .style(|_theme: &Theme| progress_bar::Style {
                background: Color::from_rgb(0.22, 0.23, 0.25).into(),
                bar: LINE_COLOR.into(),
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
            })
            .into(),
        _ => Space::new().width(Length::Fill).into(),
    };

    let shown = match value {
        ParamValue::Bool(true) => fl!("block-on"),
        ParamValue::Bool(false) => fl!("block-off"),
        other => other.to_string(),
    };

    row![
        text(label).size(13).width(150),
        container(bar).width(Length::Fill),
        text(shown).size(13).width(70).align_x(Alignment::End),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn muted<'a>(content: String) -> Element<'a, Message> {
    text(content)
        .size(13)
        .style(|_theme: &Theme| text::Style {
            color: Some(MUTED_TEXT),
        })
        .into()
}

fn panel_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.15).into()),
        border: Border {
            color: Color::from_rgba(0.5, 0.5, 0.5, 0.3),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}
