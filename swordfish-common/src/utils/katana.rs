use crate::structs::Card;
use log::{error, trace};

// atopwl
pub fn parse_cards_from_qingque_atopwl(content: &String) -> Vec<Card> {
    let mut cards: Vec<Card> = Vec::new();
    for line in content.split("\n") {
        trace!("Parsing line: {}", line);
        let mut line_split = line.split(" · ");
        let wishlist = match line_split.nth(1) {
            Some(wishlist_str) => {
                let mut wl_string = wishlist_str.to_string();
                // Remove `
                wl_string.remove(0);
                // Remove ❤ (Double because heart is 2 bytes)
                wl_string.remove(0);
                wl_string.remove(0);
                // Remove last ``
                wl_string.pop();
                // Remove "," in the number
                wl_string.remove_matches(",");
                // Remove whitespace
                wl_string = wl_string
                    .split_whitespace()
                    .collect::<String>()
                    .trim()
                    .to_string();
                trace!("Formatted wishlist number:{}", wl_string);
                match wl_string.parse::<u32>() {
                    Ok(wishlist) => wishlist,
                    Err(_) => {
                        error!("Failed to parse wishlist number: {}", wishlist_str);
                        continue;
                    }
                }
            }
            None => continue,
        };
        let series = match line_split.next() {
            Some(series) => series.to_string(),
            None => continue,
        };
        let name = match line_split.next() {
            Some(name) => {
                let mut name_string = name.to_string();
                name_string.remove_matches("**");
                name_string
            }
            None => continue,
        };
        let card = Card {
            wishlist: Some(wishlist),
            name,
            series,
            print: 0,
            last_update_ts: 0,
        };
        trace!("Parsed card: {:?}", card);
        cards.push(card);
    }
    cards
}

// kc o:w
pub fn parse_cards_from_katana_kc_ow(content: &String) -> Vec<Card> {
    let mut cards: Vec<Card> = Vec::new();
    for line in content.split("\n") {
        trace!("Parsing line: {}", line);
        if !line.ends_with("**") {
            continue;
        }
        let mut line_split = line.split(" · ");
        let tag_wl_block = line_split.nth(0).unwrap();
        let mut wl_block = match tag_wl_block.split("`").nth(1) {
            Some(wl_block) => {
                // If one does not start with ♡, it is not a wishlist command
                // then we'll just break entirely.
                if !wl_block.starts_with("♡") {
                    break;
                }
                wl_block.to_string()
            },
            None => break,
        };
        wl_block.remove(0);
        wl_block = wl_block.trim().to_string();
        let wishlist = match wl_block.parse::<u32>() {
            Ok(wishlist) => wishlist,
            Err(_) => {
                error!("Failed to parse wishlist number: {}", wl_block);
                continue;
            }
        };
        let series = match line_split.nth(4) {
            Some(series) => series.to_string(),
            None => continue,
        };
        let name = match line_split.next() {
            Some(name) => {
                let mut name_string = name.to_string();
                name_string.remove_matches("**");
                name_string
            }
            None => continue,
        };
        let card = Card {
            wishlist: Some(wishlist),
            name,
            series,
            print: 0,
            last_update_ts: 0,
        };
        trace!("Parsed card: {:?}", card);
        cards.push(card);
    }
    cards
}
