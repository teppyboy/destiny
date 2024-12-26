use crate::{
    commands::{Context, Error},
    utils::message::{info_reply, send_reply},
};
use std::time::{SystemTime, UNIX_EPOCH};

/// Shows the latency between the bot and Discord server.
#[poise::command(slash_command, prefix_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let start = SystemTime::now();
    let current_time_ts = start.duration_since(UNIX_EPOCH).unwrap().as_micros() as f64;
    let msg_ts = ctx.created_at().timestamp_micros() as f64;
    send_reply(
        &ctx,
        info_reply(
            Some(ctx.serenity_context()),
            format!(
                "Time taken to receive message: `{}ms`\n\n\
This only reflects the time taken for the bot to receive the message from Discord server.",
                (current_time_ts - msg_ts) / 1000.0 // Message timestamp can't be negative
            ),
            Some("Ping".to_string()),
        )
        .await,
    )
    .await;
    Ok(())
}
