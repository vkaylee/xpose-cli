use reqwest::Client;
use serde::{Deserialize, Serialize};
use log::{info, error};
use std::time::Duration;

#[derive(Serialize)]
pub struct RequestPayload {
    pub device_id: String,
    pub port: Option<u16>,
    pub protocol: Option<String>,
}

#[derive(Serialize)]
pub struct HeartbeatPayload {
    pub device_id: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct TunnelInfo {
    pub id: String,
    pub name: String,
    pub token: String,
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub min_cli_version: String,
    pub recommended_version: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct ApiResponse {
    pub success: Option<bool>,
    pub message: Option<String>,
    pub tunnel: Option<TunnelInfo>,
    pub error: Option<String>,
}

pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            base_url,
        }
    }

    pub async fn get_config(&self) -> Result<ServerConfig, String> {
        let url = format!("{}/api/config", self.base_url);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                error!("Config fetch failed: {}", e);
                e.to_string()
            })?;
        let data: ServerConfig = res.json().await.map_err(|e| e.to_string())?;
        info!("Fetched server config: min_cli_version={}", data.min_cli_version);
        Ok(data)
    }

    pub async fn request_tunnel(
        &self,
        device_id: &str,
        port: Option<u16>,
        protocol: Option<&str>,
    ) -> Result<TunnelInfo, String> {
        let url = format!("{}/api/request", self.base_url);
        let payload = RequestPayload {
            device_id: device_id.to_string(),
            port,
            protocol: protocol.map(|s| s.to_string()),
        };

        let res = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let data: ApiResponse = res.json().await.map_err(|e| e.to_string())?;

        if status.is_success() {
            if let Some(tunnel) = data.tunnel {
                Ok(tunnel)
            } else {
                Err("Invalid response format from server".to_string())
            }
        } else {
            Err(data
                .error
                .unwrap_or_else(|| "Unknown server error".to_string()))
        }
    }

    pub async fn send_heartbeat(&self, device_id: &str) -> Result<(), String> {
        let url = format!("{}/api/heartbeat", self.base_url);
        let payload = HeartbeatPayload {
            device_id: device_id.to_string(),
        };

        let res = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if res.status().is_success() {
            Ok(())
        } else {
            Err("Heartbeat failed".to_string())
        }
    }

    pub async fn release_tunnel(&self, device_id: &str) -> Result<(), String> {
        let url = format!("{}/api/release", self.base_url);
        let payload = HeartbeatPayload {
            device_id: device_id.to_string(),
        };

        let res = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if res.status().is_success() {
            Ok(())
        } else {
            Err("Failed to release tunnel".to_string())
        }
    }

    pub async fn post_telemetry(&self, payload: serde_json::Value) -> Result<(), String> {
        let url = format!("{}/api/telemetry", self.base_url);
        let _ = self.client.post(&url).json(&payload).send().await;
        Ok(())
    }
}
