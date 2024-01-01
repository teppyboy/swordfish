pub use leptess::{LepTess, Variable};

pub fn init_tesseract(numeric_only: bool) -> Result<LepTess, String> {
    let mut lep_tess = match LepTess::new(None, "eng") {
        Ok(lep_tess) => lep_tess,
        Err(why) => return Err(format!("Failed to initialize Tesseract: {:?}", why)),
    };
    if numeric_only {
        match lep_tess.set_variable(Variable::TesseditCharWhitelist, "0123456789") {
            Ok(_) => (),
            Err(why) => return Err(format!("Failed to set whitelist: {:?}", why)),
        };
    }
    Ok(lep_tess)
}
