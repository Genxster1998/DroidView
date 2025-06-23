use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

pub fn init_logging() {
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    info!("DroidView logging initialized");
}
