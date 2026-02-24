use log::info;
use reqwest::Client;
use std::env;
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct CloudflaredConfig {
    pub bin_path: PathBuf,
}

impl CloudflaredConfig {
    pub fn new() -> Self {
        Self::from_env(
            env::var("HOME").or_else(|_| env::var("USERPROFILE")).ok(),
            Some(env::temp_dir()),
            Some(PathBuf::from(".")),
        )
    }

    pub fn from_env(
        home: Option<String>,
        temp: Option<PathBuf>,
        project_root: Option<PathBuf>,
    ) -> Self {
        let bin_name = if cfg!(target_os = "windows") {
            "cloudflared.exe"
        } else {
            "cloudflared"
        };

        let mut paths_to_try = Vec::new();

        if let Some(h) = home {
            paths_to_try.push(Path::new(&h).join(".xpose").join("bin"));
        }

        if let Some(t) = temp {
            paths_to_try.push(t.join("xpose-bin"));
        }

        if let Some(p) = project_root {
            paths_to_try.push(p.join(".xpose-bin"));
        }

        if let Some(bin_dir) = paths_to_try.into_iter().next() {
            // In real usage we might create dir, but for testing or if it exists...
            // To make it safe to run in tests, we don't necessarily create_dir_all here
            // if we just want to determine the path.
            // Actually the current code DOES create_dir_all.
            let p = bin_dir.join(bin_name);
            // Return the first one that we can reasonably use.
            // For now let's keep the logic of returning the first one.
            return Self { bin_path: p };
        }

        Self {
            bin_path: PathBuf::from(bin_name),
        }
    }

    pub fn is_installed(&self) -> bool {
        self.bin_path.exists()
    }

    pub async fn download(&self) -> Result<(), String> {
        let os = env::consts::OS;
        let arch = env::consts::ARCH;

        let release_name = match get_release_name(os, arch) {
            Ok(name) => name,
            Err(e) => return Err(e),
        };

        let url = get_download_url(release_name);

        info!("Downloading cloudflared binary for {os} {arch} from {url}");

        let client = Client::new();
        let mut response = client.get(&url).send().await.map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to download cloudflared: HTTP {}",
                response.status()
            ));
        }

        // Write directly if it's the executable
        if request_is_archive(release_name) {
            let tmp_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
            let archive_path = tmp_dir.path().join("archive");

            let mut file = fs::File::create(&archive_path).map_err(|e| e.to_string())?;
            while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
                file.write_all(&chunk).map_err(|e| e.to_string())?;
            }

            if release_name.ends_with(".tgz") || release_name.ends_with(".tar.gz") {
                let tar_gz = fs::File::open(&archive_path).map_err(|e| e.to_string())?;
                let tar = flate2::read::GzDecoder::new(tar_gz);
                let mut archive = tar::Archive::new(tar);
                for file in archive.entries().map_err(|e| e.to_string())? {
                    let mut file = file.map_err(|e| e.to_string())?;
                    let path = file.path().map_err(|e| e.to_string())?.to_path_buf();
                    // Just extract the cloudflared binary
                    if path.file_name().unwrap_or_default() == "cloudflared" {
                        file.unpack(&self.bin_path).map_err(|e| e.to_string())?;
                        break;
                    }
                }
            } else if release_name.ends_with(".zip") {
                let zip_file = fs::File::open(&archive_path).map_err(|e| e.to_string())?;
                let mut archive = zip::ZipArchive::new(zip_file).map_err(|e| e.to_string())?;
                for i in 0..archive.len() {
                    let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
                    if file.name().contains("cloudflared.exe") {
                        let mut outpath =
                            fs::File::create(&self.bin_path).map_err(|e| e.to_string())?;
                        std::io::copy(&mut file, &mut outpath).map_err(|e| e.to_string())?;
                        break;
                    }
                }
            }
        } else {
            let mut file = fs::File::create(&self.bin_path).map_err(|e| e.to_string())?;
            while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
                file.write_all(&chunk).map_err(|e| e.to_string())?;
            }
        }

        // Set executable permissions on Unix
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&self.bin_path)
                .map_err(|e| e.to_string())?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&self.bin_path, perms).map_err(|e| e.to_string())?;
        }

        // Download License for compliance
        let license_url = "https://raw.githubusercontent.com/cloudflare/cloudflared/master/LICENSE";
        let license_path = self.bin_path.parent().unwrap().join("LICENSE.cloudflared");
        if !license_path.exists() {
            if let Ok(mut res) = client.get(license_url).send().await {
                if let Ok(mut file) = fs::File::create(&license_path) {
                    while let Some(chunk) = res.chunk().await.ok().flatten() {
                        let _ = file.write_all(&chunk);
                    }
                }
            }
        }

        info!("Downloaded cloudflared and license successfully.");
        Ok(())
    }

    pub fn start_tunnel(
        &self,
        token: &str,
        metrics_port: u16,
    ) -> Result<std::process::Child, String> {
        self.create_tunnel_command(token, metrics_port)
            .spawn()
            .map_err(|e| e.to_string())
    }

    pub fn create_tunnel_command(&self, token: &str, metrics_port: u16) -> Command {
        let metrics_addr = format!("localhost:{metrics_port}");
        let mut cmd = Command::new(&self.bin_path);
        cmd.arg("tunnel")
            .arg("--metrics")
            .arg(&metrics_addr)
            .arg("run")
            .arg("--token")
            .arg(token)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped()); // capture stderr to extract hostname
        cmd
    }
}

