use serenity::builder::{CreateEmbed, CreateEmbedFooter};
use serenity::client::Context;
use serenity::model::Color;

pub async fn crate_embed(
    client: &Context,
    title: Option<String>,
    description: Option<String>,
    color: Color,
) -> CreateEmbed {
    let user = client.http.get_current_user().await.unwrap();
    let embed = CreateEmbed::new()
        .title(title.unwrap_or("Swordfish".to_string()))
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
