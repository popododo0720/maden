pub use log::{debug, error, info, trace, warn};

pub fn init() {
    let mut builder = env_logger::Builder::from_env(env_logger::Env::default());

    #[cfg(debug_assertions)]
    builder.filter_level(log::LevelFilter::Debug);

    #[cfg(not(debug_assertions))]
    builder.filter_level(log::LevelFilter::Info);

    builder
        .format_timestamp_millis()
        .format_level(true)
        .format_module_path(true)
        .format_line_number(true)
        .init();
}