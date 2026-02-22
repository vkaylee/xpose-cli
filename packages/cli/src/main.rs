mod api;
mod cloudflared;
mod config;
mod dashboard;
mod i18n;
mod registry;
mod ui;

use config::XposeConfig;

use clap::{
    builder::styling::{AnsiColor, Effects, Styles},
    Parser,
};
use console::style;
use dotenvy::dotenv;
use std::env;
use std::fs;
use std::process;
use tokio::signal;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use api::ApiClient;
use cloudflared::CloudflaredConfig;
use fern::Dispatch;
use log::{info, LevelFilter};
use std::sync::{Arc, Mutex};
use ui::Ui;

const KEY_SERVER_URL: &str = match option_env!("XPOSE_SERVER_URL") {
    Some(url) => url,
    None => "http://127.0.0.1:8787",
};

fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Green.on_default() | Effects::BOLD)
        .usage(AnsiColor::Magenta.on_default() | Effects::BOLD)
        .literal(AnsiColor::Cyan.on_default())
        .placeholder(AnsiColor::Cyan.on_default())
}

#[derive(Parser, Debug)]
#[command(
    name = "xpose",
    about = "Cloudflare Tunnel CLI for developers",
    styles = cli_styles(),
    after_help = "\x1b[1;32mEXAMPLES:\x1b[0m\n  \x1b[90m# Expose local port 3000\x1b[0m\n  \x1b[36m$ xpose 3000\x1b[0m\n\n  \x1b[90m# Open interactive dashboard\x1b[0m\n  \x1b[36m$ xpose dashboard\x1b[0m\n"
)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(help = "The local port to forward")]
    port: Option<u16>,

    #[arg(long, help = "Use UDP protocol instead of TCP")]
    udp: bool,

    #[arg(long, help = "Override language (en, vi, zh)")]
    lang: Option<String>,

    #[arg(long, help = "Key server URL to use")]
    server_url: Option<String>,

    #[arg(long, help = "Auto-open browser after connecting")]
    open: bool,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    #[command(about = "Open local management dashboard")]
    Dashboard,
    #[command(about = "Check and install CLI updates")]
    Update {
        #[arg(long, help = "Force update even if up to date")]
        force: bool,
    },
    #[command(about = "Configure xpose settings")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(clap::Subcommand, Debug)]
enum ConfigAction {
    #[command(about = "Set a configuration value")]
    Set { key: String, value: String },
    #[command(about = "Get a configuration value")]
    Get { key: String },
}

pub fn map_error(e: &str) -> String {
    if e.contains("timeout") {
        "Request timed out. Please check your internet connection.".to_string()
    } else if e.contains("403") {
        "Access denied. This port might be restricted for security reasons.".to_string()
    } else if e.contains("409") {
        "Tunnel collision. Someone else might be using this tunnel, please retry.".to_string()
    } else if e.contains("503") {
        "No tunnels available in the pool. Please try again later.".to_string()
    } else {
        format!("An unexpected error occurred: {e}")
    }
}

#[derive(Debug, PartialEq)]
enum VersionStatus {
    UpToDate,
    UpdateAvailable,
    Outdated,
}

fn check_version_compatibility(current: &str, min: &str, recommended: &str) -> VersionStatus {
    fn trim_v(s: &str) -> &str {
        s.trim_start_matches('v').trim()
    }
    let current_v =
        semver::Version::parse(trim_v(current)).unwrap_or_else(|_| semver::Version::new(0, 0, 0));
    let min_v =
        semver::Version::parse(trim_v(min)).unwrap_or_else(|_| semver::Version::new(0, 0, 0));
    let recommended_v = semver::Version::parse(trim_v(recommended))
        .unwrap_or_else(|_| semver::Version::new(0, 0, 0));

    if current_v < min_v {
        VersionStatus::Outdated
    } else if current_v < recommended_v {
        VersionStatus::UpdateAvailable
    } else {
        VersionStatus::UpToDate
    }
}

