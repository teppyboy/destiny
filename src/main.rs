use crate::config::Config;
use dotenvy::dotenv;
use serenity::gateway::ActivityData;
use serenity::prelude::*;
use std::{env, path::Path};
use tokio::sync::OnceCell;
use tracing::{error, info};

mod config;
mod logging;

static CONFIG: OnceCell<Config> = OnceCell::const_new();

#[tokio::main(flavor = "multi_thread", worker_threads = 32)]
async fn main() {
    // .env is not required for our code to work.
    match dotenv() {
        Ok(_) => {}
        Err(_) => {}
    }
    let discord_token = env::var("DISCORD_TOKEN").expect("Discord token not found.");
    let config: Config;
    if Path::new("./config.toml").exists() {
        config = config::Config::load("./config.toml");
    } else {
        config = config::Config::new();
        config.save("./config.toml");
    }
    let level_str = config.log.level.clone();
    let log_level = env::var("LOG_LEVEL").unwrap_or(level_str);
    logging::setup(&log_level).expect("Failed to setup logging.");
    CONFIG
        .set(config)
        .expect("Failed to register config to global state.");
    info!("Destiny v{} - dev dev", env!("CARGO_PKG_VERSION"));
    info!("Log level: {}", log_level);
    info!("Initializing Discord client...");

    // Login with a bot token from the environment
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(discord_token, intents)
        .activity(ActivityData::playing("music"))
        .await
        .expect("Error creating client");

    info!("Starting client...");
    if let Err(why) = client.start_autosharded().await {
        error!("An error occurred while running the client: {:?}", why);
    }
}
