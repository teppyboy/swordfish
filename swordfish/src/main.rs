#![feature(lazy_cell)]
use dotenvy::dotenv;
use serenity::all::MessageUpdateEvent;
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, Configuration, StandardFramework};
use serenity::model::{
    channel::Message,
    id::{ChannelId, MessageId},
};
use serenity::prelude::*;
use std::env;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;
use swordfish_common::*;

use crate::config::Config;

mod config;
mod helper;
mod katana;
mod template;

const GITHUB_URL: &str = "https://github.com/teppyboy/swordfish";
static CONFIG: OnceLock<Config> = OnceLock::new();

#[group]
#[commands(ping, debug, info)]
struct General;
struct Handler;
#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.id == ctx.cache.current_user().id {
            return;
        }
        trace!("Message: {}, sender: {}", msg.content, msg.author.id);
        if msg.author.id.get() == constants::KATANA_ID {
            parse_katana(&ctx, &msg).await.unwrap();
        }
        if msg.content == "pong" {
            info!("Message contains 'pong'");
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pongo!").await {
                println!("Error sending message: {:?}", why);
            }
        }
    }
    async fn message_update(
        &self,
        ctx: Context,
        old_if_available: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        let author = match event.author {
            Some(ref v) => v,
            None => {
                return;
            }
        };
        if author.id == ctx.cache.current_user().id {
            return;
        }
        let content = match event.content {
            Some(ref v) => v,
            None => {
                return;
            }
        };
        trace!("Message update: {}, sender: {}", content, author.id);
        if author.id.get() == constants::QINGQUE_ID {
            parse_qingque(&ctx, event).await.unwrap();
        }
    }
}

async fn parse_qingque(ctx: &Context, event: MessageUpdateEvent) -> Result<(), String> {
    if event.embeds.is_none() || event.embeds.clone().unwrap().len() == 0 {
        return Ok(());
    }
    let embed = &event.embeds.unwrap()[0];
    let embed_title = match embed.title {
        Some(ref title) => title,
        None => {
            return Ok(());
        }
    };
    match embed_title.as_str() {
        "Top Wishlist" => {
            let cards = utils::katana::parse_cards_from_qingque_atopwl(
                &embed.description.as_ref().unwrap(),
            );
            trace!("Begin importing cards");
            match database::katana::write_cards(cards).await {
                Ok(_) => {
                    trace!("Imported successully");
                }
                Err(why) => {
                    error!("Failed to import card: {:?}", why);
                }
            }
        }
        _ => {
            return Ok(());
        }
    }
    Ok(())
}

