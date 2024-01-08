use crate::helper;
use crate::katana;
use crate::utils;
use crate::CONFIG;
use serenity::framework::standard::CommandResult;
use serenity::model::{
    channel::Message,
    id::{ChannelId, MessageId},
};
use serenity::prelude::*;
use tokio::time::Instant;

pub async fn dbg_get_message(command: &str, ctx: &Context, msg: &Message) -> Result<Message, ()> {
    let mut args = msg.content.split(" ");
    let target_channel_id = match args.nth(2) {
        Some(content) => match content.parse::<u64>() {
            Ok(id) => id,
            Err(why) => {
                helper::error_message(
                    ctx,
                    msg,
                    format!("Failed to parse channel ID: `{:?}`", why),
                    None,
                )
                .await;
                return Err(());
            }
        },
        None => {
            helper::error_message(
                ctx,
                msg,
                format!("Usage: `{} <channel ID> <message ID>`", command),
                None,
            )
            .await;
            return Err(());
        }
    };
    let target_msg_id = match args.nth(0) {
        Some(content) => match content.parse::<u64>() {
            Ok(id) => id,
            Err(why) => {
                helper::error_message(
                    ctx,
                    msg,
                    format!("Failed to parse message ID: `{:?}`", why),
                    None,
                )
                .await;
                return Err(());
            }
        },
        None => {
            helper::error_message(
                ctx,
                msg,
                format!("Usage: `{} <channel ID> <message ID>`", command),
                None,
            )
            .await;
            return Err(());
        }
    };
    let target_msg = match ctx
        .http()
        .get_message(
            ChannelId::new(target_channel_id),
            MessageId::new(target_msg_id),
        )
        .await
    {
        Ok(msg) => msg,
        Err(why) => {
            helper::error_message(
                ctx,
                msg,
                format!("Failed to get message: `{:?}`", why),
                None,
            )
            .await;
            return Err(());
        }
    };
    Ok(target_msg)
}

pub async fn dbg_parse_qingque_atopwl(ctx: &Context, msg: &Message) -> CommandResult {
    let target_msg = match dbg_get_message("embed", ctx, msg).await {
        Ok(msg) => msg,
        Err(_) => {
            return Ok(());
        }
    };
    if target_msg.embeds.len() == 0 {
        helper::error_message(
            ctx,
            msg,
            "Message does not contain any embeds".to_string(),
            None,
        )
        .await;
        return Ok(());
    }
    let embed = &target_msg.embeds[0];
    let embed_description = match embed.description {
        Some(ref description) => description,
        None => {
            helper::error_message(
                ctx,
                msg,
                "Embed does not contain a description".to_string(),
                None,
            )
            .await;
            return Ok(());
        }
    };
    let cards = utils::katana::parse_cards_from_qingque_atopwl(embed_description);
    helper::info_message(
        ctx,
        msg,
        format!("Parsed cards: ```\n{:?}\n```", cards),
        None,
    )
    .await;
    Ok(())
}

pub async fn dbg_parse_katana_kc_ow(ctx: &Context, msg: &Message) -> CommandResult {
    let target_msg = match dbg_get_message("embed", ctx, msg).await {
        Ok(msg) => msg,
        Err(_) => {
            return Ok(());
        }
    };
    if target_msg.embeds.len() == 0 {
        helper::error_message(
            ctx,
            msg,
            "Message does not contain any embeds".to_string(),
            None,
        )
        .await;
        return Ok(());
    }
    let embed = &target_msg.embeds[0];
    let embed_description = match embed.description {
        Some(ref description) => description,
        None => {
            helper::error_message(
                ctx,
                msg,
                "Embed does not contain a description".to_string(),
                None,
            )
            .await;
            return Ok(());
        }
    };
    let cards = utils::katana::parse_cards_from_katana_kc_ow(embed_description);
    helper::info_message(
        ctx,
        msg,
        format!("Parsed cards: ```\n{:?}\n```", cards),
        None,
    )
    .await;
    Ok(())
}

pub async fn dbg_parse_katana_klu_results(ctx: &Context, msg: &Message) -> CommandResult {
    let target_msg = match dbg_get_message("embed", ctx, msg).await {
        Ok(msg) => msg,
        Err(_) => {
            return Ok(());
        }
    };
    if target_msg.embeds.len() == 0 {
        helper::error_message(
            ctx,
            msg,
            "Message does not contain any embeds".to_string(),
            None,
        )
        .await;
        return Ok(());
    }
    let embed = &target_msg.embeds[0];
    let fields = match embed.fields.len() {
        0 => {
            helper::error_message(
                ctx,
                msg,
                "Embed does not contain any fields".to_string(),
                None,
            )
            .await;
            return Ok(());
        }
        _ => &embed.fields,
    };
    let embed_field = fields.get(0).unwrap();
    let cards = utils::katana::parse_cards_from_katana_klu_results(&embed_field.value);
    helper::info_message(
        ctx,
        msg,
        format!("Parsed cards: ```\n{:?}\n```", cards),
        None,
    )
    .await;
    Ok(())
}

