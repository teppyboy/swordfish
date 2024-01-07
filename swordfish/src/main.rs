#![feature(lazy_cell)]
use dotenvy::dotenv;
use serenity::all::{Embed, MessageUpdateEvent};
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, Configuration, StandardFramework};
use serenity::model::channel::Message;
use serenity::prelude::*;
use std::env;
use std::path::Path;
use std::sync::OnceLock;
use swordfish_common::*;
use tokio::time::Instant;

use crate::config::Config;

mod config;
mod debug;
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
        match author.id.get() {
            constants::KATANA_ID => {
                parse_katana_event(&ctx, event).await.unwrap();
            }
            constants::QINGQUE_ID => {
                parse_qingque_event(&ctx, event).await.unwrap();
            }
            _ => {}
        }
    }
}

async fn parse_qingque_event(ctx: &Context, event: MessageUpdateEvent) -> Result<(), String> {
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

async fn parse_katana_event(ctx: &Context, event: MessageUpdateEvent) -> Result<(), String> {
    if event.embeds.is_none() || event.embeds.clone().unwrap().len() == 0 {
        return Ok(());
    }
    let embed = &event.embeds.unwrap()[0];
    parse_katana_embed(embed).await;
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
    } else {
        if msg.embeds.len() == 0 {
            return Ok(());
        }
        let embed = &msg.embeds[0];
        parse_katana_embed(embed).await;
    }
    Ok(())
}

async fn parse_katana_embed(embed: &Embed) {
    match embed.author {
        Some(ref author) => match author.name.as_str() {
            "Card Collection" => {
                let cards = utils::katana::parse_cards_from_katana_kc_ow(
                    &embed.description.as_ref().unwrap(),
                );
                if cards.len() == 0 {
                    return;
                }
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
            _ => {}
        },
        None => {}
    };
    match embed.title {
        Some(ref title) => match title.as_str() {
            "Character Lookup" => {
                let card = match utils::katana::parse_cards_from_katana_klu_lookup(
                    &embed.description.as_ref().unwrap(),
                ) {
                    Some(card) => card,
                    None => {
                        return;
                    }
                };
                trace!("Begin importing a card");
                match database::katana::write_card(card).await {
                    Ok(_) => {
                        trace!("Imported successully");
                    }
                    Err(why) => {
                        error!("Failed to import card: {:?}", why);
                    }
                }
            }
            "Character Results" => {
                let fields = match embed.fields.len() {
                    0 => {
                        return;
                    }
                    _ => &embed.fields,
                };
                let embed_field = fields.get(0).unwrap();
                let cards = utils::katana::parse_cards_from_katana_klu_results(&embed_field.value);
                if cards.len() == 0 {
                    return;
                }
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
            _ => {}
        },
        None => {}
    };
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
    framework.configure(Configuration::new().prefix(CONFIG.get().unwrap().general.prefix.clone()));

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
    if !["debug", "trace"].contains(&config.log.level.as_str()) {
        return Ok(());
    }
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
        "info" => debug::dbg_info(ctx, msg).await?,
        "kdropanalyze" => debug::dbg_kdropanalyze(ctx, msg).await?,
        "kda" => debug::dbg_kdropanalyze(ctx, msg).await?,
        "embed" => debug::dbg_embed(ctx, msg).await?,
        "parse-qingque-atopwl" => debug::dbg_parse_qingque_atopwl(ctx, msg).await?,
        "parse-katana-kc_ow" => debug::dbg_parse_katana_kc_ow(ctx, msg).await?,
        "parse-katana-klu_lookup" => debug::dbg_parse_katana_klu_lookup(ctx, msg).await?,
        "parse-katana-klu_results" => debug::dbg_parse_katana_klu_results(ctx, msg).await?,
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
        Like my work? Consider supporting me at my [Ko-fi](https://ko-fi.com/tretrauit) or [Patreon](https://patreon.com/tretrauit)!",
        env!("CARGO_PKG_VERSION"),
        GITHUB_URL,
        CONFIG.get().unwrap().log.level.clone().as_str(),
        env!("BUILD_PROFILE"),
    );
    helper::info_message(ctx, msg, reply_str, Some("Information".to_string())).await;
    Ok(())
}
