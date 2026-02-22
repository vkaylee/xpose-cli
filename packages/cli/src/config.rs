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

impl XposeConfig {
    pub fn load() -> Self {
        Self::load_from_path("xpose.yaml")
    }

    pub fn save(&self) -> Result<(), String> {
        self.save_to_path("xpose.yaml")
    }

    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = serde_yaml::to_string(self).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
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

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct HooksConfig {
    pub on_connect: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_default() {
        let config = XposeConfig::default();
        assert_eq!(config.port, None);
        assert_eq!(config.protocol, None);
    }

    #[test]
    fn test_config_load_no_file() {
        let config = XposeConfig::load_from_path("non_existent_file.yaml");
        assert_eq!(config.port, None);
    }

    #[test]
    fn test_config_load_valid_yaml() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "port: 8080\nprotocol: udp\nlang: vi\nhooks:\n  on_connect: \"echo hello\""
        )
        .unwrap();

        let config = XposeConfig::load_from_path(file.path());
        assert_eq!(config.port, Some(8080));
        assert_eq!(config.protocol, Some("udp".to_string()));
        assert_eq!(config.lang, Some("vi".to_string()));
        assert_eq!(
            config.hooks.unwrap().on_connect,
            Some("echo hello".to_string())
        );
    }
}
