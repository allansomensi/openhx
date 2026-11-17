//! The active preset's signal chain and the settings of the selected block.
//!
//! Laid out like the block view of helix_usb: path 1 from input to output
//! with one colored box per position, path 2 below it between the split and
//! the join, and below that the selected block's type, model and settings.

use crate::{app::App, message::Message};
use iced::widget::{
    Space, button, column, container, progress_bar, row, scrollable, space, stack, text,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Theme};
use openhx_core::{
    Block, BlockCategory, ParamDef, ParamValue, PathSplit,
    blocks::{POSITIONS_PER_PATH, models_in},
};
use openhx_i18n::fl;

/// Size of one block box in the chain.
const BLOCK_WIDTH: f32 = 46.0;
const BLOCK_HEIGHT: f32 = 52.0;
/// Width of the gap between two boxes, holding the connecting lines.
const CONNECTOR_WIDTH: f32 = 14.0;
/// Space between path 1's row and path 2's row.
const ROW_GAP: f32 = 16.0;
/// Thickness of the signal lines.
const LINE_WIDTH: f32 = 2.0;

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
        .on_press_maybe((!app.blocks.loading).then_some(Message::BlocksRequested));

    let header = row![
        text(fl!("blocks-title")).size(18),
        space::horizontal(),
        refresh
    ]
    .align_y(Alignment::Center);

    let mut content = column![header].spacing(14).width(Length::Fill);

    let blocks = &app.blocks;
    if blocks.loading {
        content = content.push(muted(fl!("blocks-loading")));
    }
    if let Some(err) = &blocks.error {
        content = content.push(error_text(fl!("blocks-load-failed", error = err.clone())));
    }

    if !blocks.loading && blocks.error.is_none() && blocks.chain.blocks.is_empty() {
        content = content.push(muted(fl!("blocks-empty")));
    } else if !blocks.chain.blocks.is_empty() {
        content = content.push(chain(app));
        content = content.push(details(app));
    }

    content.into()
}

/// The signal chain: path 1 from input to output and, when it holds blocks,
/// path 2 below it, branching off at the split and merging back at the join.
///
/// The chain is laid out in columns: an endpoint, then alternately a gap
/// and a position, then the other endpoint. Each column spans both rows, so
/// a gap can draw the vertical line between them.
fn chain(app: &App) -> Element<'_, Message> {
    let split = app.blocks.chain.path2_span();

    let mut strip = row![terminal()];
    for position in 0..POSITIONS_PER_PATH {
        strip = strip.push(gap(position, split));
        strip = strip.push(position_column(app, position, split));
    }
    strip = strip.push(gap(POSITIONS_PER_PATH, split)).push(terminal());

    container(strip)
        .padding(12)
        .width(Length::Fill)
        .style(panel_style)
        .into()
}

/// Vertical center of path 1's row (`0`) or path 2's row (`1`).
fn row_center(row: u8) -> f32 {
    f32::from(row) * (BLOCK_HEIGHT + ROW_GAP) + BLOCK_HEIGHT / 2.0
}

/// The column of one position: path 1's box and, below it, path 2's box
/// when the position lies between split and join.
fn position_column(app: &App, position: u8, split: Option<PathSplit>) -> Element<'_, Message> {
    let chain = &app.blocks.chain;
    let tile = |path: u8, block| {
        let selected = app.blocks.selected == Some((path, position));
        block_box(block, path, position, selected)
    };

    let mut column = column![tile(0, chain.block_at(0, position))];
    if let Some(PathSplit { split, join }) = split {
        let lower = chain.block_at(1, position);
        let on_path2 = (split..join).contains(&position) || lower.is_some();
        column = column.push(Space::new().height(ROW_GAP));
        column = column.push(if on_path2 {
            tile(1, lower)
        } else {
            Space::new().width(BLOCK_WIDTH).height(BLOCK_HEIGHT).into()
        });
    }
    column.into()
}

