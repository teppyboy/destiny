use serenity::all::ChannelId;
use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage};
use serenity::client::Context;
use serenity::model::Color;
use tracing::error;

pub async fn crate_embed(
    client: &Context,
    title: Option<String>,
    description: Option<String>,
    color: Color,
) -> CreateEmbed {
    let user = client.http.get_current_user().await.unwrap();
    let embed = CreateEmbed::new()
        .title(title.unwrap_or("Destiny".to_string()))
        .description(description.unwrap_or("".to_string()))
        .color(color)
        .footer(
            CreateEmbedFooter::new(user.name.clone())
                .icon_url(user.avatar_url().unwrap_or("".to_string())),
        );
    return embed;
}

pub async fn error_embed(
    client: &Context,
    mut title: Option<String>,
    description: Option<String>,
) -> CreateEmbed {
    if title.is_none() {
        title = Some("Error".to_string());
    }
    return crate_embed(client, title, description, Color::RED).await;
}

pub async fn info_embed(
    client: &Context,
    mut title: Option<String>,
    description: Option<String>,
) -> CreateEmbed {
    if title.is_none() {
        title = Some("Info".to_string());
    }
    return crate_embed(client, title, description, Color::DARK_GREEN).await;
}

pub async fn error_message(
    ctx: &Context,
    channel_id: &ChannelId,
    content: String,
    title: Option<String>,
) {
    match channel_id
        .send_message(
            ctx,
            CreateMessage::new().add_embed(error_embed(ctx, title, Some(content)).await),
        )
        .await
    {
        Ok(_) => (),
        Err(why) => {
            error!("Failed to send error message: {:?}", why);
        }
    };
}

pub async fn info_message(
    ctx: &Context,
    channel_id: &ChannelId,
    content: String,
    title: Option<String>,
) {
    match channel_id
        .send_message(
            ctx,
            CreateMessage::new().add_embed(info_embed(ctx, title, Some(content)).await),
        )
        .await
    {
        Ok(_) => (),
        Err(why) => {
            error!("Failed to send error message: {:?}", why);
        }
    };
}
