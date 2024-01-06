use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Card {
    pub wishlist: Option<u32>,
    pub name: String,
    pub series: String,
    pub print: i32,
    pub last_update_ts: i64,
}