/// The lines in the gap before position `gap` (or before the output).
fn gap<'a>(gap: u8, split: Option<PathSplit>) -> Element<'a, Message> {
    let height = if split.is_some() {
        2.0 * BLOCK_HEIGHT + ROW_GAP
    } else {
        BLOCK_HEIGHT
    };
    let middle = CONNECTOR_WIDTH / 2.0;

    let mut lines = stack![
        Space::new().width(CONNECTOR_WIDTH).height(height),
        line(0.0, row_center(0), CONNECTOR_WIDTH, LINE_WIDTH),
    ];
    if let Some(PathSplit { split, join }) = split {
        let (top, bottom) = (row_center(0), row_center(1));
        let vertical = || line(middle - LINE_WIDTH / 2.0, top, LINE_WIDTH, bottom - top);
        if gap == split {
            lines = lines.push(vertical());
            lines = lines.push(line(middle, bottom, CONNECTOR_WIDTH - middle, LINE_WIDTH));
        }
        if gap == join {
            lines = lines.push(vertical());
            lines = lines.push(line(0.0, bottom, middle, LINE_WIDTH));
        }
        if split < gap && gap < join {
            lines = lines.push(line(0.0, bottom, CONNECTOR_WIDTH, LINE_WIDTH));
        }
    }
    lines.into()
}

/// A line segment at `(x, y)` (its center for horizontal lines' `y`) of the
/// given size, within a gap.
fn line<'a>(x: f32, y: f32, width: f32, height: f32) -> Element<'a, Message> {
    let top = if height <= LINE_WIDTH {
        y - LINE_WIDTH / 2.0
    } else {
        y
    };
    container(
        container(Space::new().width(width).height(height)).style(|_theme: &Theme| {
            container::Style {
                background: Some(LINE_COLOR.into()),
                ..Default::default()
            }
        }),
    )
    .padding(Padding {
        top,
        left: x,
        right: 0.0,
        bottom: 0.0,
    })
    .into()
}

/// The ○ marking a path's input or output, centered on path 1's row.
fn terminal<'a>() -> Element<'a, Message> {
    let ring =
        container(Space::new().width(12).height(12)).style(|_theme: &Theme| container::Style {
            border: Border {
                color: LINE_COLOR,
                width: 2.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        });
    container(ring)
        .height(BLOCK_HEIGHT)
        .center_y(BLOCK_HEIGHT)
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

/// Widget IDs of the device type and module name lists, for scrolling them
/// to the selected block.
pub const DEVICE_TYPE_LIST_ID: &str = "device-type-list";
pub const MODULE_LIST_ID: &str = "module-list";

/// Device type list, module name list and settings of the selected block.
fn details(app: &App) -> Element<'_, Message> {
    let Some(block) = app.blocks.selected_block() else {
        return muted(fl!("block-hint"));
    };

    let row = row![
        device_type_list(app, block).width(Length::FillPortion(2)),
        module_list(app, block).width(Length::FillPortion(3)),
        settings_panel(block).width(Length::FillPortion(6)),
    ]
    .spacing(10)
    .height(Length::Fill);

    container(row)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Every block category; the browsed one is highlighted and picking another
/// shows its models.
fn device_type_list<'a>(app: &App, block: &Block) -> iced::widget::Container<'a, Message> {
    let browsed = app.blocks.browsed_category(block);

    let entries = BlockCategory::ALL
        .iter()
        .fold(column![].spacing(1), |col, &category| {
            col.push(list_entry(
                category.label(),
                browsed == Some(category),
                Some(Message::DeviceTypeChosen(category)),
            ))
        });

    list_panel(fl!("block-device-type"), entries, DEVICE_TYPE_LIST_ID)
}

/// The models of the browsed category, with the block's own model
/// highlighted. Read-only: picking a model is not possible yet.
fn module_list<'a>(app: &App, block: &Block) -> iced::widget::Container<'a, Message> {
    let current = block.model_id();

    let entries = app
        .blocks
        .browsed_category(block)
        .map_or(&[][..], models_in)
        .iter()
        .fold(column![].spacing(1), |col, model| {
            col.push(list_entry(model.name, model.id == current, None))
        });

    list_panel(fl!("block-module-name"), entries, MODULE_LIST_ID)
}

/// The block's position, state and parameter values.
fn settings_panel(block: &Block) -> iced::widget::Container<'_, Message> {
    let state = if block.enabled {
        fl!("block-on")
    } else {
        fl!("block-off")
    };

    // Shown 1-based, like the device does.
    let path_number: u32 = u32::from(block.path) + 1;
    let position_number: u32 = u32::from(block.position) + 1;
    let location = fl!(
        "block-position",
        path = path_number,
        position = position_number
    );

    // Parameters without a definition are numbered from 1.
    let params = block
        .params_with_defs()
        .zip(1usize..)
        .map(|((value, def), number)| {
            let label = def.map_or_else(
                || fl!("block-param", number = number),
                |d| d.name.to_owned(),
            );
            param_row(label, value, def)
        });
    let cab_params = block
        .cab_params_with_defs()
        .zip(1usize..)
        .map(|((value, def), number)| {
            let label = def.map_or_else(
                || fl!("block-cab-param", number = number),
                |d| fl!("block-cab-param-named", name = d.name),
            );
            param_row(label, value, def)
        });
    let settings = column(params.chain(cab_params)).spacing(6);

    let content = column![
        text(fl!("block-settings")).size(14),
        text(format!("{location} · {state}")).size(13),
        muted(fl!("block-read-only")),
        scrollable(settings).height(Length::Fill),
    ]
    .spacing(8);

    container(content)
        .padding(10)
        .height(Length::Fill)
        .style(panel_style)
}

/// A titled panel holding a scrollable list.
fn list_panel<'a>(
    title: String,
    entries: iced::widget::Column<'a, Message>,
    id: &'static str,
) -> iced::widget::Container<'a, Message> {
    let content = column![
        text(title).size(14),
        scrollable(entries).id(id).height(Length::Fill),
    ]
    .spacing(8);

    container(content)
        .padding(10)
        .height(Length::Fill)
        .style(panel_style)
}

