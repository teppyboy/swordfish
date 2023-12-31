use serenity::client::Context;
use serenity::model::channel::Message;
use serenity::builder::CreateMessage;
use crate::template::message;

pub async fn error_message(ctx: &Context, msg: &Message, content: String) {
    msg.channel_id
    .send_message(
        ctx,
        CreateMessage::new().add_embed(
            message::error_embed(
                ctx,
                None,
                Some(content),
            )
            .await,
        ),
    )
    .await?;
}