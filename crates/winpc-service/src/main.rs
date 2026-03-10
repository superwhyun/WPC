fn main() {
    winpc_service::init_tracing();
    if let Err(error) = winpc_service::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
