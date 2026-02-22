use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct XposeConfig {
    pub port: Option<u16>,
    pub protocol: Option<String>,
    pub lang: Option<String>,
    pub server_url: Option<String>,
    pub hooks: Option<HooksConfig>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct HooksConfig {
    pub on_connect: Option<String>,
}

impl XposeConfig {
    pub fn load() -> Self {
        let path = Path::new("xpose.yaml");
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(config) = serde_yaml::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }
}
