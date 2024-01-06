use crate::config::List;
use crate::template::message;
use serenity::builder::CreateMessage;
use serenity::client::Context;
use serenity::model::channel::Message;
use swordfish_common::error;

pub fn message_in_blacklist(msg: &Message, blacklist: &List) -> bool {
    if !blacklist.enabled {
        return false;
    }
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => return false,
    };
    if blacklist.servers.contains(&guild_id.get()) {
        return true;
    }
    if blacklist.channels.contains(&msg.channel_id.get()) {
        return true;
    }
    return false;
}

pub fn message_in_whitelist(msg: &Message, whitelist: &List) -> bool {
    if !whitelist.enabled {
        return true;
    }
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => return false,
    };
    if whitelist.servers.contains(&guild_id.get()) {
        return true;
    }
    if whitelist.channels.contains(&msg.channel_id.get()) {
        return true;
    }
    return false;
}

pub async fn error_message(ctx: &Context, msg: &Message, content: String, title: Option<String>) {
    match msg
        .channel_id
        .send_message(
            ctx,
            CreateMessage::new().add_embed(message::error_embed(ctx, title, Some(content)).await),
        )
        .await
    {
        Ok(_) => (),
        Err(why) => {
            error!("Failed to send error message: {:?}", why);
        }
    };
}

pub async fn info_message(ctx: &Context, msg: &Message, content: String, title: Option<String>) {
    match msg
        .channel_id
        .send_message(
            ctx,
            CreateMessage::new().add_embed(message::info_embed(ctx, title, Some(content)).await),
        )
        .await
    {
        Ok(_) => (),
        Err(why) => {
            error!("Failed to send error message: {:?}", why);
        }
    };
}
