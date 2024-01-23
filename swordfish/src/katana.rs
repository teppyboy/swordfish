use crate::helper;
use crate::tesseract::{libtesseract, subprocess};
use crate::CONFIG;
use image::imageops::colorops::contrast_in_place;
use image::io::Reader as ImageReader;
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer, ImageFormat, Rgba};
use serenity::all::Context;
use serenity::model::channel::Message;
use std::io::Cursor;
use swordfish_common::database::katana as db;
use swordfish_common::structs::{Character, DroppedCard};
use swordfish_common::{error, trace, warn};
use tokio::task;
use tokio::time::Instant;

const ALLOWED_CHARS: [char; 14] = [
    ' ', '-', '.', '!', ':', '(', ')', '\'', '/', '\'', '@', '&', '_', 'é',
];
const REGEX_CHARS: [char; 4] = ['[', ']', ')', '('];
const CARD_NAME_X_OFFSET: u32 = 22;
const CARD_NAME_Y_OFFSET: u32 = 28;
const CARD_NAME_WIDTH: u32 = 202 - CARD_NAME_X_OFFSET;
const CARD_NAME_HEIGHT: u32 = 70 - CARD_NAME_Y_OFFSET;
const CARD_SERIES_X_OFFSET: u32 = 22;
const CARD_SERIES_Y_OFFSET: u32 = 278;
const CARD_SERIES_WIDTH: u32 = 206 - CARD_SERIES_X_OFFSET;
const CARD_SERIES_HEIGHT: u32 = 330 - CARD_SERIES_Y_OFFSET;

fn replace_string(text: &mut String, from: &str, to: &str) -> bool {
    match text.find(from) {
        Some(i) => {
            text.replace_range(i..i + from.len(), to);
            true
        }
        None => false,
    }
}

fn fix_tesseract_string(text: &mut String) {
    // Remove the \n
    trace!("Text: {}", text);
    if text.ends_with("\n") {
        text.pop();
    }
    // Workaround for a bug the text
    // e.g. "We Never Learn\nN" -> "We Never Learn"
    trace!("Text: {}", text);
    if text.ends_with("\nN") {
        text.truncate(text.len() - 2);
    }
    // Replace first (to prevent "byte index 13 is not a char boundary; it is inside '—' (bytes 11..14)")
    while replace_string(text, "—", "-") {
        trace!("Replacing '—' with '-'");
    }
    // Workaround for a bug the text
    trace!("Text: {}", text);
    if text.starts_with("- ") || text.starts_with("-.") {
        text.drain(0..2);
    }
    // Remove the first character if it is not alphanumeric
    if !text.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        text.remove(0);
    }
    // Workaround IR -> Ik
    // Maybe it only occurs if Ik is in the start of the string?
    // e.g. "IReda" -> "Ikeda"
    trace!("Text: {}", text);
    replace_string(text, "IR", "Ik");
    // Workaround for "A\n"
    // This is usually the corner of the card
    trace!("Text: {}", text);
    replace_string(text, "A\n", "");
    // Workaround for '“NO'
    // This is usually the left bottom corner of the card
    trace!("Text: {}", text);
    if text.ends_with(r##"“NO"##) {
        text.drain(text.len() - 4..text.len());
    }
    // Workaround for "\n." (and others in the future)
    let text_clone = text.clone();
    let mut clone_chars = text_clone.chars();
    for (i, c) in clone_chars.clone().enumerate() {
        if c != '\n' {
            continue;
        }
        let prev_char = match clone_chars.nth(i - 1) {
            Some(c) => c,
            None => continue,
        };
        let mut rm_prev: i8 = 0;
        trace!("Prev char: {}", prev_char);
        if ['-'].contains(&prev_char) {
            rm_prev = 1;
            text.remove(i - 1);
        }
        // Fix for "Asobi ni Iku lo Asobi ni Oide" -> "Asobi ni Iku yo! Asobi ni Oide"
        if prev_char == 'l' {
            let prev_prev_char = match clone_chars.nth(i - 2) {
                Some(c) => c,
                None => continue,
            };
            trace!("Prev prev char: {}", prev_prev_char);
            if prev_prev_char == 'o' {
                rm_prev = -1;
                text.drain(i - 3..i - 1);
                text.insert_str(i - 2, "yo!")
            }
        }
        let next_char = match clone_chars.nth(i + 1) {
            Some(c) => c,
            None => break,
        };
        trace!("Next char: {}", next_char);
        if ['.'].contains(&next_char) {
            text.remove((i as i8 + 1 - rm_prev) as usize);
        }
    }
    // Replace "\n" with " "
    trace!("Text: {}", text);
    while replace_string(text, "\n", " ") {
        trace!("Replacing '\\n' with ' '");
    }
    // Remove all non-alphanumeric characters
    trace!("Text: {}", text);
    text.retain(|c| ALLOWED_CHARS.contains(&c) || c.is_ascii_alphanumeric());
    // Fix "mn" -> "III"
    trace!("Text: {}", text);
    if text.ends_with("mn") {
        text.pop();
        text.pop();
        text.push_str("III");
    }
    // Fix "1ll" -> "III"
    trace!("Text: {}", text);
    replace_string(text, "1ll", "III");
    // Fix "lll" -> "!!!"
    trace!("Text: {}", text);
    replace_string(text, "lll", "!!!");
    // Fix "Il" -> "II" in the end of the string
    trace!("Text: {}", text);
    if text.ends_with("Il") {
        text.pop();
        text.pop();
        text.push_str("II");
    }
    // Replace multiple spaces with one space
    trace!("Text: {}", text);
    while replace_string(text, "  ", " ") {
        trace!("Removing multiple spaces");
    }
    // Remove the last character if it is a dash
    if text.ends_with("-") {
        text.pop();
    }
    // Workaround if the first character is a space
    trace!("Text: {}", text);
    while text.starts_with(|c: char| c.is_whitespace()) {
        trace!("Removing leading space");
        text.remove(0);
    }
    // Workaround if the last character is a space
    trace!("Text: {}", text);
    while text.ends_with(|c: char| c.is_whitespace()) {
        trace!("Removing ending space");
        text.pop();
    }
    trace!("Text (final): {}", text);
}

