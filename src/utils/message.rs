use poise::CreateReply;
use serenity::all::{CacheHttp, ChannelId, CreateMessage};
use serenity::builder::{CreateEmbed, CreateEmbedFooter};
use serenity::client::Context;
use serenity::model::Color;
use tracing::error;

pub async fn create_embed(
    client: Option<&Context>,
    title: Option<String>,
    description: Option<String>,
    color: Color,
) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title(title.unwrap_or("Destiny".to_string()))
        .description(description.unwrap_or("".to_string()))
        .color(color);
    if client.is_some() {
        let user = client.unwrap().http.get_current_user().await.unwrap();
        embed = embed.footer(
            CreateEmbedFooter::new(user.name.clone())
                .icon_url(user.avatar_url().unwrap_or("".to_string())),
        )
    }
    return embed;
}

pub async fn error_embed(
    client: Option<&Context>,
    mut title: Option<String>,
    description: Option<String>,
) -> CreateEmbed {
    if title.is_none() {
        title = Some("Error".to_string());
    }
    return create_embed(client, title, description, Color::RED).await;
}

pub async fn info_embed(
    client: Option<&Context>,
    mut title: Option<String>,
    description: Option<String>,
) -> CreateEmbed {
    if title.is_none() {
        title = Some("Info".to_string());
    }
    return create_embed(client, title, description, Color::DARK_GREEN).await;
}

pub async fn send_message(http: impl CacheHttp, channel_id: &ChannelId, message: CreateMessage) {
    match channel_id.send_message(http, message).await {
        Ok(_) => (),
        Err(why) => {
            error!("Failed to send error message: {:?}", why);
        }
    };
}

pub async fn info_message(
    client: Option<&Context>,
    content: String,
    title: Option<String>,
) -> CreateMessage {
    CreateMessage::new().add_embed(info_embed(client, title, Some(content)).await)
}

pub async fn error_reply(
    client: Option<&Context>,
    content: String,
    title: Option<String>,
) -> CreateReply {
    CreateReply::default()
        .embed(error_embed(client, title, Some(content)).await)
        .reply(true)
}

pub async fn info_reply(
    client: Option<&Context>,
    content: String,
    title: Option<String>,
) -> CreateReply {
    CreateReply::default()
        .embed(info_embed(client, title, Some(content)).await)
        .reply(true)
}

pub async fn send_reply(ctx: &crate::commands::Context<'_>, reply: CreateReply) {
    match ctx.send(reply).await {
        Ok(_) => (),
        Err(why) => {
            error!("Failed to send reply: {:?}", why);
        }
    };
}
