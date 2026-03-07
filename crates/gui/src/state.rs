#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Waiting,
    Connected,
    Error,
}
