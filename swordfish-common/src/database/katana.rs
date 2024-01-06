use crate::database;
use crate::structs::Card;
use mongodb::Collection;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

static KATANA: OnceLock<Collection<Card>> = OnceLock::new();

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

pub async fn write_card(mut card: Card) {
    // todo!("Write card to database");
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
        KATANA
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
            .unwrap();
    } else {
        KATANA.get().unwrap().insert_one(card, None).await.unwrap();
    }
}
