fn main() {
    openhx_i18n::localize();

    if let Err(e) = openhx_cli::run() {
        eprintln!("Error: {e}");

        std::process::exit(1);
    }
}
