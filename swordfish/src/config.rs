use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileLog {
    pub enabled: bool,
    pub path: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Log {
    pub level: String,
    pub file: FileLog,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tesseract {
    pub backend: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Debug {
    pub allowed_users: Vec<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct List {
    pub enabled: bool,
    pub servers: Vec<u64>,
    pub channels: Vec<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DropAnalyzer {
    pub enabled: bool,
    pub blacklist: List,
    pub whitelist: List,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Features {
    pub katana_drop_analysis: DropAnalyzer,
    pub sofa_drop_analysis: DropAnalyzer,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub log: Log,
    pub tesseract: Tesseract,
    pub debug: Debug,
    pub features: Features,
}

impl Config {
    pub fn new() -> Config {
        Config {
            log: Log {
                level: "info".to_string(),
                file: FileLog {
                    enabled: false,
                    path: "swordfish.log".to_string(),
                },
            },
            tesseract: Tesseract {
                backend: "libtesseract".to_string(),
            },
            debug: Debug {
                allowed_users: vec![],
            },
            features: Features {
                katana_drop_analysis: DropAnalyzer {
                    enabled: false,
                    blacklist: List {
                        enabled: false,
                        servers: vec![],
                        channels: vec![],
                    },
                    whitelist: List {
                        enabled: false,
                        servers: vec![],
                        channels: vec![],
                    },
                },
                sofa_drop_analysis: DropAnalyzer {
                    enabled: false,
                    blacklist: List {
                        enabled: false,
                        servers: vec![],
                        channels: vec![],
                    },
                    whitelist: List {
                        enabled: false,
                        servers: vec![],
                        channels: vec![],
                    },
                },
            },
        }
    }
    pub fn save(&self, path: &str) {
        let toml = toml::to_string(&self).unwrap();
        fs::write(path, toml).expect("Failed to write config file");
    }
    pub fn load(path: &str) -> Config {
        let content = fs::read_to_string(path).expect("Failed to read config file");
        let config: Config = toml::from_str(&content.as_str()).unwrap();
        return config;
    }
}