fn regexify_text(text: &String) -> String {
    let partial_match: bool;
    let short_text = text.len() < 6;
    if text.len() > 23 {
        partial_match = true;
    } else {
        partial_match = false;
    }
    let mut regex = String::new();
    let mut ascii_text = String::new();
    let mut prev_chars: Vec<char> = Vec::new();
    for c in text.chars() {
        // Here comes the workaround...
        // The character "0" is sometimes used in place of "O" in names
        if ['0', 'O'].contains(&c) {
            ascii_text.push_str("[0O]");
        } else if ['u', 'v'].contains(&c) {
            ascii_text.push_str("[uv]");
        } else if ['t'].contains(&c) {
            ascii_text.push_str("[ti]");
        } else if ['I', 'l', '!', '1'].contains(&c) {
            ascii_text.push_str("[Il!1i]");
        } else if ['.'].contains(&c) {
            if prev_chars.len() > 3 {
                let prev_char = prev_chars[prev_chars.len() - 1];
                let prev_prev_char = prev_chars[prev_chars.len() - 2];
                if prev_char.is_numeric() && prev_prev_char.is_whitespace() {
                    continue;
                }
            }
            ascii_text.push(' ');
        } else if ['R'].contains(&c) {
            ascii_text.push_str("[Rk]");
        } else if ['m'].contains(&c) {
            ascii_text.push_str("(m|ra)");
        } else if ['a'].contains(&c) {
            ascii_text.push_str("[ao]")
        } else if c.is_ascii_alphanumeric() {
            ascii_text.push(c);
        } else {
            ascii_text.push(' ');
        }
        prev_chars.push(c);
    }
    // Filter for short string.
    if short_text && !ascii_text.contains(|c: char| c.is_whitespace()) {
        regex.push_str("^");
        let mut request_quantifier: bool = false;
        let mut regex_any: bool = false;
        let mut regex_any_from: usize = 0;
        for (i, char) in ascii_text.chars().enumerate() {
            trace!("Char: {}", char);
            if char == '[' {
                regex_any = true;
                regex_any_from = i;
                if i == 0 {
                    request_quantifier = true;
                }
                continue;
            } else if i == ascii_text.len() - 1 {
                regex.push_str(".*");
            }
            if regex_any {
                if char == ']' {
                    regex_any = false;
                    regex.push_str(&ascii_text[regex_any_from..i + 1]);
                    if request_quantifier {
                        regex.push_str(".*");
                    }
                }
                continue;
            }
            regex.push(char);
            if i == 0 {
                regex.push_str(".*");
            }
        }
        regex.push_str("$");
        trace!("Regex (short string): {}", regex);
        return regex;
    }
    let split = ascii_text.split_whitespace();
    let len = split.clone().count();
    trace!("Partial match: {}", partial_match);
    for (i, word) in split.enumerate() {
        if word.len() < 2 {
            if i > 0 && i < len - 1 {
                continue;
            }
            if ["x", "X"].contains(&word) {
                continue;
            }
        }
        regex.push_str("(?=.*");
        let processed_word = word.to_lowercase();
        trace!("Processed word: {}", processed_word);
        if partial_match && processed_word.len() > 4 {
            // Remove first two and last two characters for "partial match"
            if !processed_word[0..3].contains(|c: char| REGEX_CHARS.contains(&c))
                && !processed_word[word.len() - 2..word.len()]
                    .contains(|c: char| REGEX_CHARS.contains(&c))
            {
                regex.push_str(&processed_word[2..word.len() - 2]);
            } else {
                regex.push_str(&processed_word.as_str());
            }
        } else {
            // Do not push word boundary if the word contains special characters like "!"
            if processed_word.contains(|c: char| c.is_ascii_alphanumeric()) {
                regex.push_str(format!("\\b{}\\b", &processed_word.as_str()).as_str());
            } else {
                regex.push_str(format!("{}", &processed_word.as_str()).as_str());
            }
        }
        regex.push_str(")");
    }
    regex.push_str(".+");
    trace!("Regex: {}", regex);
    regex
}

