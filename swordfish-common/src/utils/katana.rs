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
            }
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

// klu (Character Results)
///
/// Given a string of the results of a katana klu command, parse it into a vector of cards.
///
/// "content" is `fields[0].value`
///
pub fn parse_cards_from_katana_klu_results(content: &String) -> Vec<Card> {
    let mut cards: Vec<Card> = Vec::new();
    for line in content.split("\n") {
        trace!("Parsing line: {}", line);
        if !line.ends_with("**") {
            continue;
        }
        // Split into ['', '1', '. ', '♡448', " · Frieren: Beyond Journey's End · **Frieren**"]
        // But we need 4th and 5th ones.
        let mut line_split = line.split("`");
        let mut wl_str = match line_split.nth(3) {
            Some(wl_str) => wl_str.to_string(),
            None => {
                error!("Failed to parse wishlist number: {}", line);
                continue;
            }
        };
        wl_str.remove(0);
        wl_str = wl_str.trim().to_string();
        let wishlist = match wl_str.parse::<u32>() {
            Ok(wishlist) => wishlist,
            Err(_) => {
                error!("Failed to parse wishlist number: {}", wl_str);
                continue;
            }
        };
        // Split to get character name and series
        let mut char_series_split = match line_split.next() {
            Some(char_series_split) => char_series_split.split(" · "),
            None => {
                error!("Failed to parse character name and series: {}", line);
                continue;
            }
        };
        let series = match char_series_split.nth(1) {
            Some(series) => series.to_string(),
            None => {
                error!("Failed to parse series: {}", line);
                continue;
            }
        };
        let name = match char_series_split.next() {
            Some(name) => {
                let mut name_string = name.to_string();
                name_string.remove_matches("**");
                name_string
            }
            None => {
                error!("Failed to parse character name: {}", line);
                continue;
            }
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

// klu (Character Lookup)
pub fn parse_cards_from_katana_klu_lookup(content: &String) -> Option<Card> {
    let mut lines = content.split("\n");
    // Character
    let mut line_split = lines.nth(0).unwrap().split(" · ");
    let name = match line_split.nth(1) {
        Some(name) => {
            let mut name_string = name.to_string();
            name_string.remove_matches("**");
            name_string
        }
        None => return None,
    };
    // Series
    let mut line_split = lines.nth(0).unwrap().split(" · ");
    let series = match line_split.nth(1) {
        Some(series) => {
            let mut series_string = series.to_string();
            series_string.remove_matches("**");
            series_string
        }
        None => return None,
    };
    // Wishlist
    let mut line_split = lines.nth(1).unwrap().split(" · ");
    let wishlist = match line_split.nth(1) {
        Some(series) => {
            let mut series_string = series.to_string();
            series_string.remove_matches("**");
            match series_string.parse::<u32>() {
                Ok(wishlist) => wishlist,
                Err(_) => {
                    error!("Failed to parse wishlist number: {}", series_string);
                    return None;
                }
            }
        }
        None => return None,
    };
    Some(Card {
        wishlist: Some(wishlist),
        name,
        series,
        print: 0,
        last_update_ts: 0,
    })
}
