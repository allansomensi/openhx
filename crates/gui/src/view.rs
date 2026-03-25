use crate::{app::App, message::Message, state::AppState};
use iced::widget::{column, container, scrollable, text};
use iced::{Alignment, Color, Element, Length, Padding, Theme};
use openhx_i18n::fl;

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
                    let label_text = format!("{:03}   {}", preset.index, preset.name);
                    let item_text = text(label_text).size(14);

                    let item = container(item_text)
                        .width(Length::Fill)
                        .padding(Padding {
                            top: 3.0,
                            right: 20.0,
                            bottom: 3.0,
                            left: 10.0,
                        })
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

            let list_container = container(scrollable(presets_list).height(Length::Fill))
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

            let sidebar = column![list_title, list_container].spacing(10);

            column![header, sidebar]
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
