use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

pub fn init_tracing() {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,backend=info"));

    let fmt_layer = fmt::layer()
        .with_target(true) // Show module path for better readability
        .with_file(true) // Show file name
        .with_line_number(true) // Show line number
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_ansi(true); // Enable colors for terminal readability
                          // TODO: Switch back to JSON format when log aggregators are set up
                          // .json();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}
