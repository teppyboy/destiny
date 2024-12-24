use tracing_subscriber::{self, EnvFilter, fmt};

pub fn setup(level: &str) -> Result<(), ()> {
    let formatter = fmt::format()
        .with_level(true)
        .with_target(true)
        .with_thread_ids(false)
        .with_line_number(true)
        .with_thread_names(false);
    let filter = EnvFilter::builder()
        .from_env()
        .unwrap()
        .add_directive(format!("destiny={}", level.to_lowercase()).parse().unwrap());
    tracing_subscriber::fmt()
        .event_format(formatter)
        .with_env_filter(filter)
        .init();
    Ok(())
}
