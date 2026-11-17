use crate::{app::App, blocks_view, message::Message, state::AppState};
use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input};
use iced::{Alignment, Color, Element, Length, Padding, Theme};
use openhx_core::Setlist;
use openhx_i18n::fl;

/// Widget ID of the preset list's scrollable.
pub const PRESET_LIST_ID: &str = "preset-list";

/// One entry of the setlist picker, showing the name stored on the device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetlistChoice {
    pub index: u8,
    pub name: String,
}

impl From<&Setlist> for SetlistChoice {
    fn from(setlist: &Setlist) -> Self {
        Self {
            index: setlist.index,
            name: setlist.name.clone(),
        }
    }
}

impl std::fmt::Display for SetlistChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.name.is_empty() {
            let number = u32::from(self.index) + 1;
            write!(f, "{}", fl!("setlist-option", number = number))
        } else {
            write!(f, "{}", self.name)
        }
    }
}

pub fn view(app: &App) -> Element<'_, Message> {
    let content = match app.state {
        AppState::Waiting => column![
            text(fl!("waiting-title")).size(30),
            text(fl!("waiting-subtitle"))
        ]
        .align_x(Alignment::Center)
        .spacing(15),

        AppState::Connected => {
            let badge_text = text(fl!(
                "connected-header",
                device_name = app.device_name.clone()
            ))
            .size(14)
            .style(move |_theme: &Theme| text::Style {
                color: Some(Color::from_rgb(0.2, 0.8, 0.2)),
            });

            let badge = container(badge_text)
                .padding([4, 12])
                .style(move |_theme: &Theme| container::Style {
                    border: iced::Border {
                        color: Color::from_rgb(0.2, 0.8, 0.2),
                        width: 1.0,
                        radius: 12.0.into(),
                    },
                    background: None,
                    ..Default::default()
                });

            let header = container(badge).width(Length::Fill).center_x(Length::Fill);

            let presets_list = app.presets.iter().enumerate().fold(
                column![].spacing(2).width(Length::Fill),
                |col, (i, preset)| {
                    let is_selected = app.selected_preset == Some(preset.index);
                    let label_text = format!("{:03}   {}", preset.index, preset.name);
                    let item_text = text(label_text).size(14);

                    let btn = button(item_text)
                        .width(Length::Fill)
                        .padding(Padding {
                            top: 3.0,
                            right: 20.0,
                            bottom: 3.0,
                            left: 10.0,
                        })
                        .on_press(Message::PresetSelected(preset.index))
                        .style(if is_selected {
                            button::primary
                        } else {
                            button::text
                        });

                    let item = container(btn)
                        .width(Length::Fill)
                        .style(move |_theme: &Theme| container::Style {
                            background: Some(if i % 2 == 0 {
                                Color::from_rgba(0.5, 0.5, 0.5, 0.1).into()
                            } else {
                                Color::TRANSPARENT.into()
                            }),
                            border: iced::Border {
                                radius: 4.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        });

                    col.push(item)
                },
            );

            let list_container = container(
                scrollable(presets_list)
                    .id(PRESET_LIST_ID)
                    .height(Length::Fill),
            )
            .width(Length::Fixed(215.0))
            .padding(5)
            .style(move |_theme: &Theme| container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.02).into()),
                border: iced::Border {
                    color: Color::from_rgba(0.5, 0.5, 0.5, 0.3),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            });

            let list_title = text("Presets").size(18);

            // Only multi-setlist devices (Helix family) get a picker.
            let setlist_picker = (app.setlists.len() > 1).then(|| {
                let choices: Vec<SetlistChoice> =
                    app.setlists.iter().map(SetlistChoice::from).collect();
                let selected = choices.iter().find(|c| c.index == app.setlist).cloned();

                pick_list(choices, selected, |choice| {
                    Message::SetlistChosen(choice.index)
                })
                .width(Length::Fixed(215.0))
                .text_size(14)
            });

            let mut sidebar = column![list_title].spacing(10);
            if let Some(picker) = setlist_picker {
                sidebar = sidebar.push(picker);
            }
            let sidebar = sidebar.push(list_container);

            let main = column![edit_panel(app), blocks_view::panel(app)]
                .spacing(20)
                .width(Length::Fill)
                .height(Length::Fill);

            let body = row![sidebar, main].spacing(20).height(Length::Fill);

            column![header, body]
                .spacing(20)
                .width(Length::Fill)
                .height(Length::Fill)
        }

        AppState::Error => {
            let error_msg = app
                .error_log
                .clone()
                .unwrap_or_else(|| fl!("error-unknown"));

            column![text(fl!("error-title")).size(30), text(error_msg)]
                .align_x(Alignment::Center)
                .spacing(15)
        }
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(20)
        .into()
}

/// Editor for the selected preset; currently its name.
fn edit_panel(app: &App) -> Element<'_, Message> {
    let title = text(fl!("edit-title")).size(18);

    let Some(preset) = app.selected_preset else {
        return column![title, text(fl!("edit-hint")).size(14)]
            .spacing(10)
            .width(Length::Fill)
            .into();
    };

    let label = text(format!("{:03}   {}", preset, fl!("rename-label"))).size(14);

    let mut name_input = text_input(&fl!("rename-placeholder"), &app.rename_input)
        .size(14)
        .padding(6)
        .width(Length::Fill);
    if !app.rename_pending {
        name_input = name_input
            .on_input(Message::RenameInputChanged)
            .on_submit(Message::RenameSubmitted);
    }

    let button_label = if app.rename_pending {
        fl!("rename-in-progress")
    } else {
        fl!("rename-button")
    };
    let rename_button = button(text(button_label).size(14))
        .padding([6, 14])
        .on_press_maybe((!app.rename_pending).then_some(Message::RenameSubmitted));

    let mut panel = column![
        title,
        label,
        row![name_input, rename_button]
            .spacing(8)
            .align_y(Alignment::Center),
    ]
    .spacing(10)
    .width(Length::Fill);

    if let Some(err) = &app.rename_error {
        panel = panel.push(blocks_view::error_text(fl!(
            "rename-failed",
            error = err.clone()
        )));
    }

    panel.into()
}
