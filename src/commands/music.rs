use crate::commands::{Context, Error};
use reqwest::Client as HttpClient;
use serenity::async_trait;
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use songbird::input::YoutubeDl;
use std::sync::LazyLock;
use tracing::{error, trace, debug};

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| HttpClient::new());

struct TrackErrorNotifier;

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                error!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}

/// baka
#[poise::command(slash_command, prefix_command)]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
    let manager = songbird::get(ctx.serenity_context())
        .await
        .unwrap()
        .clone();
    let (guild_id, channel_id) = {
        let guild = ctx.guild_id().unwrap();
        let channel_id = ctx.guild().unwrap().voice_states
            .get(&ctx.author().id)
            .and_then(|voice_state| voice_state.channel_id);

        (guild, channel_id)
    };
    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            ctx.reply("Not in a voice channel.").await?;
            return Ok(());
        },
    };
    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
        // Attach an event handler to see notifications of all track errors.
        let mut handler = handler_lock.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
    }
    Ok(())
}

/// baka2
#[poise::command(slash_command, prefix_command)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "Play a track by url or query"] #[rest] query_or_url: String,
) -> Result<(), Error> {
    debug!("Received play command with query_or_url: {} by {} ({})", query_or_url, ctx.author().name, ctx.author().id);
    trace!("Getting songbird manager..");
    let manager = songbird::get(ctx.serenity_context())
        .await
        .unwrap()
        .clone();
    trace!("Getting VC handler...");
    if let Some(handler_lock) = manager.get(ctx.guild_id().unwrap()) {
        let mut handler = handler_lock.lock().await;
        trace!("Cloning HTTP client (inefficient af)...");
        let client = HTTP_CLIENT.clone();
        let search = !query_or_url.starts_with("http") || query_or_url.contains(" ");
        trace!("Search?: {}", search);
        trace!("Begin searching...");
        let src = if search {
            YoutubeDl::new_search(client, query_or_url)
        } else {
            YoutubeDl::new(client, query_or_url)
        };
        trace!("Got result, playing");
        let _ = handler.play_input(src.clone().into());
        trace!("Done (for now).");
    } else {
        ctx.reply("Not in a voice channel.").await?;
    }
    Ok(())
}
