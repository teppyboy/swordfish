use image::imageops::colorops::contrast_in_place;
use image::io::Reader as ImageReader;
use image::ImageFormat;
use once_cell::sync::Lazy;
use regex::Regex;
use serenity::model::channel::Message;
use std::io::Cursor;
use std::{env, thread};
use swordfish_common::structs::Card;
use swordfish_common::tesseract;
use swordfish_common::{trace, warn};

static TEXT_NUM_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"[A-Za-z0-9]").unwrap());
static ALLOWED_CHARS_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"['-: ]").unwrap());

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
        for _ in 0..2 {
            text.pop();
        }
    }
    // Replace first (to prevent "byte index 13 is not a char boundary; it is inside '—' (bytes 11..14)")
    while replace_string(text, "—", "-") {
        trace!("Replacing '—' with '-'");
    }
    // Workaround for a bug the text
    trace!("Text: {}", text);
    if text.starts_with("- ") || text.starts_with("-.") {
        text.remove(0);
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
    // Workaround for "\n." (and others in the future)
    for (i, c) in text.clone().chars().enumerate() {
        if c != '\n' {
            continue;
        }
        let prev_char = match text.chars().nth(i - 1) {
            Some(c) => c,
            None => continue,
        };
        let next_char = match text.chars().nth(i + 1) {
            Some(c) => c,
            None => break,
        };
        let mut rm_prev: bool = false;
        trace!("Prev char: {}", prev_char);
        if ['-'].contains(&prev_char) {
            rm_prev = true;
            text.remove(i - 1);
        }
        trace!("Next char: {}", next_char);
        if ['.'].contains(&next_char) {
            if rm_prev {
                text.remove(i);
            } else {
                text.remove(i + 1);
            }
        }
    }
    // Replace "\n" with " "
    trace!("Text: {}", text);
    replace_string(text, "\n", " ");
    // Remove all non-alphanumeric characters
    trace!("Text: {}", text);
    text.retain(|c| {
        TEXT_NUM_REGEX.is_match(&c.to_string()) || ALLOWED_CHARS_REGEX.is_match(&c.to_string())
    });
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
    // Replace multiple spaces with one space
    trace!("Text: {}", text);
    while replace_string(text, "  ", " ") {
        trace!("Removing multiple spaces");
    }
    // Workaround if the first character is a space
    trace!("Text: {}", text);
    while text.starts_with(" ") {
        trace!("Removing leading space");
        text.remove(0);
    }
    trace!("Text (final): {}", text);
}

fn save_image_if_trace(img: &image::DynamicImage, path: &str) {
    let log_lvl = match env::var("LOG_LEVEL") {
        Ok(log_lvl) => log_lvl,
        Err(_) => return,
    };
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

pub fn analyze_card(card: image::DynamicImage, count: u32) -> Card {
    trace!("Spawning threads for analyzing card...");
    // Read the name and the series
    let card_clone = card.clone();
    let name_thread = thread::spawn(move || {
        let mut leptess = tesseract::init_tesseract(false).expect("Failed to initialize Tesseract");
        // let binding = tesseract::init_tesseract_quick(false);
        // let mut leptess = binding.lock().unwrap();
        let name_img = card_clone.crop_imm(22, 26, 204 - 22, 70 - 26);
        let mut buffer: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        match name_img.write_to(&mut buffer, ImageFormat::Png) {
            Ok(_) => {}
            Err(why) => {
                panic!("{}", format!("Failed to write image: {:?}", why));
            }
        };
        save_image_if_trace(&name_img, format!("debug/4-{}-name.png", count).as_str());
        leptess.set_image_from_mem(&buffer.get_mut()).unwrap();
        let mut name_str = leptess.get_utf8_text().expect("Failed to read name");
        fix_tesseract_string(&mut name_str);
        name_str
    });
    let card_clone = card.clone();
    let series_thread = thread::spawn(move || {
        let mut leptess = tesseract::init_tesseract(false).expect("Failed to initialize Tesseract");
        // let binding = tesseract::init_tesseract_quick(false);
        // let mut leptess = binding.lock().unwrap();
        let series_img = card_clone.crop_imm(22, 276, 204 - 22, 330 - 276);
        let mut buffer: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        match series_img.write_to(&mut buffer, ImageFormat::Png) {
            Ok(_) => {}
            Err(why) => {
                panic!("{}", format!("Failed to write image: {:?}", why));
            }
        };
        save_image_if_trace(
            &series_img,
            format!("debug/4-{}-series.png", count).as_str(),
        );
        leptess.set_image_from_mem(&buffer.get_mut()).unwrap();
        let mut series_str = leptess.get_utf8_text().expect("Failed to read name");
        fix_tesseract_string(&mut series_str);
        series_str
    });
    let name = name_thread.join().unwrap();
    trace!("Name: {}", name);
    let series = series_thread.join().unwrap();
    trace!("Series: {}", series);
    // TODO: Read the print number
    // TODO: Read the wishlist number (from our database)
    return Card {
        wishlist: None,
        name,
        series,
        print: 0,
    };
}

pub async fn analyze_drop_message(message: &Message) -> Result<Vec<Card>, String> {
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
    contrast_in_place(&mut img, 127.0);
    save_image_if_trace(&img, "debug/2-contrast.png");
    // Cropping cards
    let distance = 257 - 29 + 305 - 259;
    let cards_count = img.width() / distance;
    trace!("Cropping {} cards...", cards_count);
    let mut jobs: Vec<_> = Vec::new();
    let mut cards: Vec<Card> = Vec::with_capacity(cards_count.try_into().unwrap());
    for index in 0..cards_count {
        let i = index.clone();
        let x = 29 + distance * i;
        let y = 34;
        let width = 257 + distance * i - x;
        let height = 387 - y;
        trace!("Cropping card {} ({}, {}, {}, {})", i, x, y, width, height);
        let card_img = img.crop_imm(x, y, width, height);
        save_image_if_trace(&card_img, &format!("debug/3-cropped-{}.png", i));
        let job = move || {
            trace!("Analyzing card {}", i);
            Ok((i, analyze_card(card_img, i)))
        };
        jobs.push(job);
    }
    let mut tasks: Vec<thread::JoinHandle<Result<(u32, Card), String>>> = Vec::new();
    for job in jobs {
        let task = thread::spawn(job);
        tasks.push(task);
    }
    for task in tasks {
        let result = task.join();
        match result {
            Ok(result) => {
                match result {
                    Ok((i, card)) => {
                        trace!("Finished analyzing card {}", i);
                        cards.push(card);
                    }
                    Err(why) => return Err(format!("Failed to analyze card: {}", why)),
                };
            }
            Err(why) => return Err(format!("Failed to analyze card: {:?}", why)),
        };
    }
    Ok(cards)
}
