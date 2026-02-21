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
        let bin_name = if cfg!(target_os = "windows") {
            "cloudflared.exe"
        } else {
            "cloudflared"
        };

        let mut paths_to_try = Vec::new();

        if let Ok(home) = env::var("HOME").or_else(|_| env::var("USERPROFILE")) {
            paths_to_try.push(Path::new(&home).join(".xpose").join("bin"));
        }

        // 2. Temp directory
        paths_to_try.push(env::temp_dir().join("xpose-bin"));

        // 3. Project local directory
        paths_to_try.push(Path::new(".").join(".xpose-bin"));

        for bin_dir in paths_to_try {
            if !bin_dir.exists() {
                if fs::create_dir_all(&bin_dir).is_ok() {
                    let p = bin_dir.join(bin_name);
                    return Self { bin_path: p };
                }
            } else {
                let p = bin_dir.join(bin_name);
                return Self { bin_path: p };
            }
        }

        // Ultimate fallback to current directory
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

        // Map Rust's os/arch to Cloudflared release names
        let release_name = match (os, arch) {
            ("linux", "x86_64") => "cloudflared-linux-amd64",
            ("linux", "aarch64") => "cloudflared-linux-arm64",
            ("macos", "x86_64") => "cloudflared-darwin-amd64.tgz",
            ("macos", "aarch64") => "cloudflared-darwin-arm64.tgz",
            ("windows", "x86_64") => "cloudflared-windows-amd64.exe",
            _ => return Err(format!("Unsupported OS or architecture: {} {}", os, arch)),
        };

        let url = format!(
            "https://github.com/cloudflare/cloudflared/releases/latest/download/{}",
            release_name
        );

        info!(
            "Downloading cloudflared binary for {} {} from {}",
            os, arch, url
        );

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
        if !license_path.exists()
            && let Ok(mut res) = client.get(license_url).send().await
            && let Ok(mut file) = fs::File::create(&license_path)
        {
            while let Some(chunk) = res.chunk().await.ok().flatten() {
                let _ = file.write_all(&chunk);
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
        let metrics_addr = format!("localhost:{}", metrics_port);
        let child = Command::new(&self.bin_path)
            .arg("tunnel")
            .arg("--metrics")
            .arg(&metrics_addr)
            .arg("run")
            .arg("--token")
            .arg(token)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;

        Ok(child)
    }
}

fn request_is_archive(name: &str) -> bool {
    name.ends_with(".tgz") || name.ends_with(".zip") || name.ends_with(".tar.gz")
}
