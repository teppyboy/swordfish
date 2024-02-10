use crate::helper;
use crate::tesseract::utils::{fix_tesseract_string, regexify_text};
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

const CARD_NAME_X_OFFSET: u32 = 22;
const CARD_NAME_Y_OFFSET: u32 = 28;
const CARD_NAME_WIDTH: u32 = 202 - CARD_NAME_X_OFFSET;
const CARD_NAME_HEIGHT: u32 = 70 - CARD_NAME_Y_OFFSET;
const CARD_SERIES_X_OFFSET: u32 = 22;
const CARD_SERIES_Y_OFFSET: u32 = 278;
const CARD_SERIES_WIDTH: u32 = 206 - CARD_SERIES_X_OFFSET;
const CARD_SERIES_HEIGHT: u32 = 328 - CARD_SERIES_Y_OFFSET;

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
