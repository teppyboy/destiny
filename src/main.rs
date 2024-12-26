use crate::{
    config::Config,
    commands::{Context, Error},
    utils::message::{info_reply, send_reply},
};
use commands::music::HttpKey;
use dotenvy::dotenv;
use reqwest::Client as HttpClient;
use serenity::all::Ready;
use serenity::prelude::*;
use serenity::{async_trait, gateway::ActivityData};
use songbird::SerenityInit;
use std::{env, path::Path, sync::Arc};
use tokio::sync::OnceCell;
use tracing::{error, info};

mod commands;
mod config;
mod logging;
mod utils;

static CONFIG: OnceCell<Config> = OnceCell::const_new();

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: serenity::client::Context, ready: Ready) {
        info!(
            "Connected to Discord as '{}#{}'",
            ready.user.name,
            ready.user.discriminator.unwrap()
        );
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 32)]
async fn main() {
    // .env is not required for our code to work.
    let _ = dotenv();
    let discord_token = env::var("DISCORD_TOKEN").expect("Discord token not found.");
    let config: Config;
    if Path::new("./config.toml").exists() {
        config = config::Config::load("./config.toml");
    } else {
        config = config::Config::new();
        println!("Config file not found. Creating a new one...");
        config.save("./config.toml");
    }
    let level_str = config.log.level.clone();
    let log_level = env::var("LOG_LEVEL").unwrap_or(level_str);
    let log_file_name: Option<&str> = match &config.log.file.enabled {
        true => Some(&config.log.file.path),
        false => None,
    };
    logging::setup(&log_level, log_file_name).expect("Failed to setup logging.");
    CONFIG
        .set(config.clone())
        .expect("Failed to register config to global state.");
    info!(
        "Destiny v{} - {}",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_REPOSITORY")
    );
    info!("Log level: {}", log_level);
    info!("Initializing Discord client...");

    // Login with a bot token from the environment
    let mut commands: Vec<
        poise::Command<
            crate::commands::Data,
            Box<(dyn serde::ser::StdError + std::marker::Send + Sync + 'static)>,
        >,
    > = vec![about(), commands::age::age(), commands::ping::ping()];
    if config.features.music_player.enabled {
        info!("Music player enabled.");
        commands.append(&mut commands::music::exports());
    }
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands,
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some(config.general.prefix.into()),
                edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                    std::time::Duration::from_secs(3600),
                ))),
                case_insensitive_commands: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(commands::Data {})
            })
        })
        .build();
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(discord_token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .type_map_insert::<HttpKey>(HttpClient::new())
        .activity(ActivityData::playing("music!"))
        .await
        .expect("Error creating client");

    info!("Starting client...");
    if let Err(why) = client.start_autosharded().await {
        error!("An error occurred while running the client: {:?}", why);
    }
}

/// Shows the latency between the bot and Discord server.
#[poise::command(slash_command, prefix_command)]
pub async fn about(ctx: Context<'_>) -> Result<(), Error> {
    let reply_str = format!(
        "Destiny v{} ({}) - {}\n\
        Log level: `{}`\n\
        Build type: `{}`\n\n\
        Like my work? Consider supporting me at my [Ko-fi](https://ko-fi.com/tretrauit) or [Patreon](https://patreon.com/tretrauit)!",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH"),
        env!("CARGO_PKG_REPOSITORY"),
        CONFIG.get().unwrap().log.level.clone().as_str(),
        env!("BUILD_PROFILE"),
    );
    send_reply(
        &ctx,
        info_reply(
            Some(ctx.serenity_context()),
            reply_str,
            Some("Information".to_string()),
        )
        .await,
    )
    .await;
    Ok(())
}
