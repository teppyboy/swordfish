#![feature(lazy_cell)]
use dotenvy::dotenv;
use serenity::all::{Embed, MessageUpdateEvent};
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, Configuration, StandardFramework};
use serenity::gateway::ActivityData;
use serenity::model::channel::Message;
use serenity::prelude::*;
use std::env;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use swordfish_common::*;
use tokio::sync::OnceCell;

use crate::config::Config;
use crate::tesseract::libtesseract;

mod config;
mod debug;
mod helper;
mod katana;
mod template;
mod tesseract;

const GITHUB_URL: &str = "https://github.com/teppyboy/swordfish";
static CONFIG: OnceCell<Config> = OnceCell::const_new();

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
            constants::CALF_ID => {
                parse_calf_event(&ctx, event).await.unwrap();
            }
            _ => {}
        }
    }
}

async fn parse_calf_event(ctx: &Context, event: MessageUpdateEvent) -> Result<(), String> {
    if event.content.is_none() {
        return Ok(());
    }
    let content = event.content.unwrap();
    if content.contains("Apricot v6 Drop Analysis Engine") {
        let cards = utils::katana::parse_cards_from_calf_analysis(&content);
        if cards.len() == 0 {
            return Ok(());
        }
        debug!("Importing cards from Calf Analysis");
        match database::katana::write_characters(cards).await {
            Ok(_) => {
                debug!("Imported successully");
            }
            Err(why) => {
                error!("Failed to import card: {:?}", why);
            }
        }
    }
    Ok(())
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
            debug!("Importing cards from Qingque 'Top Wishlist'");
            match database::katana::write_characters(cards).await {
                Ok(_) => {
                    debug!("Imported successully");
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
    if (msg.content.contains("is dropping") && msg.content.contains("cards!"))
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
        katana::handle_drop_message(ctx, msg).await;
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
                debug!("Importing cards from Katana 'Card Collection'");
                match database::katana::write_characters(cards).await {
                    Ok(_) => {
                        debug!("Imported successully");
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
                debug!("Importing a card from Katana 'Character Lookup'");
                match database::katana::write_character(card).await {
                    Ok(_) => {
                        debug!("Imported successully");
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
                debug!("Importing cards from Katana 'Character Results'");
                match database::katana::write_characters(cards).await {
                    Ok(_) => {
                        debug!("Imported successully");
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

#[tokio::main(flavor = "multi_thread", worker_threads = 32)]
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
    let config = CONFIG.get().unwrap();
    if config.log.file.enabled {
        info!("Logging to file: {}", CONFIG.get().unwrap().log.file.path);
    }
    if config.tesseract.backend == "libtesseract" {
        info!("Using libtesseract as Tesseract backend");
        info!("Initializing libtesseract...");
        libtesseract::init().await;
    } else {
        info!("Using subprocess as Tesseract backend");
    }
    info!("Initializing database...");
    swordfish_common::database::init().await;
    info!("Initializing Discord client...");
    let framework = StandardFramework::new().group(&GENERAL_GROUP);
    framework.configure(Configuration::new().prefix(config.general.prefix.clone()));

    // Login with a bot token from the environment
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .activity(ActivityData::playing("with Seele"))
        .await
        .expect("Error creating client");

    info!("Starting client...");
    // start listening for events by starting a single shard
    if let Err(why) = client.start_autosharded().await {
        eprintln!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let start = SystemTime::now();
    let current_time_ts = start.duration_since(UNIX_EPOCH).unwrap().as_micros() as f64;
    let msg_ts = msg.timestamp.timestamp_micros() as f64;
    helper::info_message(
        ctx,
        msg,
        format!(
            "Time taken to receive message: `{}ms`\n\n\
    This only reflects the time taken for the bot to receive the message from Discord server.",
            (current_time_ts - msg_ts) / 1000.0 // Message timestamp can't be negative
        ),
        Some("Ping".to_string()),
    )
    .await;
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
        "info" => debug::dbg_info(ctx, msg).await?,
        "kdropanalyze" => debug::dbg_kdropanalyze(ctx, msg).await?,
        "kda" => debug::dbg_kdropanalyze(ctx, msg).await?,
        "embed" => debug::dbg_embed(ctx, msg).await?,
        "message" => debug::dbg_message(ctx, msg).await?,
        "regexify-text" => debug::dbg_regexify_text(ctx, msg).await?,
        "regextxt" => debug::dbg_regexify_text(ctx, msg).await?,
        "parse-qingque-atopwl" => debug::dbg_parse_qingque_atopwl(ctx, msg).await?,
        "parse-katana-kc_ow" => debug::dbg_parse_katana_kc_ow(ctx, msg).await?,
        "parse-katana-klu_lookup" => debug::dbg_parse_katana_klu_lookup(ctx, msg).await?,
        "parse-katana-klu_results" => debug::dbg_parse_katana_klu_results(ctx, msg).await?,
        "parse-calf-analysis" => debug::dbg_parse_calf_analysis(ctx, msg).await?,
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
        "Swordfish v{} ({}) - {}\n\
        Log level: `{}`\n\
        Build type: `{}`\n\n\
        Like my work? Consider supporting me at my [Ko-fi](https://ko-fi.com/tretrauit) or [Patreon](https://patreon.com/tretrauit)!",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH"),
        GITHUB_URL,
        CONFIG.get().unwrap().log.level.clone().as_str(),
        env!("BUILD_PROFILE"),
    );
    helper::info_message(ctx, msg, reply_str, Some("Information".to_string())).await;
    Ok(())
}
