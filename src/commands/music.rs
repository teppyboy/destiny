use crate::commands::{Context, Error};
use crate::utils;
use reqwest::Client as HttpClient;
use serenity::async_trait;
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use songbird::input::{Compose, YoutubeDl};
use songbird::Songbird;
use std::sync::Arc;
use std::sync::LazyLock;
use tracing::{debug, error, trace};

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

pub async fn join_vc(ctx: Context<'_>, manager: Arc<Songbird>) {
    trace!("Joining VC...");
    let (guild_id, channel_id) = {
        let guild = ctx.guild_id().unwrap();
        let channel_id = ctx
            .guild()
            .unwrap()
            .voice_states
            .get(&ctx.author().id)
            .and_then(|voice_state| voice_state.channel_id);

        (guild, channel_id)
    };
    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            utils::message::error_message(
                &ctx.serenity_context(),
                &ctx.channel_id(),
                "You are not in a voice channel.".to_string(),
                Some("Error".to_string()),
            )
            .await;
            return;
        }
    };
    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
        // Attach an event handler to see notifications of all track errors.
        let mut handler = handler_lock.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
    }
}

/// baka
#[poise::command(slash_command, prefix_command)]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();
    join_vc(ctx, manager).await;
    Ok(())
}

/// Play a track by url or query
#[poise::command(slash_command, prefix_command)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "The track to play, can be url or query"]
    #[rest]
    query: String,
) -> Result<(), Error> {
    debug!(
        "Received command with query_or_url: {} by {} ({})",
        query,
        ctx.author().name,
        ctx.author().id
    );
    trace!("Getting songbird manager..");
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();
    trace!("Getting VC handler...");
    if manager.get(ctx.guild_id().unwrap()).is_none() {
        join_vc(ctx, manager.clone()).await;
    }
    let handler_lock = manager.get(ctx.guild_id().unwrap()).unwrap();
    let mut handler = handler_lock.lock().await;
    trace!("Cloning HTTP client (inefficient af)...");
    let client = HTTP_CLIENT.clone();
    let search = !query.starts_with("http") || query.contains(" ");
    trace!("Search?: {}", search);
    trace!("Begin searching...");
    let mut src = if search {
        YoutubeDl::new_search(client, query)
    } else {
        YoutubeDl::new(client, query)
    };
    trace!("Got result, playing");
    let _ = handler.play_input(src.clone().into());
    trace!("Done (for now).");
    let metadata = match src.aux_metadata().await {
        Ok(meta) => meta,
        Err(why) => {
            error!("Failed to get metadata: {:?}", why);
            return Ok(());
        }
    };
    utils::message::info_message(
        &ctx.serenity_context(),
        &ctx.channel_id(),
        format!(
            "Playing track: [{}]({})",
            metadata.title.unwrap(),
            metadata.source_url.unwrap()
        ),
        Some("Music".to_string()),
    )
    .await;
    Ok(())
}
