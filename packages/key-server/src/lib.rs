use serde::{Deserialize, Serialize};
use worker::*;

#[derive(Serialize, Deserialize, Debug)]
struct Tunnel {
    id: String,
    name: String,
    token: String,
    status: String,
    device_id: Option<String>,
    port: Option<u16>,
    protocol: Option<String>,
    last_heartbeat: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ServerConfigResponse {
    pub min_cli_version: String,
    pub recommended_version: String,
}

#[derive(Deserialize)]
struct AddTunnelRequest {
    id: String,
    name: String,
    token: String,
}

#[derive(Deserialize)]
struct RequestTunnelRequest {
    device_id: String,
    port: Option<u16>,
    protocol: Option<String>,
}

#[derive(Deserialize)]
struct DeviceRequest {
    device_id: String,
}

pub const MIN_CLI_VERSION: &str = "0.1.0";
pub const RECOMMENDED_VERSION: &str = "0.1.0";

pub const RUNNING_MESSAGE: &str = "Cloudflare Tunnel CLI Key Server (Rust 🦀) is running.";

pub const BANNED_KEYWORDS: &[&str] = &[
    "bank", "login", "facebook", "google", "paypal", "stripe", "admin", "secure",
];
pub const ALLOWED_PORTS: &[u16] = &[
    80, 443, 3000, 3001, 5000, 5173, 8000, 8008, 8080, 8443, 9000,
];

pub fn is_port_allowed(port: u16) -> bool {
    ALLOWED_PORTS.contains(&port)
}


fn is_safe_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    for kw in BANNED_KEYWORDS {
        if lower.contains(kw) {
            return false;
        }
    }
    true
}

async fn check_rate_limit(db: &D1Database, ip: &str) -> Result<bool> {
    let now = (Date::now().as_millis() / 1000) as i64;
    let minute_ago = now - 60;

    // Clean up old entries (simple way: delete if last_request is old)
    let _ = db
        .prepare("DELETE FROM rate_limits WHERE last_request < ?")
        .bind(&[minute_ago.into()])?
        .run()
        .await;

    let res: Option<serde_json::Value> = db
        .prepare("SELECT request_count FROM rate_limits WHERE ip = ?")
        .bind(&[ip.into()])?
        .first(None)
        .await?;

    if let Some(row) = res {
        let count = row
            .get("request_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        if count >= 20 {
            // 20 requests per minute
            return Ok(false);
        }
        db.prepare("UPDATE rate_limits SET request_count = request_count + 1, last_request = ? WHERE ip = ?")
            .bind(&[now.into(), ip.into()])?
            .run()
            .await?;
    } else {
        db.prepare("INSERT INTO rate_limits (ip, last_request, request_count) VALUES (?, ?, 1)")
            .bind(&[ip.into(), now.into()])?
            .run()
            .await?;
    }

    Ok(true)
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let router = Router::new();

    router
        .get("/", |_, _| Response::ok(RUNNING_MESSAGE))
        .get("/api/config", |_, _| {
            Response::from_json(&serde_json::json!({
                "min_cli_version": MIN_CLI_VERSION,
                "recommended_version": RECOMMENDED_VERSION
            }))
        })
        .post_async("/admin/tunnels", |mut req, ctx| async move {
            let admin_secret = ctx.env.var("ADMIN_SECRET")?.to_string();
            let auth_header = req.headers().get("Authorization")?.unwrap_or_default();
            let token = auth_header.replace("Bearer ", "");

            if token != admin_secret {
                return Response::error("Unauthorized", 401);
            }

            let body: AddTunnelRequest = req.json().await?;

            // Keyword filtering for admin too
            if !is_safe_name(&body.name) {
                return Response::error("Prohibited keyword in tunnel name", 400);
            }

            let db = ctx.env.d1("DB")?;
            let result = db.prepare("INSERT INTO tunnels (id, name, token, status) VALUES (?, ?, ?, 'AVAILABLE')")
                .bind(&[body.id.into(), body.name.into(), body.token.into()])?
                .run()
                .await;

            match result {
                Ok(_) => Response::from_json(&serde_json::json!({"success": true, "message": "Tunnel added"})),
                Err(e) => Response::error(format!("Database error: {e}"), 500),
            }
        })
        .post_async("/api/request", |mut req, ctx| async move {
            let ip = req.headers().get("cf-connecting-ip")?.unwrap_or_else(|| "unknown".to_string());
            let db = ctx.env.d1("DB")?;

            if !check_rate_limit(&db, &ip).await? {
                return Response::error("Too many requests. Please wait a minute.", 429);
            }

            let body: RequestTunnelRequest = req.json().await?;

            // Port restriction
            if let Some(p) = body.port {
                if !ALLOWED_PORTS.contains(&p) {
                    return Response::error(format!("Port {p} is restricted for security reasons."), 403);
                }
            }

            let db = ctx.env.d1("DB")?;
            let now = (Date::now().as_millis() / 1000) as f64;

            // 1. Check if device already has a busy tunnel
            let existing: Option<Tunnel> = db.prepare("SELECT * FROM tunnels WHERE status = 'BUSY' AND device_id = ?")
                .bind(&[body.device_id.clone().into()])?
                .first::<Tunnel>(None)
                .await?;

            if let Some(t) = existing {
                db.prepare("UPDATE tunnels SET last_heartbeat = ?, port = ?, protocol = ? WHERE id = ?")
                    .bind(&[
                        now.into(),
                        body.port.unwrap_or(t.port.unwrap_or(0)).into(),
                        body.protocol.unwrap_or(t.protocol.unwrap_or_else(|| "tcp".to_string())).into(),
                        t.id.clone().into()
                    ])?
                    .run()
                    .await?;

                return Response::from_json(&serde_json::json!({
                    "success": true,
                    "message": "Reconnected",
                    "tunnel": { "id": t.id, "name": t.name, "token": t.token }
                }));
            }

            // 2. Find available tunnel
            let available: Option<Tunnel> = db.prepare("SELECT * FROM tunnels WHERE status = 'AVAILABLE' LIMIT 1")
                .first::<Tunnel>(None)
                .await?;

            match available {
                Some(t) => {
                    let res = db.prepare("UPDATE tunnels SET status = 'BUSY', device_id = ?, port = ?, protocol = ?, last_heartbeat = ?, created_at = ? WHERE id = ? AND status = 'AVAILABLE'")
                        .bind(&[
                            body.device_id.into(),
                            body.port.into(),
                            body.protocol.unwrap_or_else(|| "tcp".to_string()).into(),
                            now.into(),
                            now.into(),
                            t.id.clone().into()
                        ])?
                        .run()
                        .await?;

                    let changes = res.meta()?.and_then(|m| m.changes).unwrap_or(0);
                    if changes > 0 {
                        Response::from_json(&serde_json::json!({
                            "success": true,
                            "tunnel": { "id": t.id, "name": t.name, "token": t.token }
                        }))
                    } else {
                        Response::error("Collision, please retry", 409)
                    }
                }
                None => Response::error("No tunnels available", 503)
            }
        })
        .post_async("/api/heartbeat", |mut req, ctx| async move {
            let body: DeviceRequest = req.json().await?;
            let db = ctx.env.d1("DB")?;
            let now = (Date::now().as_millis() / 1000) as f64;

            let res = db.prepare("UPDATE tunnels SET last_heartbeat = ? WHERE device_id = ? AND status = 'BUSY'")
                .bind(&[now.into(), body.device_id.into()])?
                .run()
                .await?;

            let changes = res.meta()?.and_then(|m| m.changes).unwrap_or(0);
            if changes > 0 {
                Response::from_json(&serde_json::json!({"success": true, "timestamp": now}))
            } else {
                Response::error("No active session", 404)
            }
        })
        .post_async("/api/release", |mut req, ctx| async move {
            let body: DeviceRequest = req.json().await?;
            let db = ctx.env.d1("DB")?;

            db.prepare("UPDATE tunnels SET status = 'AVAILABLE', device_id = NULL, port = NULL, last_heartbeat = NULL WHERE device_id = ? AND status = 'BUSY'")
                .bind(&[body.device_id.into()])?
                .run()
                .await?;

            Response::from_json(&serde_json::json!({"success": true}))
        })
        .post_async("/api/telemetry", |mut req, _ctx| async move {
            let body: serde_json::Value = req.json().await?;
            console_log!("[Telemetry] Received report: {:?}", body);
            Response::ok("Report received")
        })
        .run(req, env)
        .await
}

