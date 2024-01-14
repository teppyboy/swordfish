pub use leptess::{LepTess, Variable};
use std::{
    sync::{Arc, Mutex},
    thread,
};
use tokio::task;

static mut TESSERACT_VEC: Vec<Arc<Mutex<LepTess>>> = Vec::new();
static mut TESSERACT_NUMERIC_VEC: Vec<Arc<Mutex<LepTess>>> = Vec::new();

pub unsafe fn get_tesseract() -> Arc<Mutex<LepTess>> {
    let lep_tess: Arc<Mutex<LepTess>>;
    if TESSERACT_VEC.len() == 0 {
        lep_tess = Arc::new(Mutex::new(create_tesseract(false).unwrap()));
    } else {
        lep_tess = TESSERACT_VEC.pop().unwrap();
    }
    lep_tess
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
                let ocr = create_tesseract(false).unwrap();
                TESSERACT_VEC.push(Arc::new(Mutex::new(ocr)));
            }
            if TESSERACT_NUMERIC_VEC.len() < 9 {
                let ocr = create_tesseract(true).unwrap();
                TESSERACT_NUMERIC_VEC.push(Arc::new(Mutex::new(ocr)));
            }
        }
        thread::sleep(tokio::time::Duration::from_millis(500));
    });
}