/// One list row; clickable when `on_press` is given, highlighted when
/// `selected`.
fn list_entry<'a>(
    label: &'static str,
    selected: bool,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let background = selected.then_some(Background::Color(Color {
        a: 0.45,
        ..LINE_COLOR
    }));

    let entry = button(text(label).size(13))
        .width(Length::Fill)
        .padding([2, 6])
        .on_press_maybe(on_press)
        // A read-only entry has no `on_press`; draw it like an active one
        // instead of greyed out.
        .style(move |theme: &Theme, status| {
            let hovered = matches!(status, button::Status::Hovered);
            button::Style {
                background: background
                    .or_else(|| hovered.then(|| Color::from_rgba(1.0, 1.0, 1.0, 0.06).into())),
                text_color: theme.palette().text,
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    entry.into()
}

/// One parameter: its label, a bar for values on the 0–1 knob scale, and
/// the raw value.
fn param_row<'a>(
    label: String,
    value: &ParamValue,
    def: Option<&ParamDef>,
) -> Element<'a, Message> {
    // Without a definition only values that look like a 0–1 knob get a bar.
    let fraction = match (def, value) {
        (Some(def), _) => def.fraction(value),
        (None, ParamValue::Float(v)) if (0.0..=1.0).contains(v) => Some(*v as f32),
        (None, _) => None,
    };
    let bar: Element<'a, Message> = match fraction {
        Some(fraction) => progress_bar(0.0..=1.0, fraction)
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
        None => Space::new().width(Length::Fill).into(),
    };

    let shown = def.map_or_else(|| value.to_string(), |def| def.format(value));

    row![
        text(label).size(13).width(150),
        container(bar).width(Length::Fill),
        text(shown).size(13).width(80).align_x(Alignment::End),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

/// A line of text in the error color.
pub fn error_text<'a>(content: String) -> Element<'a, Message> {
    text(content)
        .size(13)
        .style(|_theme: &Theme| text::Style {
            color: Some(ERROR_TEXT),
        })
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
