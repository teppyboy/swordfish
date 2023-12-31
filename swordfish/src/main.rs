use dotenvy::dotenv;
use once_cell::sync::Lazy;
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
use std::sync::{Arc, Mutex};
use swordfish_common::*;

mod config;
mod helper;
mod katana;
mod template;

const GITHUB_URL: &str = "https://github.com/teppyboy/swordfish";
static mut LEPTESS_ARC: Lazy<Arc<Mutex<tesseract::LepTess>>> = Lazy::new(|| {
    trace!("Initializing Tesseract...");
    Arc::new(Mutex::new(
        tesseract::init_tesseract().expect("Failed to initialize Tesseract"),
    ))
});

#[group]
#[commands(ping, kdropanalyze)]
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
}

async fn parse_katana(ctx: &Context, msg: &Message) -> Result<(), String> {
    if msg.content.contains("is dropping 3 cards!")
        || msg
            .content
            .contains("I'm dropping 3 cards since this server is currently active!")
    {
        trace!("Card drop detected, executing drop analyzer...");
        unsafe {
            match katana::analyze_drop_message(&LEPTESS_ARC, msg) {
                Ok(_) => {
                    // msg.reply(ctx, "Drop analysis complete").await?;
                }
                Err(why) => {
                    trace!("Failed to analyze drop: `{:?}`", why);
                    // helper::error_message(ctx, msg, format!("Failed to analyze drop: `{:?}`", why)).await;
                }
            };
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().unwrap();
    let token = env::var("DISCORD_TOKEN").expect("Token not found");
    let config: config::Config;
    if Path::new("./config.toml").exists() {
        config = config::Config::load("./config.toml");
    } else {
        config = config::Config::new();
        config.save("./config.toml");
    }
    let log_level = env::var("LOG_LEVEL").unwrap_or(config.log.level);
    setup_logger(&log_level).expect("Failed to setup logger");
    info!("Swordfish v{} - {}", env!("CARGO_PKG_VERSION"), GITHUB_URL);
    info!("Log level: {}", log_level);
    info!("Loading database...");
    warn!("Databases are not implemented yet");
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
async fn kdropanalyze(ctx: &Context, msg: &Message) -> CommandResult {
    let mut args = msg.content.split(" ");
    let target_channel_id = match args.nth(1) {
        Some(content) => match content.parse::<u64>() {
            Ok(id) => id,
            Err(why) => {
                helper::error_message(ctx, msg, format!("Failed to parse channel ID: `{:?}`", why)).await;
                return Ok(());
            }
        },
        None => {
            helper::error_message(ctx, msg, "Usage: `kdropanalyze <channel ID> <message ID>`".to_string()).await;
            return Ok(());
        }
    };
    let target_msg_id = match args.nth(0) {
        Some(content) => match content.parse::<u64>() {
            Ok(id) => id,
            Err(why) => {
                helper::error_message(ctx, msg, format!("Failed to parse message ID: `{:?}`", why)).await;
                return Ok(());
            }
        },
        None => {
            helper::error_message(ctx, msg, "Usage: `kdropanalyze <channel ID> <message ID>`".to_string()).await;
            return Ok(());
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
            helper::error_message(ctx, msg, format!("Failed to get message: `{:?}`", why)).await;
            return Ok(());
        }
    };
    unsafe {
        match katana::analyze_drop_message(&LEPTESS_ARC, &target_msg).await {
            Ok(_) => {
                msg.reply(ctx, "Drop analysis complete").await?;
            }
            Err(why) => {
                helper::error_message(ctx, msg, format!("Failed to analyze drop: `{:?}`", why)).await;
            }
        };
    }
    Ok(())
}
