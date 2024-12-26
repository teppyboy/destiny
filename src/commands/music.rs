use crate::commands::{Context, Error};
use crate::utils::message::{error_reply, info_message, info_reply, send_message, send_reply};
use reqwest::Client as HttpClient;
use serenity::all::{ChannelId, Http, Mentionable};
use serenity::async_trait;
use serenity::prelude::TypeMapKey;
use songbird::Songbird;
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use songbird::input::{AuxMetadata, Compose, YoutubeDl};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;
use tracing::{debug, error, trace};

static TRACK_METADATA: LazyLock<Mutex<HashMap<Uuid, AuxMetadata>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}

struct TrackStartNotifier {
    channel_id: ChannelId,
    metadata: AuxMetadata,
    http: Arc<Http>,
}

#[async_trait]
impl VoiceEventHandler for TrackStartNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        send_message(
            &self.http,
            &self.channel_id,
            info_message(
                None,
                format!(
                    "Playing track: [{}]({})",
                    self.metadata.title.as_ref().unwrap(),
                    self.metadata.source_url.as_ref().unwrap()
                ),
                Some("Music".to_string()),
            )
            .await,
        )
        .await;
        None
    }
}

struct TrackEndNotifier;

#[async_trait]
impl VoiceEventHandler for TrackEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_info) = ctx {
            // Remove the metadata from the map (since the track has ended)
            let mut metadatas = TRACK_METADATA.lock().await;
            for (_, handle) in track_info.iter() {
                metadatas.remove(&handle.uuid());
            }
        }
        None
    }
}

async fn in_vc(ctx: &Context<'_>, manager: &Arc<Songbird>) -> bool {
    let handler_lock = manager.get(ctx.guild_id().unwrap());
    return handler_lock.is_some()
        && handler_lock
            .unwrap()
            .lock()
            .await
            .current_channel()
            .is_some()
}

async fn join_vc(ctx: Context<'_>, manager: Arc<Songbird>) -> Result<ChannelId, String> {
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
    if let Ok(_) = manager.join(guild_id, connect_to).await {
        return Ok(channel_id.unwrap());
    }
    return Err("Failed to join voice channel.".to_string());
}

async fn notify_if_not_vc(ctx: &Context<'_>, manager: &Arc<Songbird>) -> bool {
    if !in_vc(&ctx, &manager).await {
        send_reply(
            &ctx,
            error_reply(
                Some(ctx.serenity_context()),
                "Not in a voice channel.".to_string(),
                Some("Music".to_string()),
            )
            .await,
        )
        .await;
        return true;
    }
    false
}

async fn get_http_client(ctx: &Context<'_>) -> HttpClient {
    let data = ctx.serenity_context().data.read().await;
    data.get::<HttpKey>()
        .cloned()
        .expect("Guaranteed to exist in the typemap.")
}

async fn query_track(ctx: &Context<'_>, query: String) -> Result<YoutubeDl, Error> {
    let client = get_http_client(ctx).await;
    let search = !query.starts_with("http") || query.contains(" ");
    let src = if search {
        YoutubeDl::new_search(client, query)
    } else {
        YoutubeDl::new(client, query)
    };
    Ok(src)
}

/// Joins the voice channel of the user
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();
    match join_vc(ctx, manager).await {
        Ok(channel_id) => {
            send_reply(
                &ctx,
                info_reply(
                    Some(ctx.serenity_context()),
                    format!("Joined {}", channel_id.mention()),
                    Some("Music".to_string()),
                )
                .await,
            )
            .await;
        }
        Err(why) => {
            error!("Failed to join VC: {:?}", why);
            send_reply(
                &ctx,
                error_reply(
                    Some(ctx.serenity_context()),
                    format!("Failed to join voice channel: {}", why),
                    Some("Music".to_string()),
                )
                .await,
            )
            .await;
        }
    };
    Ok(())
}

/// Shows the current queue
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn queue(ctx: Context<'_>) -> Result<(), Error> {
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();
    if notify_if_not_vc(&ctx, &manager).await {
        return Ok(());
    }
    let handler_lock = manager.get(ctx.guild_id().unwrap()).unwrap();
    let handler = handler_lock.lock().await;
    let queue = handler.queue();
    let mut queue_str = format!("## Queue \n");
    if queue.len() == 0 {
        queue_str.push_str("Empty, add a track by executing `/play` command :)");
    } else {
        let metdatas = TRACK_METADATA.lock().await;
        for (index, song) in queue.current_queue().into_iter().enumerate() {
            // Safe to unwrap because we are sure that the metadata exists
            let metadata = metdatas.get(&song.uuid()).unwrap();
            queue_str.push_str(&format!(
                "{}. [{}]({}){}\n",
                index + 1,
                metadata.title.as_ref().unwrap(),
                metadata.source_url.as_ref().unwrap(),
                if index == 0 { " (Now Playing)" } else { "" }
            ));
        }
    }
    send_reply(
        &ctx,
        info_reply(
            Some(ctx.serenity_context()),
            queue_str,
            Some("Music".to_string()),
        )
        .await,
    )
    .await;
    Ok(())
}

