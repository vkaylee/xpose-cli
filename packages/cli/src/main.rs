mod api;
mod cloudflared;
mod ui;

use clap::{
    Parser,
    builder::styling::{AnsiColor, Effects, Styles},
};
use dotenvy::dotenv;
use std::env;
use std::process;
use tokio::signal;
use tokio::time::{Duration, sleep};

use api::ApiClient;
use cloudflared::CloudflaredConfig;
use fern::Dispatch;
use log::{LevelFilter, info};
use std::sync::{Arc, Mutex};
use ui::Ui;

const KEY_SERVER_URL: &str = "http://127.0.0.1:8787";

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
    after_help = "\
\x1b[1;32mEXAMPLES:\x1b[0m
  \x1b[90m# Expose local port 3000 via TCP\x1b[0m
  \x1b[36m$ xpose 3000\x1b[0m

  \x1b[90m# Expose local port 8080 via UDP\x1b[0m
  \x1b[36m$ xpose 8080 --udp\x1b[0m

  \x1b[90m# Expose without passing a port (reads from .env MT_TUNNEL_PORT or auto-detects)\x1b[0m
  \x1b[36m$ xpose\x1b[0m

\x1b[1;32mTYPICAL WORKFLOW:\x1b[0m
  Run \x1b[36m'xpose <port>'\x1b[0m and you'll get a public URL instantly. 
  The connection will stay alive as long as this terminal is open.
"
)]
struct Args {
    #[arg(
        help = "The local port to forward (optional, defaults to auto-detect or .env MT_TUNNEL_PORT)"
    )]
    port: Option<u16>,

    #[arg(long, help = "Use UDP protocol instead of TCP")]
    udp: bool,
}

fn map_error(e: &str) -> String {
    if e.contains("timeout") {
        "Request timed out. Please check your internet connection.".to_string()
    } else if e.contains("403") {
        "Access denied. This port might be restricted for security reasons.".to_string()
    } else if e.contains("409") {
        "Tunnel collision. Someone else might be using this tunnel, please retry.".to_string()
    } else if e.contains("503") {
        "No tunnels available in the pool. Please try again later.".to_string()
    } else {
        format!("An unexpected error occurred: {}", e)
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
    info!("Starting xpose CLI v{}", env!("CARGO_PKG_VERSION"));

    let args = Args::parse();
    let ui = Ui::new();

    let port = args
        .port
        .or_else(|| env::var("MT_TUNNEL_PORT").ok().and_then(|p| p.parse().ok()));

    let port = match port {
        Some(p) => p,
        None => {
            ui.info("No port specified. Scanning common ports (3000, 8000, 8080)...");
            let mut found_port = None;
            for &p in &[3000, 8000, 8080] {
                if std::net::TcpStream::connect_timeout(
                    &format!("127.0.0.1:{}", p).parse().unwrap(),
                    Duration::from_millis(150),
                )
                .is_ok()
                {
                    found_port = Some(p);
                    break;
                }
            }

            match found_port {
                Some(p) => {
                    ui.success(&format!("Found active service on port {}", p));
                    p
                }
                None => {
                    ui.error("No active service found on common ports. Please specify a port.");
                    println!("Usage: xpose <PORT>\nExample: xpose 3000");
                    process::exit(1);
                }
            }
        }
    };

    let protocol = if args.udp { "udp" } else { "tcp" };
    let device_id = format!(
        "device_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    let api_client = ApiClient::new(KEY_SERVER_URL.to_string());
    let cf_config = CloudflaredConfig::new();

    send_telemetry(
        &api_client,
        "start",
        &device_id,
        serde_json::json!({"port": port, "protocol": protocol}),
    )
    .await;

    ui.info("Checking environment...");

    if !cf_config.is_installed() {
        let pb = ui.create_spinner("Downloading cloudflared binary...");
        if let Err(e) = cf_config.download().await {
            pb.finish_and_clear();
            ui.error(&map_error(&e));
            process::exit(1);
        }
        pb.finish_with_message("Cloudflared installed successfully.");
    } else {
        ui.success("Cloudflared binary found.");
    }

    // Version Check
    if let Ok(config) = api_client.get_config().await {
        let current_version = env!("CARGO_PKG_VERSION");
        if config.min_cli_version.as_str() > current_version {
            ui.error(&format!("Critical: Your CLI version (v{}) is outdated. Minimum required: v{}. Please update.", current_version, config.min_cli_version));
            process::exit(1);
        } else if config.recommended_version.as_str() > current_version {
            ui.info(&format!(
                "Update available: v{} (Current: v{}). Please run 'npm update -g xpose-cli' soon.",
                config.recommended_version, current_version
            ));
        }
    }

    let pb = ui.create_spinner("Requesting tunnel from pool...");
    let tunnel_info = match api_client
        .request_tunnel(&device_id, Some(port), Some(protocol))
        .await
    {
        Ok(info) => info,
        Err(e) => {
            pb.finish_and_clear();
            ui.error(&map_error(&e.to_string()));
            process::exit(1);
        }
    };
    pb.finish_with_message("Tunnel allocated.");

    let metrics_port = 55555;
    let mut child = match cf_config.start_tunnel(&tunnel_info.token, metrics_port) {
        Ok(c) => c,
        Err(e) => {
            ui.error(&format!("Failed to start cloudflared: {}", e));
            let _ = api_client.release_tunnel(&device_id).await;
            process::exit(1);
        }
    };

    let public_url = format!("https://{}.trycloudflare.com", tunnel_info.name);
    sleep(Duration::from_millis(1500)).await;

    ui.draw_connected_panel(port, &public_url, protocol);
    ui.info("cloudflared is running in background. Tunnel token hidden for security.");

    let heartbeat_device = device_id.clone();
    let heartbeat_api = ApiClient::new(KEY_SERVER_URL.to_string());
    let metrics_url = format!("http://localhost:{}/metrics", metrics_port);
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
    let health_url = format!("http://localhost:{}", port);
    let ui_health_clone = ui_ref.clone();

    let telemetry_api = ApiClient::new(KEY_SERVER_URL.to_string());
    let telemetry_device = device_id.clone();

    let handle = tokio::spawn(async move {
        let mut last_rx: u64 = 0;
        let mut last_tx: u64 = 0;
        let mut tick_count = 0;

        loop {
            sleep(Duration::from_secs(1)).await;
            tick_count += 1;
            if tick_count >= 300 {
                let _ = heartbeat_api.send_heartbeat(&heartbeat_device).await;
                tick_count = 0;
            }

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
                        } else if line.starts_with("cloudflared_tunnel_tx_bytes")
                            && let Some(val) = line.split_whitespace().last()
                        {
                            tx_bytes = val.parse().unwrap_or(last_tx);
                        }
                    }

                    let rx_speed = rx_bytes.saturating_sub(last_rx);
                    let tx_speed = tx_bytes.saturating_sub(last_tx);
                    last_rx = rx_bytes;
                    last_tx = tx_bytes;

                    if let Ok(mut ui) = ui_clone.lock() {
                        ui.draw_live_metrics(rx_bytes, tx_bytes, rx_speed, tx_speed, ping_ms);
                    }
                }
            }

            // Health check every 5 seconds
            if tick_count % 5 == 0
                && let Ok(res) = health_client.get(&health_url).send().await
                && res.status().is_server_error()
                && let Ok(ui) = ui_health_clone.lock()
            {
                ui.error(&format!(
                    "Warning: Local service on port {} returned status {}",
                    port,
                    res.status()
                ));
            }
        }
    });

    match signal::ctrl_c().await {
        Ok(()) => {
            println!("\nShutting down tunnel...");
            let _ = child.kill();
            send_telemetry(
                &telemetry_api,
                "stop",
                &telemetry_device,
                serde_json::json!({}),
            )
            .await;
            let _ = api_client.release_tunnel(&device_id).await;
            handle.abort();
            process::exit(0);
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
            process::exit(1);
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
