use crate::app::App;

pub mod app;
mod message;
pub mod state;
mod update;
mod view;
mod worker;

pub fn run() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    tracing::info!("Starting OpenHX...");

    iced::application(App::new, App::update, App::view)
        .title(|_app: &App| "OpenHX".to_string())
        .theme(|_app: &App| iced::Theme::Dark)
        .subscription(|app: &App| app.subscription())
        .centered()
        .window_size((800.0, 600.0))
        .run()
}