async fn send_telemetry(api: &ApiClient, event: &str, device_id: &str, details: serde_json::Value) {
    let payload = serde_json::json!({
        "event": event,
        "device_id": device_id,
        "details": details,
        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
    });
    let _ = api.post_telemetry(payload).await;
}

#[tokio::main]
async fn main() {
    let _ = dotenv();
    setup_logging().expect("Failed to initialize logging");

    // Load config from xpose.yaml
    let yaml_config = XposeConfig::load();

    let args = Args::parse();

    // Merge i18n: Args > YAML > Auto-detect
    let lang = args.lang.clone().or(yaml_config.lang.clone());
    let i18n = i18n::I18n::new(lang);
    let ui = Ui::new(i18n.clone());

    let registry = registry::Registry::new();

    if let Some(command) = args.command {
        let server_url = args
            .server_url
            .clone()
            .or(yaml_config.server_url.clone())
            .unwrap_or_else(|| KEY_SERVER_URL.to_string());

        let api_client = ApiClient::new(server_url.clone());

        match command {
            Commands::Dashboard => {
                let mut app = dashboard::DashboardApp::new(server_url.clone(), i18n);
                if let Err(e) = app.run() {
                    eprintln!("Error running dashboard: {e}");
                }
                process::exit(0);
            }
            Commands::Update { force } => {
                handle_update(&api_client, &ui, &i18n, force).await;
                process::exit(0);
            }
            Commands::Config { action } => {
                handle_config(action, yaml_config, &ui, &i18n).await;
                process::exit(0);
            }
        }
    }

    info!("{} v{}", i18n.t("startup"), env!("CARGO_PKG_VERSION"));

    // Merge Port & Protocol: Args > YAML > Env
    let port = args
        .port
        .or(yaml_config.port)
        .or_else(|| env::var("MT_TUNNEL_PORT").ok().and_then(|p| p.parse().ok()));

    let protocol = if args.udp || yaml_config.protocol.as_deref() == Some("udp") {
        "udp"
    } else {
        "tcp"
    };

    let port = match port {
        Some(p) => p,
        None => {
            ui.info(i18n.t("scanning_ports"));
            let mut found_port = None;
            for &p in &[3000, 8000, 8080] {
                if std::net::TcpStream::connect_timeout(
                    &format!("127.0.0.1:{p}").parse().unwrap(),
                    Duration::from_millis(100),
                )
                .is_ok()
                {
                    found_port = Some(p);
                    break;
                }
            }

            match found_port {
                Some(p) => {
                    ui.success(&format!("{} {}", i18n.t("found_service"), p));
                    p
                }
                None => {
                    ui.error(i18n.t("no_port_found"));
                    process::exit(1);
                }
            }
        }
    };

    let device_id = match get_machine_id() {
        Ok(id) => id,
        Err(_) => "unknown-device".to_string(),
    };

    let server_url = args
        .server_url
        .clone()
        .or(yaml_config.server_url.clone())
        .unwrap_or_else(|| KEY_SERVER_URL.to_string());

    let api_client = ApiClient::new(server_url.clone());

    let cf_config = CloudflaredConfig::new();
    if !cf_config.is_installed() {
        let pb = ui.create_spinner(i18n.t("downloading_binary"));
        if let Err(e) = cf_config.download().await {
            pb.finish_and_clear();
            ui.error(&map_error(&e));
            process::exit(1);
        }
        pb.finish_with_message(i18n.t("installed_success"));
    } else {
        ui.success(i18n.t("binary_found"));
    }

    // Version Check
    let config = match api_client.get_config().await {
        Ok(c) => c,
        Err(e) => {
            ui.error(&format!("Failed to fetch config: {}", e));
            process::exit(1);
        }
    };

    let current_version = env!("CARGO_PKG_VERSION");
    match check_version_compatibility(
        current_version,
        &config.min_cli_version,
        &config.recommended_version,
    ) {
        VersionStatus::Outdated => {
            ui.error(
                &i18n
                    .t("version_outdated")
                    .replace("{}", current_version)
                    .replace("{}", &config.min_cli_version),
            );
            process::exit(1);
        }
        VersionStatus::UpdateAvailable => {
            ui.info(
                &i18n
                    .t("update_available")
                    .replace("{}", &config.recommended_version)
                    .replace("{}", current_version),
            );
        }
        VersionStatus::UpToDate => {}
    }

    // QR Authentication Handshake
    ui.draw_auth_panel(); // Transitioning state

    let auth_init = match api_client.init_auth().await {
        Ok(init) => init,
        Err(e) => {
            ui.error(&format!("Failed to initiate authentication: {}", e));
            process::exit(1);
        }
    };
    ui.draw_qr_auth(&auth_init.verify_url);

    println!(
        "\n  {} {}",
        style("➜").cyan(),
        style(i18n.t("help_qr_scan")).bold()
    );
    println!(
        "  {} {}\n",
        style("⚡").yellow(),
        style(&auth_init.verify_url).underlined().dim()
    );

    // Poll for verification
    let session_id = auth_init.session_id.clone();
    let auth_token = auth_init.auth_token.clone();

    let mut verified = false;
    let start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(300); // 5 mins

    while !verified {
        if start_time.elapsed() > timeout {
            ui.error("Authentication timed out.");
            process::exit(1);
        }

        let status = match api_client.check_auth_status(&session_id, &auth_token).await {
            Ok(s) => s,
            Err(e) => {
                ui.error(&format!("Failed to check authentication status: {}", e));
                process::exit(1);
            }
        };

        if status == "VERIFIED" {
            verified = true;
        } else {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    ui.success("Authenticated successfully!");

    let pb = ui.create_spinner(i18n.t("requesting_tunnel"));
    let tunnel_info = match api_client
        .request_tunnel(
            &device_id,
            Some(port),
            Some(protocol),
            Some(session_id.clone()),
            Some(auth_token.clone()),
        )
        .await
    {
        Ok(info) => info,
        Err(e) => {
            pb.finish_and_clear();
            ui.error(&map_error(&e.to_string()));
            process::exit(1);
        }
    };
    pb.finish_with_message(i18n.t("tunnel_allocated"));

    let active_tunnels = registry.list_active();
    let mut metrics_port = 55555;
    while active_tunnels
        .iter()
        .any(|t| t.metrics_port == metrics_port)
    {
        metrics_port += 1;
    }

    let mut child = match cf_config.start_tunnel(&tunnel_info.token, metrics_port) {
        Ok(c) => c,
        Err(e) => {
            ui.error(&format!("Failed to start cloudflared: {e}"));
            let _ = api_client.release_tunnel(&device_id).await;
            process::exit(1);
        }
    };

    let public_url = format!("https://{}.trycloudflare.com", tunnel_info.name);
    sleep(Duration::from_millis(1500)).await;

    ui.draw_connected_panel(port, &public_url, protocol);

    // Hook: on_connect
    let mut hook_executed = false;
    if let Some(hook) = yaml_config.hooks.and_then(|h| h.on_connect) {
        let cmd = hook.replace("{}", &public_url);
        info!("Executing on_connect hook: {}", cmd);
        let _ = if cfg!(target_os = "windows") {
            process::Command::new("cmd").args(["/C", &cmd]).spawn()
        } else {
            process::Command::new("sh").args(["-c", &cmd]).spawn()
        };
        hook_executed = true;
    }

    if args.open && !hook_executed {
        let _ = opener::open(&public_url);
    }

    ui.info(i18n.t("running_background"));

    let _ = registry.register(registry::TunnelEntry {
        pid: process::id(),
        port,
        protocol: protocol.to_string(),
        url: public_url.clone(),
        start_time: registry::get_now_secs(),
        metrics_port,
    });

    let heartbeat_device = device_id.clone();
    let heartbeat_api = ApiClient::new(server_url.clone());
    let metrics_url = format!("http://localhost:{metrics_port}/metrics");
    let metrics_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let ui_ref = Arc::new(Mutex::new(ui));
    let ui_clone = ui_ref.clone();

    let health_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap();
    let health_url = format!("http://localhost:{port}");
    let ui_health_clone = ui_ref.clone();

    let telemetry_api = ApiClient::new(server_url.clone());
    let telemetry_device = device_id.clone();

    let handle = tokio::spawn(async move {
        let mut last_rx: u64 = 0;
        let mut last_tx: u64 = 0;
        let mut tick_count = 0;
        let mut sys = sysinfo::System::new();
        let pid = sysinfo::Pid::from(process::id() as usize);

        loop {
            sleep(Duration::from_secs(1)).await;
            tick_count += 1;
            if tick_count >= 300 {
                let _ = heartbeat_api.send_heartbeat(&heartbeat_device).await;
                tick_count = 0;
            }

            // Fetch RAM usage
            sys.refresh_processes_specifics(
                sysinfo::ProcessesToUpdate::Some(&[pid]),
                false,
                sysinfo::ProcessRefreshKind::nothing().with_memory(),
            );
            let ram_bytes = sys.process(pid).map(|p| p.memory()).unwrap_or(0);

            let start = tokio::time::Instant::now();
            if let Ok(res) = metrics_client.get(&metrics_url).send().await {
                let ping_ms = start.elapsed().as_millis() as u64;
                if let Ok(text) = res.text().await {
                    let mut rx_bytes = last_rx;
                    let mut tx_bytes = last_tx;

                    for line in text.lines() {
                        if line.starts_with("cloudflared_tunnel_rx_bytes") {
                            if let Some(val) = line.split_whitespace().last() {
                                rx_bytes = val.parse().unwrap_or(last_rx);
                            }
                        } else if line.starts_with("cloudflared_tunnel_tx_bytes") {
                            if let Some(val) = line.split_whitespace().last() {
                                tx_bytes = val.parse().unwrap_or(last_tx);
                            }
                        }
                    }

                    let rx_speed = rx_bytes.saturating_sub(last_rx);
                    let tx_speed = tx_bytes.saturating_sub(last_tx);
                    last_rx = rx_bytes;
                    last_tx = tx_bytes;

                    if let Ok(mut ui) = ui_clone.lock() {
                        ui.draw_live_metrics(
                            rx_bytes, tx_bytes, rx_speed, tx_speed, ping_ms, ram_bytes,
                        );
                    }
                }
            }

            // Health check every 5 seconds
            if tick_count % 5 == 0 {
                if let Ok(res) = health_client.get(&health_url).send().await {
                    if res.status().is_server_error() {
                        if let Ok(ui) = ui_health_clone.lock() {
                            ui.error(&format!(
                                "Warning: Local service on port {} returned status {}",
                                port,
                                res.status()
                            ));
                        }
                    }
                }
            }
        }
    });

    let mut sigint = Box::pin(signal::ctrl_c());

    loop {
        tokio::select! {
            _ = &mut sigint => {
                println!("\nShutting down tunnel...");
                let _ = child.kill();
                send_telemetry(&telemetry_api, "stop", &telemetry_device, serde_json::json!({})).await;
                let _ = api_client.release_tunnel(&device_id).await;
                let _ = registry::Registry::new().unregister(process::id());
                handle.abort();
                process::exit(0);
            }
            res = tokio::task::spawn_blocking(|| crossterm::event::poll(Duration::from_millis(100))) => {
                if let Ok(Ok(true)) = res {
                    if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
                        match key.code {
                            crossterm::event::KeyCode::Char('x') => {
                                println!("\nStopping tunnel as requested...");
                                let _ = child.kill();
                                send_telemetry(&telemetry_api, "stop", &telemetry_device, serde_json::json!({})).await;
                                let _ = api_client.release_tunnel(&device_id).await;
                                let _ = registry::Registry::new().unregister(process::id());
                                handle.abort();
                                process::exit(0);
                            }
                            crossterm::event::KeyCode::Char('r') => {
                                println!("\nRestarting tunnel...");
                                let _ = child.kill();
                                let _ = api_client.release_tunnel(&device_id).await;
                                let _ = registry::Registry::new().unregister(process::id());
                                handle.abort();
                                ui_ref.lock().unwrap().info("Tunnel stopped. Re-run command to restart.");
                                process::exit(0);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

fn setup_logging() -> Result<(), fern::InitError> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let log_dir = std::path::Path::new(&home).join(".xpose").join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = log_dir.join("xpose.log");

    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Info)
        .chain(fern::log_file(log_file)?)
        .apply()?;

    Ok(())
}

fn get_machine_id() -> Result<String, Box<dyn std::error::Error>> {
    let path = dirs::home_dir()
        .ok_or("Could not find home directory")?
        .join(".xpose")
        .join("device_id");
    get_machine_id_from_path(path)
}

fn get_machine_id_from_path(
    path: std::path::PathBuf,
) -> Result<String, Box<dyn std::error::Error>> {
    if path.exists() {
        return Ok(fs::read_to_string(path)?.trim().to_string());
    }

    let id = Uuid::new_v4().to_string();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, &id)?;
    Ok(id)
}

async fn handle_update(api: &ApiClient, ui: &Ui, i18n: &i18n::I18n, force: bool) {
    let config = match api.get_config().await {
        Ok(c) => c,
        Err(e) => {
            ui.error(&format!("Failed to fetch update info: {}", e));
            return;
        }
    };

    let current = env!("CARGO_PKG_VERSION");
    let status = check_version_compatibility(
        current,
        &config.min_cli_version,
        &config.recommended_version,
    );

    if !force && status == VersionStatus::UpToDate {
        ui.success(i18n.t("up_to_date"));
        return;
    }

    ui.info(&format!(
        "{} v{}...",
        i18n.t("updating"),
        config.recommended_version
    ));

    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    let release_name = match (os, arch) {
        ("linux", "x86_64") => "xpose-linux-amd64",
        ("macos", "x86_64") => "xpose-darwin-amd64",
        ("macos", "aarch64") => "xpose-darwin-arm64",
        ("windows", "x86_64") => "xpose-windows-amd64.exe",
        _ => {
            ui.error("Unsupported platform for self-update");
            return;
        }
    };

    let url = format!(
        "https://github.com/vkaylee/xpose-cli/releases/download/v{}/{}",
        config.recommended_version, release_name
    );

    let pb = ui.create_spinner(i18n.t("downloading_update"));
    let client = reqwest::Client::new();
    let res = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            pb.finish_and_clear();
            ui.error(&format!("Download failed: {e}"));
            return;
        }
    };

    if !res.status().is_success() {
        pb.finish_and_clear();
        ui.error(&format!("Server returned error: {}", res.status()));
        return;
    }

    let bytes = match res.bytes().await {
        Ok(b) => b.to_vec(),
        Err(e) => {
            pb.finish_and_clear();
            ui.error(&format!("Failed to read response body: {e}"));
            return;
        }
    };

    let current_exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => {
            pb.finish_and_clear();
            ui.error(&format!("Could not determine current executable path: {e}"));
            return;
        }
    };

    let mut temp_exe = current_exe.clone();
    temp_exe.set_extension("tmp");

    if let Err(e) = fs::write(&temp_exe, &bytes) {
        pb.finish_and_clear();
        ui.error(&format!("Failed to write temporary file: {e}"));
        return;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&temp_exe) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&temp_exe, perms);
        }
    }

    if let Err(e) = fs::rename(&temp_exe, &current_exe) {
        pb.finish_and_clear();
        ui.error(&format!(
            "Failed to replace binary: {e}. Try running with sudo/admin."
        ));
        let _ = fs::remove_file(&temp_exe);
        return;
    }

    pb.finish_and_clear();
    ui.success(&format!(
        "{} v{}",
        i18n.t("update_success"),
        config.recommended_version
    ));
}

