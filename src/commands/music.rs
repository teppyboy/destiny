use crate::commands::{Context, Error};
use crate::utils::message::{error_reply, info_reply};
use reqwest::Client as HttpClient;
use serenity::async_trait;
use songbird::Songbird;
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use songbird::input::{Compose, YoutubeDl};
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

pub async fn join_vc(ctx: Context<'_>, manager: Arc<Songbird>) -> Result<(), String> {
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
            return Err("User not in a voice channel.".to_string());
        }
    };
    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
        // Attach an event handler to see notifications of all track errors.
        let mut handler = handler_lock.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
    }
    Ok(())
}

pub async fn query_track(query: String) -> Result<YoutubeDl, Error> {
    let client = HTTP_CLIENT.clone();
    let search = !query.starts_with("http") || query.contains(" ");
    let src = if search {
        YoutubeDl::new_search(client, query)
    } else {
        YoutubeDl::new(client, query)
    };
    Ok(src)
}

/// baka
#[poise::command(slash_command, prefix_command)]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();
    match join_vc(ctx, manager).await {
        Ok(_) => {}
        Err(why) => {
            error!("Failed to join VC: {:?}", why);
            match ctx
                .send(
                    error_reply(
                        ctx.serenity_context(),
                        format!("Failed to join voice channel: {}", why),
                        Some("Music".to_string()),
                    )
                    .await,
                )
                .await
            {
                Err(why) => {
                    error!("Failed to send message: {:?}", why);
                }
                _ => {}
            }
        }
    };
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
        match join_vc(ctx, manager.clone()).await {
            Ok(_) => {}
            Err(why) => {
                error!("Failed to join VC: {:?}", why);
                match ctx
                    .send(
                        error_reply(
                            ctx.serenity_context(),
                            format!("Failed to join voice channel: {}", why),
                            Some("Music".to_string()),
                        )
                        .await,
                    )
                    .await
                {
                    Err(why) => {
                        error!("Failed to send message: {:?}", why);
                    }
                    _ => {}
                }
                return Ok(());
            }
        }
    }
    ctx.defer().await?;
    let handler_lock = manager.get(ctx.guild_id().unwrap()).unwrap();
    let mut handler = handler_lock.lock().await;
    let mut src = match query_track(query).await {
        Ok(src) => src,
        Err(why) => {
            error!("Failed to get track: {:?}", why);
            match ctx
                .send(
                    error_reply(
                        ctx.serenity_context(),
                        format!("Failed to join fetch track information: {}", why),
                        Some("Music".to_string()),
                    )
                    .await,
                )
                .await
            {
                Err(why) => {
                    error!("Failed to send message: {:?}", why);
                }
                _ => {}
            }
            return Ok(());
        }
    };
    let _ = handler.play_input(src.clone().into());
    trace!("Done (for now).");
    let metadata = match src.aux_metadata().await {
        Ok(meta) => meta,
        Err(why) => {
            error!("Failed to get metadata: {:?}", why);
            return Ok(());
        }
    };
    match ctx
        .send(
            info_reply(
                ctx.serenity_context(),
                format!(
                    "Playing track: [{}]({})",
                    metadata.title.unwrap(),
                    metadata.source_url.unwrap()
                ),
                Some("Music".to_string()),
            )
            .await,
        )
        .await
    {
        Err(why) => {
            error!("Failed to send message: {:?}", why);
        }
        _ => {}
    }
    Ok(())
}