fn save_image_if_trace(img: &DynamicImage, path: &str) {
    let log_lvl = CONFIG.get().unwrap().log.level.as_str();
    if log_lvl == "trace" {
        match img.save(path) {
            Ok(_) => {
                trace!("Saved image to {}", path);
            }
            Err(why) => {
                warn!("{}", format!("Failed to save image: {:?}", why))
            }
        };
    }
}

fn image_with_white_padding(im: DynamicImage) -> DynamicImage {
    // Partially copied from https://github.com/PureSci/nori/blob/main/rust-workers/src/drop.rs#L102C1-L121C6
    let mut new_im: DynamicImage =
        ImageBuffer::<Rgba<u8>, Vec<u8>>::new(im.width() + 14, im.height() + 14).into();
    let white = Rgba([255, 255, 255, 255]);
    for y in 0..im.height() {
        for x in 0..im.width() {
            let p = im.get_pixel(x, y);
            new_im.put_pixel(x + 7, y + 7, p.to_owned());
        }
    }
    for y in 0..7 {
        for x in 0..im.width() + 14 {
            new_im.put_pixel(x, y, white);
            new_im.put_pixel(x, y + im.height() + 7, white);
        }
    }
    for x in 0..7 {
        for y in 7..im.height() + 7 {
            new_im.put_pixel(x, y, white);
            new_im.put_pixel(x + im.width() + 7, y, white);
        }
    }
    new_im
}