async fn handle_config(
    action: ConfigAction,
    mut config: XposeConfig,
    ui: &ui::Ui,
    i18n: &i18n::I18n,
) {
    match action {
        ConfigAction::Set { key, value } => {
            match key.as_str() {
                "server_url" => config.server_url = Some(value.clone()),
                "lang" => config.lang = Some(value.clone()),
                "port" => config.port = value.parse().ok(),
                "protocol" => config.protocol = Some(value.clone()),
                _ => {
                    ui.error(&i18n.t("config_error").replace("{}", &key));
                    return;
                }
            }
            if let Err(e) = config.save() {
                ui.error(&format!("Failed to save config: {}", e));
            } else {
                let msg = i18n
                    .t("config_success")
                    .replacen("{}", &key, 1)
                    .replacen("{}", &value, 1);
                ui.success(&msg);
            }
        }
        ConfigAction::Get { key } => {
            let val = match key.as_str() {
                "server_url" => config.server_url.clone(),
                "lang" => config.lang.clone(),
                "port" => config.port.map(|p| p.to_string()),
                "protocol" => config.protocol.clone(),
                _ => {
                    ui.error(&i18n.t("config_error").replace("{}", &key));
                    return;
                }
            };
            println!("{}", val.unwrap_or_else(|| "not set".to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() {
        // Standard cases
        assert_eq!(
            check_version_compatibility("0.1.0", "0.1.0", "0.1.0"),
            VersionStatus::UpToDate
        );
        assert_eq!(
            check_version_compatibility("0.1.0", "0.2.0", "0.2.0"),
            VersionStatus::Outdated
        );
        assert_eq!(
            check_version_compatibility("0.1.0", "0.1.0", "0.2.0"),
            VersionStatus::UpdateAvailable
        );
        assert_eq!(
            check_version_compatibility("0.2.0", "0.1.0", "0.1.0"),
            VersionStatus::UpToDate
        );

        // Multi-digit components
        assert_eq!(
            check_version_compatibility("0.4.11", "0.4.9", "0.4.10"),
            VersionStatus::UpToDate
        );
        assert_eq!(
            check_version_compatibility("0.4.9", "0.4.11", "0.4.11"),
            VersionStatus::Outdated
        );
        assert_eq!(
            check_version_compatibility("0.4.10", "0.4.9", "0.4.11"),
            VersionStatus::UpdateAvailable
        );

        // 'v' prefix handling
        assert_eq!(
            check_version_compatibility("v0.4.11", "0.4.11", "0.4.11"),
            VersionStatus::UpToDate
        );
        assert_eq!(
            check_version_compatibility("0.4.11", "v0.4.11", "v0.4.11"),
            VersionStatus::UpToDate
        );
        assert_eq!(
            check_version_compatibility("v0.4.9", "v0.4.11", "v0.4.11"),
            VersionStatus::Outdated
        );

        // Edge cases
        assert_eq!(
            check_version_compatibility("0.2.0", "0.2.0", "0.3.0"),
            VersionStatus::UpdateAvailable
        );
        assert_eq!(
            check_version_compatibility("0.0.9", "0.1.0", "0.1.0"),
            VersionStatus::Outdated
        );
    }

    #[test]
    fn test_map_error_timeout() {
        assert_eq!(
            map_error("connection timeout"),
            "Request timed out. Please check your internet connection."
        );
    }

    #[test]
    fn test_map_error_403() {
        assert_eq!(
            map_error("error 403: forbidden"),
            "Access denied. This port might be restricted for security reasons."
        );
    }

    #[test]
    fn test_map_error_409() {
        assert_eq!(
            map_error("error 409: conflict"),
            "Tunnel collision. Someone else might be using this tunnel, please retry."
        );
    }

    #[test]
    fn test_map_error_503() {
        assert_eq!(
            map_error("error 503: unavailable"),
            "No tunnels available in the pool. Please try again later."
        );
    }

    #[test]
    fn test_map_error_unexpected() {
        assert_eq!(
            map_error("some weird error"),
            "An unexpected error occurred: some weird error"
        );
    }

    #[test]
    fn test_get_machine_id_logic() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("device_id");

        // Initial generation
        let id1 = get_machine_id_from_path(path.clone()).unwrap();
        assert!(Uuid::parse_str(&id1).is_ok());

        // Persistence check
        let id2 = get_machine_id_from_path(path).unwrap();
        assert_eq!(id1, id2);
    }
}
