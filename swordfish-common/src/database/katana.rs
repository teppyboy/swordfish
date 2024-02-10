use crate::database;
use crate::error;
use crate::structs::Character;
use mongodb::bson;
use mongodb::bson::doc;
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
    match KATANA
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
    {
        Ok(character) => character,
        Err(e) => {
            error!("Failed to get character: {}", e);
            None
        }
    }
}

async fn query_characters_regex_internal(
    stage1: bson::Document,
    names: Vec<&String>,
    series: Vec<&String>,
) -> Result<Vec<Option<Character>>, String> {
    let mut pipeline = vec![
        // Stage 1: Optimize query by doing indexed query.
        stage1, // Stage 2: Filter out characters that don't match the series
               // Will be filled later on.
    ];
    let mut characters = doc! {
        "character1": [
            {
                "$match": {
                    "name": {
                        "$regex": names[0],
                        "$options": "i"
                    },
                    "series": {
                        "$regex": series[0],
                        "$options": "i"
                    }
                }
            }
        ],
        "character2": [
            {
                "$match": {
                    "name": {
                        "$regex": names[1],
                        "$options": "i"
                    },
                    "series": {
                        "$regex": series[1],
                        "$options": "i"
                    }
                }
            }
        ],
        "character3": [
            {
                "$match": {
                    "name": {
                        "$regex": names[2],
                        "$options": "i"
                    },
                    "series": {
                        "$regex": series[2],
                        "$options": "i"
                    }
                }
            }
        ]
    };
    if names.len() == 4 {
        characters.insert(
            "character4",
            doc! {
                "$match": {
                    "name": {
                        "$regex": names[3],
                        "$options": "i"
                    },
                    "series": {
                        "$regex": series[3],
                        "$options": "i"
                    }
                }
            },
        );
    }
    let stage2 = doc! {
        "$facet": characters
    };
    pipeline.push(stage2);
    let mut characters: Vec<Option<Character>> = Vec::new();
    let result = KATANA.get().unwrap().aggregate(pipeline, None).await;
    match result {
        Ok(mut cursor) => {
            while cursor.advance().await.unwrap() {
                match cursor.deserialize_current() {
                    Ok(doc) => {
                        characters.push(Some(bson::from_document::<Character>(doc).unwrap()));
                    }
                    Err(e) => {
                        error!("Failed to get document: {}", e);
                        characters.push(None)
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to get cursor: {}", e);
            return Err(format!("Failed to get cursor: {}", e));
        }
    }
    Ok(characters)
}

///
/// Queries the database for characters with the same first letter in the name.
///
pub async fn query_characters_regex_same_name(
    names: Vec<&String>,
    series: Vec<&String>,
) -> Result<Vec<Option<Character>>, String> {
    // Stage 1: Optimize query by querying character names that start with the same letter
    let stage1 = doc! {
        "$match": {
            "name": {
                "$regex": format!("^{}", names[0][0..1].to_string()),
            },
        }
    };
    query_characters_regex_internal(stage1, names, series).await
}

///
/// Queries the database for characters with the same first letter in the series.
///
pub async fn query_characters_regex_same_series(
    names: Vec<&String>,
    series: Vec<&String>,
) -> Result<Vec<Option<Character>>, String> {
    // Stage 1: Optimize query by querying character series that start with the same letter
    let stage1 = doc! {
        "$match": {
            "series": {
                "$regex": format!("^{}", series[0][0..1].to_string()),
            },
        }
    };
    query_characters_regex_internal(stage1, names, series).await
}

///
/// Queries the database for characters with the same first letter in the name and series.
///
pub async fn query_characters_regex_same_name_series(
    names: Vec<&String>,
    series: Vec<&String>,
) -> Result<Vec<Option<Character>>, String> {
    // Stage 1: Optimize query by querying character name and series that start with the same letter
    let stage1 = doc! {
        "$match": {
            "name": {
                "$regex": format!("^{}", names[0][0..1].to_string()),
            },
            "series": {
                "$regex": format!("^{}", series[0][0..1].to_string()),
            },
        }
    };
    query_characters_regex_internal(stage1, names, series).await
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