/// Parse the public hostname from a single cloudflared JSON log line.
/// cloudflared outputs structured JSON logs to stderr, e.g.:
/// {"level":"info","hostname":"abc123.trycloudflare.com",...}
/// We also look for "host" and "url" fallbacks.
pub fn parse_hostname_from_log_line(line: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    // Named tunnels log the hostname in different fields depending on version:
    // - "hostname": the configured ingress hostname
    // - "host": seen in some routing log lines
    // - "url": seen in quick-tunnel confirmation lines
    for key in &["hostname", "host", "url"] {
        if let Some(s) = v.get(key).and_then(|v| v.as_str()) {
            let s = s.trim();
            if !s.is_empty()
                // must look like a hostname/URL, not an internal address
                && !s.starts_with("localhost")
                && !s.starts_with("127.")
                && s.contains('.')
            {
                // Normalise – strip any http/https scheme so caller decides
                let host = s
                    .trim_start_matches("https://")
                    .trim_start_matches("http://");
                return Some(format!("https://{host}"));
            }
        }
    }
    None
}

pub fn get_release_name(os: &str, arch: &str) -> Result<&'static str, String> {
    match (os, arch) {
        ("linux", "x86_64") => Ok("cloudflared-linux-amd64"),
        ("linux", "aarch64") => Ok("cloudflared-linux-arm64"),
        ("macos", "x86_64") => Ok("cloudflared-darwin-amd64.tgz"),
        ("macos", "aarch64") => Ok("cloudflared-darwin-arm64.tgz"),
        ("windows", "x86_64") => Ok("cloudflared-windows-amd64.exe"),
        _ => Err(format!("Unsupported OS or architecture: {os} {arch}")),
    }
}

pub fn get_download_url(release_name: &str) -> String {
    format!("https://github.com/cloudflare/cloudflared/releases/latest/download/{release_name}")
}

