use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TunnelEntry {
    pub pid: u32,
    pub port: u16,
    pub protocol: String,
    pub url: String,
    pub start_time: u64,
    pub metrics_port: u16,
}

pub struct Registry {
    path: PathBuf,
}

impl Registry {
    pub fn new() -> Self {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let path = Path::new(&home).join(".xpose").join("tunnels.json");
        let _ = fs::create_dir_all(path.parent().unwrap());
        Self { path }
    }

    pub fn register(&self, entry: TunnelEntry) -> Result<(), String> {
        let mut entries = self.list_all();
        entries.retain(|e| e.pid != entry.pid && e.port != entry.port);
        entries.push(entry);
        self.save(&entries)
    }

    pub fn unregister(&self, pid: u32) -> Result<(), String> {
        let mut entries = self.list_all();
        entries.retain(|e| e.pid != pid);
        self.save(&entries)
    }

    pub fn list_all(&self) -> Vec<TunnelEntry> {
        if !self.path.exists() {
            return Vec::new();
        }
        let content = fs::read_to_string(&self.path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn list_active(&self) -> Vec<TunnelEntry> {
        let mut entries = self.list_all();
        entries.retain(|e| is_process_running(e.pid));
        // Update the file if some zombies were found
        let initial_count = entries.len();
        if initial_count < self.list_all().len() {
            let _ = self.save(&entries);
        }
        entries
    }

    fn save(&self, entries: &[TunnelEntry]) -> Result<(), String> {
        let content = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
        fs::write(&self.path, content).map_err(|e| e.to_string())
    }
}

fn is_process_running(pid: u32) -> bool {
    if pid == 0 { return false; }
    #[cfg(unix)]
    {
        let path = format!("/proc/{}", pid);
        Path::new(&path).exists()
    }
    #[cfg(not(unix))]
    {
        // Simple fallback for other OS if needed, or use a crate like `sysinfo`
        true 
    }
}

pub fn get_now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process;
    use tempfile::tempdir;

    #[test]
    fn test_registry_registration() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tunnels.json");
        let registry = Registry { path: path.clone() };

        let entry = TunnelEntry {
            pid: process::id(),
            port: 3000,
            protocol: "tcp".to_string(),
            url: "http://test".to_string(),
            start_time: get_now_secs(),
            metrics_port: 55555,
        };

        registry.register(entry.clone()).unwrap();
        let active = registry.list_all();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].pid, entry.pid);

        registry.unregister(entry.pid).unwrap();
        assert_eq!(registry.list_all().len(), 0);
    }
}
