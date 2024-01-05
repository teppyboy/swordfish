pub use rusty_tesseract;
pub use rusty_tesseract::{Args, Image};
use std::{collections::HashMap, sync::LazyLock};

static TESSERACT_ARGS: LazyLock<Args> = LazyLock::new(|| Args {
    lang: "eng".to_string(),
    config_variables: HashMap::new(),
    psm: Some(6),
    dpi: None,
    oem: Some(1),
});

static TESSERACT_NUMERIC_ARGS: LazyLock<Args> = LazyLock::new(|| Args {
    lang: "eng".to_string(),
    config_variables: HashMap::from([("tessedit_char_whitelist".into(), "0123456789".into())]),
    psm: Some(6),
    dpi: None,
    oem: Some(1),
});

pub fn image_to_string(image: &Image) -> Result<String, String> {
    match rusty_tesseract::image_to_string(image, &TESSERACT_ARGS) {
        Ok(text) => Ok(text),
        Err(why) => Err(format!("Failed to OCR image: {:?}", why)),
    }
}

pub fn image_to_numeric_string(image: &Image) -> Result<String, String> {
    match rusty_tesseract::image_to_string(image, &TESSERACT_NUMERIC_ARGS) {
        Ok(text) => Ok(text),
        Err(why) => Err(format!("Failed to OCR image: {:?}", why)),
    }
}
