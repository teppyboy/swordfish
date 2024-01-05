pub use leptess::{LepTess, Variable};
use std::{sync::{
    Arc, Mutex, LazyLock
}, thread};

static TESSERACT: LazyLock<Arc<Mutex<LepTess>>> = LazyLock::new(|| {
    let mut lep_tess = match LepTess::new(None, "eng") {
        Ok(lep_tess) => lep_tess,
        Err(why) => panic!("{}", format!("Failed to initialize Tesseract: {:?}", why)),
    };
    // lep_tess.set_variable(Variable::TesseditPagesegMode, "6").unwrap();
    // Use LSTM only.
    lep_tess.set_variable(Variable::TesseditOcrEngineMode, "2").unwrap();
    Arc::new(Mutex::new(lep_tess))
});

static mut TESSERACT_VEC: Vec<Arc<Mutex<LepTess>>> = Vec::new();

pub fn get_tesseract(numeric_only: bool) -> Arc<Mutex<LepTess>> {
    TESSERACT.clone()
}

pub unsafe fn get_tesseract_from_vec(numeric_only: bool) -> Arc<Mutex<LepTess>> {
    let lep_tess: Arc<Mutex<LepTess>>;
    if TESSERACT_VEC.len() == 0 {
        for _ in 0..3 {
            let num_only = numeric_only.clone();
            thread::spawn(move || {
                let ocr = init_tesseract(num_only).unwrap();
                TESSERACT_VEC.push(Arc::new(Mutex::new(ocr)));
            });
        }
        lep_tess = Arc::new(Mutex::new(init_tesseract(numeric_only).unwrap()));
    } 
    else {
        lep_tess = TESSERACT_VEC.pop().unwrap();
        thread::spawn(move || unsafe {
            let ocr = init_tesseract(numeric_only).unwrap();
            TESSERACT_VEC.push(Arc::new(Mutex::new(ocr)));
        });
    }
    lep_tess
}

pub fn init_tesseract(numeric_only: bool) -> Result<LepTess, String> {
    let mut lep_tess = match LepTess::new(None, "eng") {
        Ok(lep_tess) => lep_tess,
        Err(why) => return Err(format!("Failed to initialize Tesseract: {:?}", why)),
    };
    lep_tess.set_variable(Variable::TesseditPagesegMode, "6").unwrap();
    // Use LSTM only.
    lep_tess.set_variable(Variable::TesseditOcrEngineMode, "1").unwrap();
    if numeric_only {
        match lep_tess.set_variable(Variable::TesseditCharWhitelist, "0123456789") {
            Ok(_) => (),
            Err(why) => return Err(format!("Failed to set whitelist: {:?}", why)),
        };
    }
    Ok(lep_tess)
}
