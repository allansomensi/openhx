#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    openhx_i18n::localize();
    openhx_gui::run().unwrap();
}
