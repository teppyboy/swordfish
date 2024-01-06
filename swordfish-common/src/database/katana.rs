use crate::database;
use crate::structs::Card;
use mongodb::Collection;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::task;
use tracing::trace;

pub static KATANA: OnceLock<Collection<Card>> = OnceLock::new();

///
/// Initialize the "katana" collection in MongoDB
///
/// This method is called automatically when you initialize the
/// database module.
///
pub fn init() {
    KATANA
        .set(
            database::MONGO_DATABASE
                .get()
                .unwrap()
                .collection::<Card>("katana"),
        )
        .unwrap();
}

pub async fn query_card(name: &str, series: &str) -> Option<Card> {
    // todo!("Query card from database");
    KATANA
        .get()
        .unwrap()
        .find_one(
            mongodb::bson::doc! {
                "name": name,
                "series": series
            },
            None,
        )
        .await
        .unwrap()
}

pub async fn write_card(mut card: Card) -> Result<(), String> {
    let old_card = KATANA
        .get()
        .unwrap()
        .find_one(
            mongodb::bson::doc! {
                "name": card.name.clone(),
                "series": card.series.clone()
            },
            None,
        )
        .await
        .unwrap();
    let start = SystemTime::now();
    let current_time_ts = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    card.last_update_ts = current_time_ts.as_secs() as i64;
    if old_card.is_some() {
        match KATANA
            .get()
            .unwrap()
            .replace_one(
                mongodb::bson::doc! {
                    "name": card.name.clone(),
                    "series": card.series.clone()
                },
                card,
                None,
            )
            .await
        {
            Ok(_) => {
                return Ok(());
            }
            Err(e) => {
                return Err(format!("Failed to update card: {}", e));
            }
        }
    } else {
        match KATANA.get().unwrap().insert_one(card, None).await {
            Ok(_) => {
                return Ok(());
            }
            Err(e) => {
                return Err(format!("Failed to insert card: {}", e));
            }
        }
    }
}

pub async fn write_cards(cards: Vec<Card>) -> Result<(), String> {
    let mut new_cards: Vec<Card> = Vec::new();
    let mut handles: Vec<task::JoinHandle<Result<Option<Card>, String>>> = Vec::new();
    for mut card in cards {
        trace!("Writing card: {:?}", card);
        handles.push(task::spawn(async {
            let old_card = KATANA
                .get()
                .unwrap()
                .find_one(
                    mongodb::bson::doc! {
                        "name": card.name.clone(),
                        "series": card.series.clone()
                    },
                    None,
                )
                .await
                .unwrap();
            let start = SystemTime::now();
            let current_time_ts = start
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            card.last_update_ts = current_time_ts.as_secs() as i64;
            if old_card.is_some() {
                match KATANA
                    .get()
                    .unwrap()
                    .replace_one(
                        mongodb::bson::doc! {
                            "name": card.name.clone(),
                            "series": card.series.clone()
                        },
                        card,
                        None,
                    )
                    .await
                {
                    Ok(_) => {
                        return Ok(None);
                    }
                    Err(e) => {
                        return Err(format!("Failed to update card: {}", e));
                    }
                }
            } else {
                return Ok(Some(card));
            };
        }));
    }
    for handle in handles {
        match handle.await.unwrap() {
            Ok(card) => {
                if card.is_some() {
                    new_cards.push(card.unwrap());
                }
            }
            Err(e) => {
                return Err(format!("Failed to update card: {}", e));
            }
        }
    }
    if new_cards.len() > 0 {
        match KATANA.get().unwrap().insert_many(new_cards, None).await {
            Ok(_) => {
                return Ok(());
            }
            Err(e) => {
                return Err(format!("Failed to insert card: {}", e));
            }
        }
    }
    Ok(())
}
