use log::{error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
pub struct RequestPayload {
    pub device_id: String,
    pub port: Option<u16>,
    pub protocol: Option<String>,
    pub session_id: Option<String>,
    pub auth_token: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct AuthInitResponse {
    pub session_id: String,
    pub auth_token: String,
    pub verify_url: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthStatusResponse {
    pub status: String,
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
#[derive(Deserialize, Debug, Clone, Default)]
pub struct GlobalStats {
    pub total: u64,
    pub busy: u64,
    pub available: u64,
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
        let res = self.client.get(&url).send().await.map_err(|e| {
            error!("Config fetch failed: {e}");
            e.to_string()
        })?;
        let data: ServerConfig = res.json().await.map_err(|e| e.to_string())?;
        info!(
            "Fetched server config: min_cli_version={}",
            data.min_cli_version
        );
        Ok(data)
    }

    pub async fn request_tunnel(
        &self,
        device_id: &str,
        port: Option<u16>,
        protocol: Option<&str>,
        session_id: Option<String>,
        auth_token: Option<String>,
    ) -> Result<TunnelInfo, String> {
        let url = format!("{}/api/request", self.base_url);
        let payload = RequestPayload {
            device_id: device_id.to_string(),
            port,
            protocol: protocol.map(|s| s.to_string()),
            session_id,
            auth_token,
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

    pub async fn get_global_stats(&self) -> Result<GlobalStats, String> {
        let url = format!("{}/api/stats", self.base_url);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if res.status().is_success() {
            let data: GlobalStats = res.json().await.map_err(|e| e.to_string())?;
            Ok(data)
        } else {
            Err("Failed to fetch global stats".to_string())
        }
    }

    pub async fn post_telemetry(&self, payload: serde_json::Value) -> Result<(), String> {
        let url = format!("{}/api/telemetry", self.base_url);
        let _ = self.client.post(&url).json(&payload).send().await;
        Ok(())
    }

    pub async fn init_auth(&self) -> Result<AuthInitResponse, String> {
        let url = format!("{}/api/auth/init", self.base_url);
        let res = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if res.status().is_success() {
            let data: AuthInitResponse = res.json().await.map_err(|e| e.to_string())?;
            Ok(data)
        } else {
            Err("Failed to initialize authentication".to_string())
        }
    }

    pub async fn check_auth_status(
        &self,
        session_id: &str,
        auth_token: &str,
    ) -> Result<String, String> {
        let url = format!(
            "{}/api/auth/check?s={}&t={}",
            self.base_url, session_id, auth_token
        );
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if res.status().is_success() {
            let data: AuthStatusResponse = res.json().await.map_err(|e| e.to_string())?;
            Ok(data.status)
        } else {
            Err("Failed to check authentication status".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_get_config_success() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let mock = server
            .mock("GET", "/api/config")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"min_cli_version": "0.1.0", "recommended_version": "0.2.0"}"#)
            .create_async()
            .await;

        let client = ApiClient::new(url);
        let config = client.get_config().await.unwrap();

        assert_eq!(config.min_cli_version, "0.1.0");
        assert_eq!(config.recommended_version, "0.2.0");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_request_tunnel_success() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let mock = server
            .mock("POST", "/api/request")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"success": true, "tunnel": {"id": "t1", "name": "n1", "token": "tok1"}}"#,
            )
            .create_async()
            .await;

        let client = ApiClient::new(url);
        let info = client
            .request_tunnel("dev1", Some(3000), Some("tcp"), None, None)
            .await
            .unwrap();

        assert_eq!(info.id, "t1");
        assert_eq!(info.name, "n1");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_send_heartbeat_success() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let mock = server
            .mock("POST", "/api/heartbeat")
            .with_status(200)
            .create_async()
            .await;

        let client = ApiClient::new(url);
        let res = client.send_heartbeat("dev1").await;

        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_release_tunnel_success() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let mock = server
            .mock("POST", "/api/release")
            .with_status(200)
            .create_async()
            .await;

        let client = ApiClient::new(url);
        let res = client.release_tunnel("dev1").await;

        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_config_server_error() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let _mock = server
            .mock("GET", "/api/config")
            .with_status(500)
            .create_async()
            .await;

        let client = ApiClient::new(url);
        let res = client.get_config().await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_request_tunnel_malformed_json() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let _mock = server
            .mock("POST", "/api/request")
            .with_status(200)
            .with_body("invalid json")
            .create_async()
            .await;

        let client = ApiClient::new(url);
        let res = client.request_tunnel("dev1", None, None, None, None).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_request_tunnel_error_message() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let _mock = server
            .mock("POST", "/api/request")
            .with_status(403)
            .with_body(r#"{"success": false, "error": "Custom error message"}"#)
            .create_async()
            .await;

        let client = ApiClient::new(url);
        let res = client.request_tunnel("dev1", None, None, None, None).await;
        assert!(res.is_err());
        assert_eq!(res.err().unwrap(), "Custom error message");
    }

    #[tokio::test]
    async fn test_get_global_stats_success() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let mock = server
            .mock("GET", "/api/stats")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"total": 10, "busy": 3, "available": 7}"#)
            .create_async()
            .await;

        let client = ApiClient::new(url);
        let stats = client.get_global_stats().await.unwrap();

        assert_eq!(stats.total, 10);
        assert_eq!(stats.busy, 3);
        assert_eq!(stats.available, 7);
        mock.assert_async().await;
    }
}
