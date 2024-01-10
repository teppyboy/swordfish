use crate::database;
use crate::structs::Character;
use mongodb::Collection;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::OnceCell;
use tokio::task;
use tracing::trace;

pub static KATANA: OnceCell<Collection<Character>> = OnceCell::const_new();

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
                .collection::<Character>("katana"),
        )
        .unwrap();
}

pub async fn query_character(name: &String, series: &String) -> Option<Character> {
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

pub async fn query_character_regex(name: &String, series: &String) -> Option<Character> {
    KATANA
        .get()
        .unwrap()
        .find_one(
            mongodb::bson::doc! {
                "name": {"$regex": name, "$options" : "i"},
                "series": {"$regex": series, "$options" : "i"}
            },
            None,
        )
        .await
        .unwrap()
}

pub async fn write_character(mut card: Character) -> Result<(), String> {
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

pub async fn write_characters(cards: Vec<Character>) -> Result<(), String> {
    let mut new_cards: Vec<Character> = Vec::new();
    let mut handles: Vec<task::JoinHandle<Result<Option<Character>, String>>> = Vec::new();
    let start = SystemTime::now();
    let current_time_ts = start.duration_since(UNIX_EPOCH).unwrap();
    for mut card in cards {
        let current_time_ts_clone = current_time_ts.clone();
        trace!("Writing card: {:?}", card);
        handles.push(task::spawn(async move {
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
            card.last_update_ts = current_time_ts_clone.as_secs() as i64;
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