async fn parse_katana(ctx: &Context, msg: &Message) -> Result<(), String> {
    if msg.content.contains("is dropping 3 cards!")
        || msg
            .content
            .contains("I'm dropping 3 cards since this server is currently active!")
    {
        let config = CONFIG.get().unwrap();
        if !config.features.katana_drop_analysis.enabled {
            return Ok(());
        }
        if helper::message_in_blacklist(msg, &config.features.katana_drop_analysis.blacklist) {
            return Ok(());
        }
        if !helper::message_in_whitelist(msg, &config.features.katana_drop_analysis.whitelist) {
            return Ok(());
        }
        let start = Instant::now();
        match katana::analyze_drop_message(msg).await {
            Ok(cards) => {
                let duration = start.elapsed();
                let mut reply_str = String::new();
                for card in cards {
                    // reply_str.push_str(&format!("{:?}\n", card));
                    let wishlist_str: String = match card.wishlist {
                        Some(wishlist) => {
                            let mut out_str = wishlist.to_string();
                            while out_str.len() < 5 {
                                out_str.push(' ');
                            }
                            out_str
                        }
                        None => "None ".to_string(),
                    };
                    let last_update_ts_str = match card.last_update_ts {
                        0 => "`Never`".to_string(),
                        ts => {
                            format!("<t:{}:R>", ts.to_string())
                        }
                    };
                    reply_str.push_str(
                        format!(
                            ":heart: `{}` • `{}` • **{}** • {} • {}\n",
                            wishlist_str, card.print, card.name, card.series, last_update_ts_str
                        )
                        .as_str(),
                    )
                }
                reply_str.push_str(&format!("Time taken (to analyze): `{:?}`", duration));
                match msg.reply(ctx, reply_str).await {
                    Ok(_) => {}
                    Err(why) => {
                        error!("Failed to reply to message: {:?}", why);
                    }
                };
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
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    match dotenv() {
        Ok(_) => {}
        Err(why) => {
            eprintln!("Failed to load .env: {:?}", why);
        }
    }
    let token = env::var("DISCORD_TOKEN").expect("Token not found");
    let config: Config;
    if Path::new("./config.toml").exists() {
        config = config::Config::load("./config.toml");
    } else {
        config = config::Config::new();
        config.save("./config.toml");
    }
    let level_str = config.log.level.clone();
    let log_level = env::var("LOG_LEVEL").unwrap_or(level_str);
    CONFIG
        .set(config)
        .expect("Failed to register config to static");
    setup_logger(&log_level).expect("Failed to setup logger");
    info!("Swordfish v{} - {}", env!("CARGO_PKG_VERSION"), GITHUB_URL);
    info!("Log level: {}", log_level);
    info!("Initializing database...");
    swordfish_common::database::init().await;
    info!("Initializing Discord client...");
    let framework = StandardFramework::new().group(&GENERAL_GROUP);
    framework.configure(Configuration::new().prefix("~")); // set the bot's prefix to "~"

    // Login with a bot token from the environment
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    info!("Starting client...");
    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        eprintln!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;
    Ok(())
}

#[command]
async fn debug(ctx: &Context, msg: &Message) -> CommandResult {
    let config = CONFIG.get().unwrap();
    if !config.debug.allowed_users.contains(&msg.author.id.get()) {
        return Ok(());
    }
    let mut args = msg.content.split(" ");
    let subcommand = match args.nth(1) {
        Some(content) => content,
        None => {
            helper::error_message(
                ctx,
                msg,
                "Usage: `debug <subcommand> [args...]`".to_string(),
                None,
            )
            .await;
            return Ok(());
        }
    };
    match subcommand {
        "kdropanalyze" => dbg_kdropanalyze(ctx, msg).await?,
        "kda" => dbg_kdropanalyze(ctx, msg).await?,
        "embed" => dbg_embed(ctx, msg).await?,
        "parse-qingque-atopwl" => dbg_parse_qingque_atopwl(ctx, msg).await?,
        _ => {
            helper::error_message(
                ctx,
                msg,
                format!("Unknown subcommand: `{}`", subcommand),
                None,
            )
            .await;
            return Ok(());
        }
    }
    Ok(())
}

#[command]
async fn info(ctx: &Context, msg: &Message) -> CommandResult {
    let reply_str = format!(
        "Swordfish v{} - {}\n\
        Log level: `{}`\n\
        Build type: `{}`\n\n\
        Like my work? Consider supporting me at my [Ko-fi](https://ko-fi.com/tretrauit) or [Patreon](https://patreon.com/tretrauit)!\n\n\
        *Debug information*\n\
        Tesseract backend: `{}`\n\
        ",
        env!("CARGO_PKG_VERSION"),
        GITHUB_URL,
        CONFIG.get().unwrap().log.level.clone().as_str(),
        env!("BUILD_PROFILE"),
        CONFIG.get().unwrap().tesseract.backend.clone().as_str(),
    );
    helper::info_message(ctx, msg, reply_str, Some("Information".to_string())).await;
    Ok(())
}

async fn dbg_get_message(command: &str, ctx: &Context, msg: &Message) -> Result<Message, ()> {
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
                return Ok(());
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
    target_msg
}

async fn dbg_parse_qingque_atopwl(ctx: &Context, msg: &Message) -> CommandResult {
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

async fn dbg_embed(ctx: &Context, msg: &Message) -> CommandResult {
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
    helper::info_message(
        ctx,
        msg,
        format!(
            "Title: \n\
    ```\
    {}\n\
    ```\n\
    Description: \n\
    ```\n\
    {}\n\
    ```",
            embed_title, embed_description
        ),
        Some("Embed information".to_string()),
    )
    .await;
    Ok(())
}

async fn dbg_kdropanalyze(ctx: &Context, msg: &Message) -> CommandResult {
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
                let wishlist_str: String = match card.wishlist {
                    Some(wishlist) => {
                        let mut out_str = wishlist.to_string();
                        while out_str.len() < 5 {
                            out_str.push(' ');
                        }
                        out_str
                    }
                    None => "None ".to_string(),
                };
                let last_update_ts_str = match card.last_update_ts {
                    0 => "`Never`".to_string(),
                    ts => {
                        format!("<t:{}:R>", ts.to_string())
                    }
                };
                reply_str.push_str(
                    format!(
                        ":heart: `{}` • `{}` • **{}** • {} • {}\n",
                        wishlist_str, card.print, card.name, card.series, last_update_ts_str
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
