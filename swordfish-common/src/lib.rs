pub use log;
pub use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::{self, fmt, EnvFilter};
pub mod constants;
pub mod tesseract;

pub fn setup_logger(level: &str) -> Result<(), fern::InitError> {
    // I don't really know how to do it because the unset variable trick doesn't work
    // since the types can be
    let formatter = fmt::format()
        .with_level(true)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false); // include the name of the current thread.pretty();
    let filter = EnvFilter::builder()
        .from_env()
        .unwrap()
        .add_directive(
            format!("swordfish={}", level.to_lowercase())
                .parse()
                .unwrap(),
        )
        .add_directive(
            format!("swordfish-common={}", level.to_lowercase())
                .parse()
                .unwrap(),
        );
    tracing_subscriber::fmt()
        .event_format(formatter)
        .with_env_filter(filter)
        .init();
    Ok(())
}
