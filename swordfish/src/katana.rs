use image::io::Reader as ImageReader;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, Configuration, StandardFramework};
use serenity::model::channel::Message;
use serenity::prelude::*;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::thread;
use swordfish_common::tesseract::LepTess;
use swordfish_common::{debug, error, info, trace, warn};

pub fn analyze_card(leptess: &LepTess, card: image::DynamicImage) {
    trace!("Analyzing card...");
    // Read the name and the series
    let name_img = card.crop_imm(0, 0, 257, 29);
    // Read the print number
}

pub async fn analyze_drop_message(
    leptess_arc: &Arc<Mutex<LepTess>>,
    message: &Message,
) -> Result<(), String> {
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
    let mut img = match ImageReader::new(Cursor::new(image_bytes)).with_guessed_format() {
        Ok(reader) => match reader.decode() {
            Ok(img) => img,
            Err(why) => return Err(format!("Failed to decode image: {:?}", why)),
        },
        Err(why) => return Err(format!("Failed to read image: {:?}", why)),
    };
    trace!("Grayscaling image...");
    img = img.grayscale();
    img.save("debug/1-grayscale.png").unwrap();
    trace!("Increasing contrast of the image...");
    img = img.adjust_contrast(1.0);
    img.save("debug/2-contrast.png").unwrap();
    // Cropping cards
    let distance = 257 - 29 + 305 - 259;
    let cards_count = img.width() / distance;
    trace!("Cropping {} cards...", cards_count);
    let mut jobs: Vec<_> = Vec::new();
    for i_real in 0..cards_count {
        let i = i_real.clone();
        let leptess_mutex = leptess_arc.clone();
        let img = img.clone();
        let job = move || {
            Ok({
                let x = 29 + distance * i;
                let y = 34;
                let width = 257 + distance * i - x;
                let height = 387 - y;
                trace!("Cropping card {} ({}, {}, {}, {})", i, x, y, width, height);
                let card_img = img.crop_imm(x, y, width, height);
                match card_img.save(format!("debug/3-cropped-{}.png", i)) {
                    Ok(_) => {
                        trace!("Saved cropped card {}", i);
                        let leptess = leptess_mutex.lock().unwrap();
                        analyze_card(&leptess, card_img);
                    }
                    Err(why) => return Err(format!("Failed to save image: {:?}", why)),
                };
            })
        };
        jobs.push(job);
    }
    let mut tasks: Vec<thread::JoinHandle<Result<(), String>>> = Vec::new();
    for job in jobs {
        let task = thread::spawn(job);
        tasks.push(task);
    }
    for task in tasks {
        let result = task.join();
        match result {
            Ok(_) => (),
            Err(why) => return Err(format!("Failed to crop card: {:?}", why)),
        };
    }
    let leptess_mutex = leptess_arc.clone();
    let mut leptess = leptess_mutex.lock().unwrap();
    match leptess.set_image_from_mem(&img.as_bytes()) {
        Ok(_) => (),
        Err(why) => return Err(format!("Failed to set image: {:?}", why)),
    };
    Ok(())
}
