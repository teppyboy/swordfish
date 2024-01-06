use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Card {
    pub wishlist: Option<i32>,
    pub name: String,
    pub series: String,
    pub print: i32,
}
