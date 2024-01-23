pub use leptess::{LepTess, Variable};
use std::{
    panic::catch_unwind,
    sync::{Arc, Mutex},
    thread,
};
use swordfish_common::error;
use tokio::task;

static mut TESSERACT_VEC: Vec<Arc<Mutex<LepTess>>> = Vec::new();
static mut TESSERACT_NUMERIC_VEC: Vec<Arc<Mutex<LepTess>>> = Vec::new();

pub unsafe fn get_tesseract() -> Result<Arc<Mutex<LepTess>>, String> {
    let lep_tess: Arc<Mutex<LepTess>>;
    if TESSERACT_VEC.len() == 0 {
        let ocr = match create_tesseract(false) {
            Ok(ocr) => ocr,
            Err(why) => {
                return Err(format!("Failed to create Tesseract: {:?}", why));
            }
        };
        lep_tess = Arc::new(Mutex::new(ocr));
    } else {
        lep_tess = TESSERACT_VEC.pop().unwrap();
    }
    Ok(lep_tess)
}

pub unsafe fn get_tesseract_numeric() -> Arc<Mutex<LepTess>> {
    let lep_tess: Arc<Mutex<LepTess>>;
    if TESSERACT_NUMERIC_VEC.len() == 0 {
        for _ in 0..3 {
            task::spawn(async move {
                let ocr = create_tesseract(true).unwrap();
                TESSERACT_NUMERIC_VEC.push(Arc::new(Mutex::new(ocr)));
            });
        }
        lep_tess = Arc::new(Mutex::new(create_tesseract(true).unwrap()));
    } else {
        lep_tess = TESSERACT_NUMERIC_VEC.pop().unwrap();
        task::spawn(async move {
            let ocr = create_tesseract(true).unwrap();
            TESSERACT_NUMERIC_VEC.push(Arc::new(Mutex::new(ocr)));
        });
    }
    lep_tess
}

pub fn create_tesseract(numeric_only: bool) -> Result<LepTess, String> {
    let mut lep_tess = match LepTess::new(None, "eng") {
        Ok(lep_tess) => lep_tess,
        Err(why) => return Err(format!("Failed to initialize Tesseract: {:?}", why)),
    };
    lep_tess
        .set_variable(Variable::TesseditPagesegMode, "6")
        .unwrap();
    // Use LSTM only.
    lep_tess
        .set_variable(Variable::TesseditOcrEngineMode, "1")
        .unwrap();
    // Set 70 as DPI
    lep_tess
        .set_variable(Variable::UserDefinedDpi, "70")
        .unwrap();
    if numeric_only {
        match lep_tess.set_variable(Variable::TesseditCharWhitelist, "0123456789") {
            Ok(_) => (),
            Err(why) => return Err(format!("Failed to set whitelist: {:?}", why)),
        };
    }
    Ok(lep_tess)
}

///
/// Initialize the Tesseract OCR engine.
///
/// Because this function creates a new thread, it should only be called once.
///
pub async fn init() {
    task::spawn_blocking(|| loop {
        unsafe {
            if TESSERACT_VEC.len() < 9 {
                match catch_unwind(|| {
                    let ocr = match create_tesseract(false) {
                        Ok(ocr) => ocr,
                        Err(why) => {
                            error!("Failed to create Tesseract: {:?}", why);
                            return;
                        }
                    };
                    TESSERACT_VEC.push(Arc::new(Mutex::new(ocr)));
                }) {
                    Ok(_) => (),
                    Err(why) => {
                        error!("Failed to create Tesseract: {:?}", why);
                    }
                }
            }
            if TESSERACT_NUMERIC_VEC.len() < 9 {
                match catch_unwind(|| {
                    let ocr = match create_tesseract(true) {
                        Ok(ocr) => ocr,
                        Err(why) => {
                            error!("Failed to create Tesseract: {:?}", why);
                            return;
                        }
                    };
                    TESSERACT_NUMERIC_VEC.push(Arc::new(Mutex::new(ocr)));
                }) {
                    Ok(_) => (),
                    Err(why) => {
                        error!("Failed to create Tesseract (numeric): {:?}", why);
                    }
                }
            }
        }
        thread::sleep(tokio::time::Duration::from_millis(500));
    });
}
