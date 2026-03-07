use crate::{app::App, message::Message, state::AppState};
use iced::widget::{column, container, scrollable, text};
use iced::{Alignment, Element, Length};
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
            let header = text(fl!(
                "connected-header",
                device_name = app.device_name.clone()
            ))
            .size(24);

            let presets_list = app
                .presets
                .iter()
                .fold(column![].spacing(5), |col, preset| {
                    col.push(text(format!("{:03}: {}", preset.index, preset.name)))
                });

            column![header, scrollable(presets_list).height(Length::Fill)]
                .spacing(20)
                .width(Length::Fill)
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