pub async fn dbg_parse_katana_klu_lookup(ctx: &Context, msg: &Message) -> CommandResult {
    let target_msg = match dbg_get_message("embed", ctx, msg).await {
        Ok(msg) => msg,
        Err(_) => {
            return Ok(());
        }
    };
    if target_msg.embeds.len() == 0 {
        helper::error_message(
            ctx,
            msg,
            "Message does not contain any embeds".to_string(),
            None,
        )
        .await;
        return Ok(());
    }
    let embed = &target_msg.embeds[0];
    let embed_description = match embed.description {
        Some(ref description) => description,
        None => {
            helper::error_message(
                ctx,
                msg,
                "Embed does not contain a description".to_string(),
                None,
            )
            .await;
            return Ok(());
        }
    };
    let card = match utils::katana::parse_cards_from_katana_klu_lookup(embed_description) {
        Some(card) => card,
        None => {
            helper::error_message(ctx, msg, "Failed to parse card".to_string(), None).await;
            return Ok(());
        }
    };
    helper::info_message(ctx, msg, format!("Parsed card: ```\n{:?}\n```", card), None).await;
    Ok(())
}

pub async fn dbg_embed(ctx: &Context, msg: &Message) -> CommandResult {
    let target_msg = match dbg_get_message("embed", ctx, msg).await {
        Ok(msg) => msg,
        Err(_) => {
            return Ok(());
        }
    };
    if target_msg.embeds.len() == 0 {
        helper::error_message(
            ctx,
            msg,
            "Message does not contain any embeds".to_string(),
            None,
        )
        .await;
        return Ok(());
    }
    let embed = &target_msg.embeds[0];
    let embed_title = match embed.title {
        Some(ref title) => title,
        None => "None",
    };
    let embed_description = match embed.description {
        Some(ref description) => description,
        None => "None",
    };
    let embed_footer_text = match embed.footer {
        Some(ref footer) => footer.text.as_str(),
        None => "None",
    };
    let fields_text = match embed.fields.len() {
        0 => "```\nNone\n```".to_string(),
        _ => {
            let mut fields_text = String::new();
            for field in &embed.fields {
                fields_text.push_str(&format!(
                    "Name: `{}`\nValue: ```\n{}\n```\nInline: `{}`\n",
                    field.name, field.value, field.inline
                ));
            }
            fields_text
        }
    };
    helper::info_message(
        ctx,
        msg,
        format!(
            "Title: \n\
    ```\n\
    {}\n\
    ```\n\
    Description: \n\
    ```\n\
    {}\n\
    ```\n\
    Fields: \n\
    {}\n\
    Footer Text: \n\
    ```\n\
    {}\n\
    ```",
            embed_title, embed_description, fields_text, embed_footer_text
        ),
        Some("Embed information".to_string()),
    )
    .await;
    Ok(())
}

pub async fn dbg_kdropanalyze(ctx: &Context, msg: &Message) -> CommandResult {
    let target_msg = match dbg_get_message("embed", ctx, msg).await {
        Ok(msg) => msg,
        Err(_) => {
            return Ok(());
        }
    };
    let start = Instant::now();
    match katana::analyze_drop_message(&target_msg).await {
        Ok(cards) => {
            let duration = start.elapsed();
            let mut reply_str = String::new();
            for card in cards {
                // reply_str.push_str(&format!("{:?}\n", card));
                let wishlist_str: String = match card.character.wishlist {
                    Some(wishlist) => {
                        let mut out_str = wishlist.to_string();
                        while out_str.len() < 5 {
                            out_str.push(' ');
                        }
                        out_str
                    }
                    None => "None ".to_string(),
                };
                let last_update_ts_str = match card.character.last_update_ts {
                    0 => "`Never`".to_string(),
                    ts => {
                        format!("<t:{}:R>", ts.to_string())
                    }
                };
                reply_str.push_str(
                    format!(
                        ":heart: `{}` • `{}` • **{}** • {} • {}\n",
                        wishlist_str,
                        card.print,
                        card.character.name,
                        card.character.series,
                        last_update_ts_str
                    )
                    .as_str(),
                )
            }
            reply_str.push_str(&format!("Time taken (to analyze): `{:?}`", duration));
            msg.reply(ctx, reply_str).await?;
        }
        Err(why) => {
            helper::error_message(
                ctx,
                msg,
                format!("Failed to analyze drop: `{:?}`", why),
                None,
            )
            .await;
        }
    };
    Ok(())
}

pub async fn dbg_info(ctx: &Context, msg: &Message) -> CommandResult {
    let reply_str = format!(
        "Tesseract backend: {}",
        CONFIG.get().unwrap().tesseract.backend,
    );
    helper::info_message(ctx, msg, reply_str, Some("Debug Information".to_string())).await;
    Ok(())
}
