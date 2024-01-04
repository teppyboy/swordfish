use crate::template::message;
use serenity::builder::CreateMessage;
use serenity::client::Context;
use serenity::model::channel::Message;
use swordfish_common::error;

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