fn request_is_archive(name: &str) -> bool {
    name.ends_with(".tgz") || name.ends_with(".zip") || name.ends_with(".tar.gz")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_is_archive() {
        assert!(request_is_archive("file.tgz"));
        assert!(request_is_archive("file.zip"));
        assert!(request_is_archive("file.tar.gz"));
        assert!(!request_is_archive("file.exe"));
        assert!(!request_is_archive("cloudflared-linux-amd64"));
    }

    #[test]
    fn test_create_tunnel_command() {
        let config = CloudflaredConfig {
            bin_path: PathBuf::from("/usr/bin/cloudflared"),
        };
        let cmd = config.create_tunnel_command("test-token", 1234);

        assert_eq!(cmd.get_program(), "/usr/bin/cloudflared");
        let args: Vec<_> = cmd.get_args().map(|s| s.to_str().unwrap()).collect();
        assert_eq!(
            args,
            vec![
                "tunnel",
                "--metrics",
                "localhost:1234",
                "run",
                "--token",
                "test-token"
            ]
        );
    }

    #[test]
    fn test_cloudflared_config_new_fallback() {
        // This test ensures the constructor doesn't crash and returns a path
        let config = CloudflaredConfig::new();
        assert!(!config.bin_path.as_os_str().is_empty());
    }

    #[test]
    fn test_get_release_name() {
        assert_eq!(
            get_release_name("linux", "x86_64").unwrap(),
            "cloudflared-linux-amd64"
        );
        assert_eq!(
            get_release_name("linux", "aarch64").unwrap(),
            "cloudflared-linux-arm64"
        );
        assert_eq!(
            get_release_name("macos", "x86_64").unwrap(),
            "cloudflared-darwin-amd64.tgz"
        );
        assert_eq!(
            get_release_name("macos", "aarch64").unwrap(),
            "cloudflared-darwin-arm64.tgz"
        );
        assert_eq!(
            get_release_name("windows", "x86_64").unwrap(),
            "cloudflared-windows-amd64.exe"
        );
        assert!(get_release_name("unknown", "x86_64").is_err());
        assert!(get_release_name("linux", "ppc64le").is_err());
    }

    #[test]
    fn test_cloudflared_config_from_env() {
        let home = Some("/home/user".to_string());
        let temp = Some(PathBuf::from("/tmp"));
        let proj = Some(PathBuf::from("."));

        let config = CloudflaredConfig::from_env(home, temp, proj);
        let path = config.bin_path.to_str().unwrap();
        assert!(path.contains(".xpose/bin"));
    }

    #[test]
    fn test_cloudflared_config_fallback() {
        let config = CloudflaredConfig::from_env(None, None, None);
        let path = config.bin_path.to_str().unwrap();
        assert!(path.contains("cloudflared"));
    }

    #[test]
    fn test_get_download_url() {
        let url = get_download_url("test-release");
        assert!(url.contains("test-release"));
        assert!(
            url.starts_with("https://github.com/cloudflare/cloudflared/releases/latest/download/")
        );
    }

    #[test]
    fn test_cloudflared_is_installed_logic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let bin_path = temp_dir.path().join("cloudflared");

        let config = CloudflaredConfig {
            bin_path: bin_path.clone(),
        };

        // Not installed yet
        assert!(!config.is_installed());

        // Create dummy bin
        fs::write(&bin_path, "dummy").unwrap();
        assert!(config.is_installed());
    }

    #[test]
    fn test_cloudflared_config_from_env_variations() {
        let home = Some("/home/user".to_string());
        let temp = Some(PathBuf::from("/tmp"));
        let proj = Some(PathBuf::from("/workspace"));

        // Case: Home directory only
        let config = CloudflaredConfig::from_env(home.clone(), None, None);
        assert!(config.bin_path.to_str().unwrap().contains("/home/user"));

        // Case: Temp directory only
        let config = CloudflaredConfig::from_env(None, temp.clone(), None);
        assert!(config.bin_path.to_str().unwrap().contains("/tmp"));

        // Case: Project root only
        let config = CloudflaredConfig::from_env(None, None, proj.clone());
        assert!(config.bin_path.to_str().unwrap().contains("/workspace"));
    }

    #[test]
    fn test_get_release_name_unsupported() {
        let res = get_release_name("unknown-os", "unknown-arch");
        assert!(res.is_err());
    }
}
