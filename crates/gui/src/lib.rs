use crate::app::App;

pub mod app;
mod message;
pub mod state;
mod update;
mod view;
mod worker;

pub fn run() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(|_app: &App| "OpenHX".to_string())
        .theme(|_app: &App| iced::Theme::Dark)
        .subscription(|app: &App| app.subscription())
        .centered()
        .window_size((800.0, 600.0))
        .run()
}
