use std::env;

use serenity::all::{Embed, MessageUpdateEvent};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;
use swordfish_common::setup_logger;
use swordfish_common::{constants, database, utils};
use swordfish_common::{debug, tokio};
use swordfish_common::{error, info, trace};

const GITHUB_URL: &str = "https://github.com/teppyboy/swordfish";

async fn parse_katana(ctx: &Context, msg: &Message) -> Result<(), String> {
    if msg.embeds.len() == 0 {
        return Ok(());
    }
    let embed = &msg.embeds[0];
    parse_katana_embed(embed).await;
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
                match database::katana::write_cards(cards).await {
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
                match database::katana::write_card(card).await {
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
                match database::katana::write_cards(cards).await {
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
            match database::katana::write_cards(cards).await {
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

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
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
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() {
    // Login with a user token from the environment
    let log_level = env::var("LOG_LEVEL").unwrap_or("info".to_string());
    setup_logger(log_level.as_str()).expect("Failed to setup logger");
    let token = env::var("DISCORD_TOKEN").expect("Token not found");
    info!("Swordfish v{} - {}", env!("CARGO_PKG_VERSION"), GITHUB_URL);
    info!("Log level: {}", log_level);
    info!("Initializing database...");
    swordfish_common::database::init().await;
    info!("Initializing Discord client...");
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        error!("An error occurred while running the client: {:?}", why);
    }
}