#[event(scheduled)]
pub async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    let db = env.d1("DB").expect("D1 Database not found");
    let sixty_mins_ago = (Date::now().as_millis() as f64 / 1000.0) - 3600.0;

    let res = db.prepare("UPDATE tunnels SET status = 'AVAILABLE', device_id = NULL, port = NULL, last_heartbeat = NULL WHERE status = 'BUSY' AND last_heartbeat < ?")
        .bind(&[sixty_mins_ago.into()])
        .expect("Failed to bind params")
        .run()
        .await;

    if let Ok(r) = res {
        let changes = r.meta().ok().flatten().and_then(|m| m.changes).unwrap_or(0);
        console_log!("[Cron] Cleaned up {} inactive tunnels", changes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_safe_name_edge_cases() {
        assert!(is_safe_name("")); // Empty name is technically safe from keywords
        assert!(is_safe_name("   ")); // Spaces are safe
        assert!(!is_safe_name("ADMIN")); // Case insensitive
        assert!(!is_safe_name("  bank  ")); // Leading/trailing spaces
        assert!(!is_safe_name("mybankapp")); // Substring
    }

    #[test]
    fn test_is_port_allowed_edge_cases() {
        assert!(!is_port_allowed(0));
        assert!(!is_port_allowed(65535));
        for &port in ALLOWED_PORTS {
            assert!(is_port_allowed(port));
        }
    }

    #[test]
    fn test_is_port_allowed() {
        assert!(is_port_allowed(80));
        assert!(is_port_allowed(3000));
        assert!(!is_port_allowed(22));
        assert!(!is_port_allowed(3306));
    }

    #[test]
    fn test_serialization() {
        let tunnel = Tunnel {
            id: "t1".to_string(),
            name: "n1".to_string(),
            token: "tok1".to_string(),
            status: "AVAILABLE".to_string(),
            device_id: None,
            port: None,
            protocol: None,
            last_heartbeat: None,
        };
        let json = serde_json::to_string(&tunnel).unwrap();
        assert!(json.contains("\"id\":\"t1\""));
        assert!(json.contains("\"status\":\"AVAILABLE\""));
    }

    #[test]
    fn test_constants() {
        assert_eq!(MIN_CLI_VERSION, "0.1.0");
        assert!(RUNNING_MESSAGE.contains("Key Server"));
    }
}