pub async fn analyze_card_libtesseract(
    card: image::DynamicImage,
    count: u32,
) -> Result<DroppedCard, String> {
    trace!("Spawning threads for analyzing card...");
    // Read the name and the series
    let card_clone = card.clone();
    let name_thread = task::spawn_blocking(move || {
        // let mut leptess =
        //     libtesseract::init_tesseract(false).expect("Failed to initialize Tesseract");
        let binding = unsafe {
            match libtesseract::get_tesseract() {
                Ok(b) => b,
                Err(why) => {
                    panic!("{}", format!("Failed to get Tesseract: {:?}", why));
                }
            }
        };
        let mut leptess = binding.lock().unwrap();
        let name_img = image_with_white_padding(card_clone.crop_imm(
            CARD_NAME_X_OFFSET,
            CARD_NAME_Y_OFFSET,
            CARD_NAME_WIDTH,
            CARD_NAME_HEIGHT,
        ));
        let mut buffer: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        match name_img.write_to(&mut buffer, ImageFormat::Png) {
            Ok(_) => {}
            Err(why) => {
                panic!("{}", format!("Failed to write image: {:?}", why));
            }
        };
        save_image_if_trace(
            &name_img,
            format!("debug/4-libtesseract-{}-name.png", count).as_str(),
        );
        leptess.set_image_from_mem(&buffer.get_mut()).unwrap();
        let mut name_str = leptess.get_utf8_text().expect("Failed to read name");
        fix_tesseract_string(&mut name_str);
        name_str
    });
    let card_clone = card.clone();
    let series_thread = task::spawn_blocking(move || {
        // let mut leptess =
        //     libtesseract::init_tesseract(false).expect("Failed to initialize Tesseract");
        let binding = unsafe {
            match libtesseract::get_tesseract() {
                Ok(b) => b,
                Err(why) => {
                    panic!("{}", format!("Failed to get Tesseract: {:?}", why));
                }
            }
        };
        let mut leptess = binding.lock().unwrap();
        let series_img = image_with_white_padding(card_clone.crop_imm(
            CARD_SERIES_X_OFFSET,
            CARD_SERIES_Y_OFFSET,
            CARD_SERIES_WIDTH,
            CARD_SERIES_HEIGHT,
        ));
        let mut buffer: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        match series_img.write_to(&mut buffer, ImageFormat::Png) {
            Ok(_) => {}
            Err(why) => {
                panic!("{}", format!("Failed to write image: {:?}", why));
            }
        };
        save_image_if_trace(
            &series_img,
            format!("debug/4-libtesseract-{}-series.png", count).as_str(),
        );
        leptess.set_image_from_mem(&buffer.get_mut()).unwrap();
        let mut series_str = leptess.get_utf8_text().expect("Failed to read series");
        fix_tesseract_string(&mut series_str);
        series_str
    });
    let name = match name_thread.await {
        Ok(name) => name,
        Err(why) => {
            return Err(format!("Failed to read name: {:?}", why));
        }
    };
    trace!("Name: {}", name);
    let series = match series_thread.await {
        Ok(series) => series,
        Err(why) => {
            return Err(format!("Failed to read series: {:?}", why));
        }
    };
    trace!("Series: {}", series);
    // TODO: Read the print number
    let mut character = Character {
        wishlist: None,
        name,
        series,
        last_update_ts: 0,
    };
    // Read the wishlist number
    match db::query_character(&character.name, &character.series).await {
        Some(c) => {
            character = c;
        }
        None => match db::query_character_regex(
            &regexify_text(&character.name),
            &regexify_text(&character.series),
        )
        .await
        {
            Some(c) => {
                character = c;
            }
            None => {}
        },
    }
    Ok(DroppedCard {
        character,
        print: 0,
        edition: 0,
    })
}

pub async fn analyze_card_subprocess(
    card: image::DynamicImage,
    count: u32,
) -> Result<DroppedCard, String> {
    trace!("Spawning threads for analyzing card...");
    // Read the name and the series
    let card_clone = card.clone();
    let name_thread = task::spawn_blocking(move || {
        let name_img = image_with_white_padding(card_clone.crop_imm(
            CARD_NAME_X_OFFSET,
            CARD_NAME_Y_OFFSET,
            CARD_NAME_WIDTH,
            CARD_NAME_HEIGHT,
        ));
        let img = subprocess::Image::from_dynamic_image(&name_img).unwrap();
        save_image_if_trace(
            &name_img,
            format!("debug/4-subprocess-{}-name.png", count).as_str(),
        );
        let mut name_str = subprocess::image_to_string(&img).unwrap();
        fix_tesseract_string(&mut name_str);
        name_str
    });
    let card_clone = card.clone();
    let series_thread = task::spawn_blocking(move || {
        let series_img = image_with_white_padding(card_clone.crop_imm(
            CARD_SERIES_X_OFFSET,
            CARD_SERIES_Y_OFFSET,
            CARD_SERIES_WIDTH,
            CARD_SERIES_HEIGHT,
        ));
        let img = subprocess::Image::from_dynamic_image(&series_img).unwrap();
        save_image_if_trace(
            &series_img,
            format!("debug/4-subprocess-{}-series.png", count).as_str(),
        );
        let mut series_str = subprocess::image_to_string(&img).unwrap();
        fix_tesseract_string(&mut series_str);
        series_str
    });
    let name = match name_thread.await {
        Ok(name) => name,
        Err(why) => {
            return Err(format!("Failed to read name: {:?}", why));
        }
    };
    trace!("Name: {}", name);
    let series = match series_thread.await {
        Ok(series) => series,
        Err(why) => {
            return Err(format!("Failed to read series: {:?}", why));
        }
    };
    // TODO: Read the print number
    let mut character = Character {
        wishlist: None,
        name,
        series,
        last_update_ts: 0,
    };
    // Read the wishlist number
    match db::query_character(&character.name, &character.series).await {
        Some(c) => {
            character = c;
        }
        None => match db::query_character_regex(
            &regexify_text(&character.name),
            &regexify_text(&character.series),
        )
        .await
        {
            Some(c) => {
                character = c;
            }
            None => {}
        },
    }
    Ok(DroppedCard {
        character,
        print: 0,
        edition: 0,
    })
}