/// Play a track by url or query
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "The track to play, can be url or query"]
    #[rest]
    query: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    debug!(
        "Received command with query_or_url: {} by {} ({})",
        query,
        ctx.author().name,
        ctx.author().id
    );
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();

    {
        match join_vc(ctx, manager.clone()).await {
            Ok(_) => {}
            Err(why) => {
                error!("Failed to join VC: {:?}", why);
                send_reply(
                    &ctx,
                    error_reply(
                        Some(ctx.serenity_context()),
                        format!("Failed to join voice channel: {}", why),
                        Some("Music".to_string()),
                    )
                    .await,
                )
                .await;
                return Ok(());
            }
        }
    }
    let handler_lock = manager.get(ctx.guild_id().unwrap()).unwrap();
    let mut handler = handler_lock.lock().await;
    let mut src = match query_track(&ctx, query).await {
        Ok(src) => src,
        Err(why) => {
            error!("Failed to get track: {:?}", why);
            send_reply(
                &ctx,
                error_reply(
                    Some(ctx.serenity_context()),
                    format!("Failed to get track: {}", why),
                    Some("Music".to_string()),
                )
                .await,
            )
            .await;
            return Ok(());
        }
    };
    let metadata = match src.aux_metadata().await {
        Ok(meta) => meta,
        Err(why) => {
            send_reply(
                &ctx,
                error_reply(
                    Some(ctx.serenity_context()),
                    format!("Failed to get metadata: {}", why),
                    Some("Music".to_string()),
                )
                .await,
            )
            .await;
            return Ok(());
        }
    };
    let song = handler.enqueue_input(src.clone().into()).await;
    let send_http = ctx.serenity_context().http.clone();
    let channel_id = ctx.channel_id();
    let _ = song.add_event(Event::Track(TrackEvent::Play), TrackStartNotifier {
        channel_id,
        metadata: metadata.clone(),
        http: send_http,
    });
    let _ = song.add_event(Event::Track(TrackEvent::End), TrackEndNotifier);
    let mut metadatas = TRACK_METADATA.lock().await;
    metadatas.insert(song.uuid(), metadata.clone());
    trace!("Added song to queue.");
    if handler.queue().len() == 1 {
        return Ok(());
    }
    send_reply(
        &ctx,
        info_reply(
            Some(ctx.serenity_context()),
            format!(
                "Added track to queue: [{}]({})",
                metadata.title.as_ref().unwrap(),
                metadata.source_url.as_ref().unwrap()
            ),
            Some("Music".to_string()),
        )
        .await,
    )
    .await;
    Ok(())
}

/// Skips the current track
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();
    if notify_if_not_vc(&ctx, &manager).await {
        return Ok(());
    }
    let handler_lock = manager.get(ctx.guild_id().unwrap()).unwrap();
    let handler = handler_lock.lock().await;
    if handler.queue().len() == 0 {
        send_reply(
            &ctx,
            error_reply(
                Some(ctx.serenity_context()),
                "Not playing anything to skip.".to_string(),
                Some("Music".to_string()),
            )
            .await,
        )
        .await;
        return Ok(());
    }
    match handler.queue().skip() {
        Ok(_) => {
            send_reply(
                &ctx,
                info_reply(
                    Some(ctx.serenity_context()),
                    "Skipped the current track.".to_string(),
                    Some("Music".to_string()),
                )
                .await,
            )
            .await;
        }
        Err(why) => {
            error!("Failed to skip track: {:?}", why);
            send_reply(
                &ctx,
                error_reply(
                    Some(ctx.serenity_context()),
                    format!("Failed to skip track: {}", why),
                    Some("Music".to_string()),
                )
                .await,
            )
            .await;
        }
    };
    Ok(())
}

/// Stops the music player and disconnect the voice channel
#[poise::command(slash_command, prefix_command, guild_only, aliases("leave"))]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();
    if notify_if_not_vc(&ctx, &manager).await {
        return Ok(());
    }
    let handler_lock = manager.get(ctx.guild_id().unwrap()).unwrap();
    let mut handler = handler_lock.lock().await;
    handler.queue().stop();
    handler.remove_all_global_events();
    handler.leave().await.unwrap();
    match ctx
        .send(
            info_reply(
                Some(ctx.serenity_context()),
                "Stopped the music player and left the voice channel.".to_string(),
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

pub fn exports() -> Vec<
    poise::Command<
        crate::commands::Data,
        Box<(dyn serde::ser::StdError + std::marker::Send + Sync + 'static)>,
    >,
> {
    vec![join(), play(), queue(), skip(), stop()]
}
