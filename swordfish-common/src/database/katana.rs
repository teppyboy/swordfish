use mongodb::Collection;

use crate::database;
use crate::structs::Card;
use std::sync::OnceLock;

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

fn query_card() {
    todo!("Query card from database");
    println!("{:?}", card);
}