async fn execute_analyze_drop(image: DynamicImage, count: u32) -> Result<DroppedCard, String> {
    let config = CONFIG.get().unwrap();
    match config.tesseract.backend.as_str() {
        "libtesseract" => analyze_card_libtesseract(image, count).await,
        "subprocess" => analyze_card_subprocess(image, count).await,
        _ => {
            panic!("Invalid Tesseract backend: {}", config.tesseract.backend);
        }
    }
}

pub async fn analyze_drop_message(message: &Message) -> Result<Vec<DroppedCard>, String> {
    if message.attachments.len() < 1 {
        return Err("No attachments found".to_string());
    };
    // Get the image attachment
    let attachment = &message.attachments[0];
    let image_bytes = match attachment.download().await {
        Ok(bytes) => bytes,
        Err(why) => return Err(format!("Failed to download attachment: {:?}", why)),
    };
    // Pre-process the image
    let mut img =
        match ImageReader::with_format(Cursor::new(image_bytes), ImageFormat::Png).decode() {
            Ok(img) => img,
            Err(why) => return Err(format!("Failed to decode image: {:?}", why)),
        };
    trace!("Grayscaling image...");
    img = img.grayscale();
    save_image_if_trace(&img, "debug/1-grayscale.png");
    trace!("Increasing contrast of the image...");
    contrast_in_place(&mut img, 127.0 / 4.0);
    save_image_if_trace(&img, "debug/2-contrast.png");
    // Cropping cards
    let distance = 257 - 29 + 305 - 259;
    let cards_count = img.width() / distance;
    trace!("Cropping {} cards...", cards_count);
    let mut jobs: Vec<_> = Vec::new();
    let mut cards: Vec<DroppedCard> = Vec::with_capacity(cards_count.try_into().unwrap());
    for index in 0..cards_count {
        let i = index.clone();
        let x = 29 + distance * i;
        let y = 34;
        let width = 257 + distance * i - x;
        let height = 387 - y;
        trace!("Cropping card {} ({}, {}, {}, {})", i, x, y, width, height);
        let card_img = img.crop_imm(x, y, width, height);
        save_image_if_trace(&card_img, &format!("debug/3-cropped-{}.png", i));
        jobs.push(async move {
            trace!("Analyzing card {}", i);
            (i, execute_analyze_drop(card_img, i).await)
        });
    }
    let mut handles: Vec<task::JoinHandle<(u32, Result<DroppedCard, String>)>> = Vec::new();
    for job in jobs {
        let handle = task::spawn(job);
        handles.push(handle);
    }
    for handle in handles {
        let result = handle.await;
        match result {
            Ok((i, card_result)) => {
                let card = match card_result {
                    Ok(card) => card,
                    Err(why) => return Err(format!("Failed to analyze card: {}", why)),
                };
                trace!("Finished analyzing card {}", i);
                cards.push(card);
            }
            Err(why) => return Err(format!("Failed to analyze card: {:?}", why)),
        };
    }
    Ok(cards)
}

pub async fn handle_drop_message(ctx: &Context, msg: &Message) {
    let start = Instant::now();
    match analyze_drop_message(msg).await {
        Ok(cards) => {
            let duration = start.elapsed();
            let mut reply_str = String::new();
            for card in cards {
                // reply_str.push_str(&format!("{:?}\n", card));
                let wishlist_str: String = match card.character.wishlist {
                    Some(wishlist) => {
                        let mut out_str = wishlist.to_string();
                        while out_str.len() < 5 {
                            out_str.push(' ');
                        }
                        out_str
                    }
                    None => "None ".to_string(),
                };
                let last_update_ts_str = match card.character.last_update_ts {
                    0 => "`Never`".to_string(),
                    ts => {
                        format!("<t:{}:R>", ts.to_string())
                    }
                };
                reply_str.push_str(
                    format!(
                        ":heart: `{}` • `{}` • **{}** • {} • {}\n",
                        wishlist_str,
                        card.print,
                        card.character.name,
                        card.character.series,
                        last_update_ts_str
                    )
                    .as_str(),
                )
            }
            reply_str.push_str(&format!("Time taken (to analyze): `{:?}`", duration));
            match msg.reply(ctx, reply_str).await {
                Ok(_) => {}
                Err(why) => {
                    error!("Failed to reply to message: {:?}", why);
                }
            };
        }
        Err(why) => {
            helper::error_message(
                ctx,
                msg,
                format!("Failed to analyze drop: `{:?}`", why),
                None,
            )
            .await;
        }
    };
}
