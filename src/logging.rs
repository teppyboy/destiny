use chrono::Local;
use std::{fs::OpenOptions, fs::create_dir, path::Path};
use tracing_subscriber::{
    self, EnvFilter, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

pub fn setup(level: &str, file_name: Option<&str>) -> Result<(), ()> {
    let formatter = fmt::format()
        .with_level(true)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .with_thread_names(false);
    let filter = EnvFilter::builder()
        .from_env()
        .unwrap()
        .add_directive(format!("destiny={}", level.to_lowercase()).parse().unwrap());
    // This is dumb af but it works.
    if file_name.is_some() {
        if !Path::new("./log").exists() {
            create_dir("./log").unwrap();
        }
        let actual_file_name = Local::now().format(file_name.unwrap()).to_string();
        let log_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(format!("./log/{}", actual_file_name))
            .unwrap();
        let subscriber = Registry::default()
            .with(fmt::layer().event_format(formatter).with_ansi(true))
            .with(fmt::layer().with_ansi(false).with_writer(log_file))
            .with(filter);
        subscriber.init();
    } else {
        let subscriber = tracing_subscriber::fmt()
            .event_format(formatter)
            .with_env_filter(filter);
        subscriber.init();
    }
    Ok(())
}
