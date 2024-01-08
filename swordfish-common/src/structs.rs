use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Character {
    pub wishlist: Option<u32>,
    pub name: String,
    pub series: String,
    pub last_update_ts: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DroppedCard {
    pub character: Character,
    pub print: i32,
    pub edition: i32,
}
