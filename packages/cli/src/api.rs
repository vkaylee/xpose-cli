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

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(mut base_url: String) -> Self {
        if base_url.ends_with('/') {
            base_url.pop();
        }
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
        let text = res.text().await.map_err(|e| e.to_string())?;

        let data: ApiResponse = match serde_json::from_str(&text) {
            Ok(d) => d,
            Err(_) => {
                if !status.is_success() {
                    return Err(text);
                }
                return Err("Failed to parse server response".to_string());
            }
        };

        if status.is_success() {
            if let Some(tunnel) = data.tunnel {
                Ok(tunnel)
            } else {
                Err("Invalid response format from server".to_string())
            }
        } else {
            let err_msg = data
                .error
                .unwrap_or_else(|| format!("Unknown server error (Status: {})", status));
            error!("Server error: {} - Body: {}", status, text);
            Err(err_msg)
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
    async fn test_url_sanitization() {
        let client = ApiClient::new("https://example.com/".to_string());
        assert_eq!(client.base_url, "https://example.com");

        let client2 = ApiClient::new("https://example.com".to_string());
        assert_eq!(client2.base_url, "https://example.com");
    }

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
    async fn test_init_auth_success() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let mock = server
            .mock("POST", "/api/auth/init")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"session_id": "s1", "auth_token": "tok1", "verify_url": "http://v"}"#)
            .create_async()
            .await;

        let client = ApiClient::new(url);
        let res = client.init_auth().await.unwrap();

        assert_eq!(res.session_id, "s1");
        assert_eq!(res.auth_token, "tok1");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_check_auth_status_success() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let mock = server
            .mock("GET", "/api/auth/check?s=s1&t=tok1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "VERIFIED"}"#)
            .create_async()
            .await;

        let client = ApiClient::new(url);
        let status = client.check_auth_status("s1", "tok1").await.unwrap();

        assert_eq!(status, "VERIFIED");
        mock.assert_async().await;
    }
}
