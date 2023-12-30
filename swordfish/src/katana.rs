use image::io::Reader as ImageReader;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, Configuration, StandardFramework};
use serenity::model::channel::Message;
use serenity::prelude::*;
use std::io::Cursor;
use swordfish_common::tesseract::LepTess;
use swordfish_common::{debug, error, info, trace, warn};

pub async fn analyze_drop_message(message: &Message) -> Result<(), String> {
    if message.attachments.len() < 1 {
        return Err("No attachments found".to_string());
    };
    trace!("Initializing Tesseract OCR engine...");
    let mut lep_tess = LepTess::new(None, "eng").unwrap();
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
    img = img.grayscale();
    img.save("debug.png").unwrap();
    match lep_tess.set_image_from_mem(&img.as_bytes()) {
        Ok(_) => (),
        Err(why) => return Err(format!("Failed to set image: {:?}", why)),
    };
    Ok(())
}
