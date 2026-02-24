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
    last_heartbeat: Option<i64>,
    created_at: Option<i64>,
    public_url: Option<String>,
    dynamic: Option<i64>,
    cf_tunnel_id: Option<String>,
}

// ── Cloudflare API client ────────────────────────────────────────────────────

/// Creates a named tunnel in Cloudflare and returns (cf_tunnel_id, token).
async fn cf_create_tunnel(
    account_id: &str,
    api_token: &str,
    name: &str,
) -> worker::Result<(String, String)> {
    let url = format!("https://api.cloudflare.com/client/v4/accounts/{account_id}/cfd_tunnel");
    let body = serde_json::json!({ "name": name, "config_src": "cloudflare" });
    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {api_token}"))?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(wasm_bindgen::JsValue::from_str(&body.to_string())));

    let req = worker::Request::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(req).send().await?;
    let json: serde_json::Value = resp.json().await?;

    if !json["success"].as_bool().unwrap_or(false) {
        let err = json["errors"]
            .as_array()
            .and_then(|e| e.first())
            .and_then(|e| e["message"].as_str())
            .unwrap_or("unknown error")
            .to_string();
        return Err(worker::Error::RustError(format!("CF create tunnel: {err}")));
    }

    let result = &json["result"];
    let cf_tunnel_id = result["id"]
        .as_str()
        .ok_or_else(|| worker::Error::RustError("Missing tunnel id".into()))?
        .to_string();
    let token = result["token"]
        .as_str()
        .ok_or_else(|| worker::Error::RustError("Missing tunnel token".into()))?
        .to_string();

    Ok((cf_tunnel_id, token))
}

/// Configures the ingress rules for a cloudflared remote-managed tunnel.
async fn cf_configure_ingress(
    account_id: &str,
    api_token: &str,
    cf_tunnel_id: &str,
    hostname: &str,
    port: u16,
    protocol: &str,
) -> worker::Result<()> {
    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{account_id}/cfd_tunnel/{cf_tunnel_id}/configurations"
    );
    let service = format!("{protocol}://localhost:{port}");
    let body = serde_json::json!({
        "config": {
            "ingress": [
                { "hostname": hostname, "service": service },
                { "service": "http_status:404" }
            ]
        }
    });
    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {api_token}"))?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Put)
        .with_headers(headers)
        .with_body(Some(wasm_bindgen::JsValue::from_str(&body.to_string())));

    let req = worker::Request::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(req).send().await?;
    let json: serde_json::Value = resp.json().await?;

    if !json["success"].as_bool().unwrap_or(false) {
        let err = json["errors"]
            .as_array()
            .and_then(|e| e.first())
            .and_then(|e| e["message"].as_str())
            .unwrap_or("unknown error")
            .to_string();
        return Err(worker::Error::RustError(format!(
            "CF configure ingress: {err}"
        )));
    }
    Ok(())
}

/// Creates a DNS route (CNAME) for the tunnel.
async fn cf_route_dns(
    account_id: &str,
    api_token: &str,
    cf_tunnel_id: &str,
    hostname: &str,
) -> worker::Result<()> {
    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{account_id}/cfd_tunnel/{cf_tunnel_id}/routes"
    );
    let body = serde_json::json!({ "type": "dns", "user_hostname": hostname });
    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {api_token}"))?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(wasm_bindgen::JsValue::from_str(&body.to_string())));

    let req = worker::Request::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(req).send().await?;
    let json: serde_json::Value = resp.json().await?;

    if !json["success"].as_bool().unwrap_or(false) {
        let err = json["errors"]
            .as_array()
            .and_then(|e| e.first())
            .and_then(|e| e["message"].as_str())
            .unwrap_or("unknown error")
            .to_string();
        return Err(worker::Error::RustError(format!("CF route DNS: {err}")));
    }
    Ok(())
}

/// Deletes a tunnel from Cloudflare.
async fn cf_delete_tunnel(
    account_id: &str,
    api_token: &str,
    cf_tunnel_id: &str,
) -> worker::Result<()> {
    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{account_id}/cfd_tunnel/{cf_tunnel_id}"
    );
    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {api_token}"))?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Delete).with_headers(headers);

    let req = worker::Request::new_with_init(&url, &init)?;
    Fetch::Request(req).send().await?;
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct AuthSession {
    id: String,
    auth_token: String,
    status: String,
    created_at: Option<i64>,
}

#[derive(Serialize, Deserialize)]
struct AuthInitResponse {
    session_id: String,
    auth_token: String,
    verify_url: String,
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
    public_url: Option<String>,
}

#[derive(Deserialize)]
pub struct RequestTunnelRequest {
    pub device_id: String,
    pub port: Option<u16>,
    pub protocol: Option<String>,
    pub session_id: Option<String>,
    pub auth_token: Option<String>,
}

#[derive(Deserialize)]
struct DeviceRequest {
    device_id: String,
}

pub const MIN_CLI_VERSION: &str = "0.4.17";
pub const RECOMMENDED_VERSION: &str = "0.4.21";

pub const RUNNING_MESSAGE: &str = "Cloudflare Tunnel CLI Key Server (Rust 🦀) is running.";

pub const BANNED_KEYWORDS: &[&str] = &[
    "bank",
    "login",
    "facebook",
    "google",
    "paypal",
    "stripe",
    "admin",
    "secure",
    "microsoft",
    "office",
    "binance",
    "coinbase",
    "metamask",
    "icloud",
    "netflix",
    "steam",
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

fn json_error(msg: impl Into<String>, status: u16) -> Result<Response> {
    Response::from_json(&serde_json::json!({
        "success": false,
        "error": msg.into()
    }))
    .map(|res| res.with_status(status))
}

async fn check_rate_limit(db: &D1Database, ip: &str) -> Result<bool> {
    let now = (Date::now().as_millis() / 1000) as f64;
    let minute_ago = now - 60.0;

    // Clean up old entries
    if let Err(e) = db
        .prepare("DELETE FROM rate_limits WHERE last_request < ?")
        .bind(&[minute_ago.into()])?
        .run()
        .await
    {
        console_log!("[RateLimit] Cleanup error: {}", e);
    }

    let res: Option<serde_json::Value> = db
        .prepare("SELECT request_count FROM rate_limits WHERE ip = ?")
        .bind(&[ip.into()])?
        .first(None)
        .await
        .map_err(|e| {
            console_log!("[RateLimit] Select error: {}", e);
            e
        })?;

    if let Some(row) = res {
        let count = row
            .get("request_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        if count >= 30 {
            // Increased to 30 requests per minute
            return Ok(false);
        }
        db.prepare(
            "UPDATE rate_limits SET request_count = request_count + 1, last_request = ? WHERE ip = ?",
        )
        .bind(&[now.into(), ip.into()])?
        .run()
        .await
        .map_err(|e| {
            console_log!("[RateLimit] Update error: {}", e);
            e
        })?;
    } else {
        db.prepare("INSERT INTO rate_limits (ip, last_request, request_count) VALUES (?, ?, 1)")
            .bind(&[ip.into(), now.into()])?
            .run()
            .await
            .map_err(|e| {
                console_log!("[RateLimit] Insert error: {}", e);
                e
            })?;
    }

    Ok(true)
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    if let Ok(Some(version)) = req.headers().get("X-CLI-Version") {
        console_log!("[API] CLI Version: {}", version);
    }

    let router = Router::new();

    router
        .get("/", |_, _| handle_index())
        .get("/api/config", |_, _| handle_config_api())
        .post_async("/admin/tunnels", handle_admin_tunnels)
        .post_async("/api/auth/init", handle_auth_init)
        .get_async("/api/auth/check", handle_auth_check)
        .get_async("/api/auth/verify", handle_auth_verify_get)
        .post_async("/api/auth/verify", handle_auth_verify_post)
        .post_async("/api/request", handle_request_tunnel)
        .post_async("/api/heartbeat", handle_heartbeat)
        .post_async("/api/release", handle_release)
        .get_async("/api/stats", handle_stats)
        .post_async("/api/telemetry", handle_telemetry)
        .run(req, env)
        .await
}

fn handle_index() -> Result<Response> {
    Response::ok(RUNNING_MESSAGE)
}

pub fn get_server_config() -> ServerConfigResponse {
    ServerConfigResponse {
        min_cli_version: MIN_CLI_VERSION.to_string(),
        recommended_version: RECOMMENDED_VERSION.to_string(),
    }
}

fn handle_config_api() -> Result<Response> {
    Response::from_json(&get_server_config())
}

pub fn is_authorized_admin(req: &Request, admin_secret: &str) -> bool {
    let auth_header = match req.headers().get("Authorization") {
        Ok(Some(h)) => h,
        _ => return false,
    };
    let token = auth_header.replace("Bearer ", "");
    token == admin_secret
}

async fn handle_admin_tunnels(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let admin_secret = ctx.env.var("ADMIN_SECRET")?.to_string();

    if !is_authorized_admin(&req, &admin_secret) {
        return json_error("Unauthorized", 401);
    }

    let body: AddTunnelRequest = req.json().await?;

    // Keyword filtering for admin too
    if !is_safe_name(&body.name) {
        return json_error("Prohibited keyword in tunnel name", 400);
    }

    let db = ctx.env.d1("DB")?;
    let result = db
        .prepare("INSERT INTO tunnels (id, name, token, status, public_url) VALUES (?, ?, ?, 'AVAILABLE', ?)")
        .bind(&[
            body.id.into(),
            body.name.into(),
            body.token.into(),
            body.public_url.into(),
        ])?
        .run()
        .await;

    match result {
        Ok(_) => Response::from_json(&serde_json::json!({
            "success": true,
            "message": "Tunnel added"
        })),
        Err(e) => json_error(format!("Database error: {e}"), 500),
    }
}

pub fn get_verify_url(url: &Url, session_id: &str) -> String {
    format!(
        "{}://{}/api/auth/verify?s={}",
        url.scheme(),
        url.host_str().unwrap_or("localhost"),
        session_id
    )
}

async fn handle_auth_init(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.env.d1("DB")?;
    let session_id = uuid::Uuid::new_v4().to_string();
    let auth_token = uuid::Uuid::new_v4().to_string();

    db.prepare("INSERT INTO auth_sessions (id, auth_token, status) VALUES (?, ?, 'PENDING')")
        .bind(&[session_id.clone().into(), auth_token.clone().into()])?
        .run()
        .await?;

    let url = req.url()?;
    let verify_url = get_verify_url(&url, &session_id);

    Response::from_json(&AuthInitResponse {
        session_id,
        auth_token,
        verify_url,
    })
}

async fn handle_auth_check(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let session_id = url
        .query_pairs()
        .find(|(k, _)| k == "s")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();
    let auth_token = url
        .query_pairs()
        .find(|(k, _)| k == "t")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();

    let db = ctx.env.d1("DB")?;
    let session: Option<AuthSession> = db
        .prepare("SELECT * FROM auth_sessions WHERE id = ? AND auth_token = ?")
        .bind(&[session_id.into(), auth_token.into()])?
        .first::<AuthSession>(None)
        .await?;

    match session {
        Some(s) => Response::from_json(&serde_json::json!({ "status": s.status })),
        None => json_error("Session not found", 404),
    }
}

pub fn get_verify_html(session_id: &str) -> String {
    format!(
        r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>xpose - Verify Connection</title>
                    <meta name="viewport" content="width=device-width, initial-scale=1">
                    <style>
                        body {{ font-family: sans-serif; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; background: #0f172a; color: white; }}
                        .card {{ background: #1e293b; padding: 2rem; border-radius: 1rem; box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1); text-align: center; max-width: 400px; }}
                        h1 {{ color: #38bdf8; }}
                        button {{ background: #0ea5e9; color: white; border: none; padding: 0.75rem 1.5rem; border-radius: 0.5rem; font-size: 1rem; cursor: pointer; transition: background 0.2s; }}
                        button:hover {{ background: #0284c7; }}
                        .success {{ color: #4ade80; }}
                    </style>
                </head>
                <body>
                    <div class="card">
                        <h1>Confirm Connection</h1>
                        <p>A new CLI instance is requesting access to your tunnels.</p>
                        <form method="POST">
                            <input type="hidden" name="s" value="{}">
                            <button type="submit">Verify Now</button>
                        </form>
                    </div>
                </body>
                </html>
            "#,
        session_id
    )
}

async fn handle_auth_verify_get(req: Request, _: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let session_id = url
        .query_pairs()
        .find(|(k, _)| k == "s")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();

    let html = get_verify_html(&session_id);
    let headers = Headers::new();
    headers.set("Content-Type", "text/html")?;
    Ok(Response::ok(html)?.with_headers(headers))
}

async fn handle_auth_verify_post(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let form = req.form_data().await?;
    let session_id = form
        .get("s")
        .and_then(|e| match e {
            FormEntry::Field(f) => Some(f),
            _ => None,
        })
        .unwrap_or_default();

    let db = ctx.env.d1("DB")?;
    db.prepare("UPDATE auth_sessions SET status = 'VERIFIED' WHERE id = ? AND status = 'PENDING'")
        .bind(&[session_id.into()])?
        .run()
        .await?;

    let html = r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>xpose - Verified</title>
                    <style>
                        body { font-family: sans-serif; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; background: #0f172a; color: white; }
                        .card { background: #1e293b; padding: 2rem; border-radius: 1rem; text-align: center; }
                        h1 { color: #4ade80; }
                    </style>
                </head>
                <body>
                    <div class="card">
                        <h1>✓ Verified Successfully</h1>
                        <p>You can go back to your terminal now.</p>
                    </div>
                </body>
                </html>
            "#;
    let headers = Headers::new();
    headers.set("Content-Type", "text/html")?;
    Ok(Response::ok(html)?.with_headers(headers))
}

pub fn validate_tunnel_request(body: &RequestTunnelRequest) -> Result<(), (String, u16)> {
    if let Some(p) = body.port {
        if !ALLOWED_PORTS.contains(&p) {
            return Err((format!("Port {p} is restricted for security reasons."), 403));
        }
    }
    Ok(())
}

async fn handle_request_tunnel(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let ip = req
        .headers()
        .get("cf-connecting-ip")?
        .unwrap_or_else(|| "unknown".to_string());
    let db = ctx.env.d1("DB")?;

    match check_rate_limit(&db, &ip).await {
        Ok(true) => {}
        Ok(false) => return json_error("Too many requests. Please wait a minute.", 429),
        Err(e) => {
            console_log!("[RequestTunnel] Rate limit check failed: {}", e);
            return json_error(format!("Internal check failed: {}", e), 500);
        }
    }

    let body: RequestTunnelRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => return json_error(format!("Invalid JSON: {}", e), 400),
    };

    if let Err((msg, status)) = validate_tunnel_request(&body) {
        return json_error(msg, status);
    }

    let now = (Date::now().as_millis() / 1000) as f64;

    // 1. Check if device already has a busy tunnel
    let existing: Option<Tunnel> = db
        .prepare("SELECT * FROM tunnels WHERE status = 'BUSY' AND device_id = ?")
        .bind(&[body.device_id.clone().into()])?
        .first::<Tunnel>(None)
        .await
        .map_err(|e| {
            console_log!("[RequestTunnel] Existing lookup failed: {}", e);
            e
        })?;

    if let Some(t) = existing {
        db.prepare("UPDATE tunnels SET last_heartbeat = ?, port = ?, protocol = ? WHERE id = ?")
            .bind(&[
                now.into(),
                body.port.unwrap_or(t.port.unwrap_or(0)).into(),
                body.protocol
                    .unwrap_or(t.protocol.unwrap_or_else(|| "tcp".to_string()))
                    .into(),
                t.id.clone().into(),
            ])?
            .run()
            .await
            .map_err(|e| {
                console_log!("[RequestTunnel] Reconnect update failed: {}", e);
                e
            })?;

        return Response::from_json(&serde_json::json!({
            "success": true,
            "message": "Reconnected",
            "tunnel": { "id": t.id, "name": t.name, "token": t.token, "public_url": t.public_url }
        }));
    }

    // 2. Find available tunnel
    let available: Option<Tunnel> = db
        .prepare("SELECT * FROM tunnels WHERE status = 'AVAILABLE' LIMIT 1")
        .first::<Tunnel>(None)
        .await
        .map_err(|e| {
            console_log!("[RequestTunnel] Available lookup failed: {}", e);
            e
        })?;

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
                .await
                .map_err(|e| {
                    console_log!("[RequestTunnel] Allocation update failed: {}", e);
                    e
                })?;

            let changes = res.meta()?.and_then(|m| m.changes).unwrap_or(0);
            if changes > 0 {
                Response::from_json(&serde_json::json!({
                    "success": true,
                    "tunnel": { "id": t.id, "name": t.name, "token": t.token, "public_url": t.public_url }
                }))
            } else {
                json_error("Collision, please retry", 409)
            }
        }
        None => {
            // No pool tunnel available — try dynamic provisioning via Cloudflare API
            let api_token = ctx.env.var("CF_API_TOKEN").map(|v| v.to_string()).ok();
            let account_id = ctx.env.var("CF_ACCOUNT_ID").map(|v| v.to_string()).ok();
            let tunnel_domain = ctx.env.var("CF_TUNNEL_DOMAIN").map(|v| v.to_string()).ok();

            match (api_token, account_id, tunnel_domain) {
                (Some(api_token), Some(account_id), Some(tunnel_domain)) => {
                    let port = body.port.unwrap_or(80);
                    let protocol = body.protocol.unwrap_or_else(|| "tcp".to_string());
                    let short_id = uuid::Uuid::new_v4()
                        .to_string()
                        .chars()
                        .take(8)
                        .collect::<String>();
                    let tunnel_name = format!("xpose-{short_id}");
                    let hostname = format!("{short_id}.{tunnel_domain}");
                    let public_url = format!("tcp://{hostname}");

                    // Create tunnel
                    let (cf_tunnel_id, token) =
                        match cf_create_tunnel(&account_id, &api_token, &tunnel_name).await {
                            Ok(r) => r,
                            Err(e) => {
                                console_log!("[Dynamic] Create tunnel failed: {}", e);
                                return json_error(format!("Failed to create tunnel: {e}"), 503);
                            }
                        };

                    // Configure ingress rules
                    if let Err(e) = cf_configure_ingress(
                        &account_id,
                        &api_token,
                        &cf_tunnel_id,
                        &hostname,
                        port,
                        &protocol,
                    )
                    .await
                    {
                        console_log!("[Dynamic] Configure ingress failed: {}", e);
                        let _ = cf_delete_tunnel(&account_id, &api_token, &cf_tunnel_id).await;
                        return json_error(format!("Failed to configure tunnel: {e}"), 503);
                    }

                    // Route DNS
                    if let Err(e) =
                        cf_route_dns(&account_id, &api_token, &cf_tunnel_id, &hostname).await
                    {
                        console_log!("[Dynamic] Route DNS failed: {}", e);
                        let _ = cf_delete_tunnel(&account_id, &api_token, &cf_tunnel_id).await;
                        return json_error(format!("Failed to route DNS: {e}"), 503);
                    }

                    // Persist to DB
                    let tunnel_id = uuid::Uuid::new_v4().to_string();
                    let res = db
                        .prepare("INSERT INTO tunnels (id, name, token, status, device_id, port, protocol, last_heartbeat, created_at, public_url, dynamic, cf_tunnel_id) VALUES (?, ?, ?, 'BUSY', ?, ?, ?, ?, ?, ?, 1, ?)")
                        .bind(&[
                            tunnel_id.clone().into(),
                            tunnel_name.clone().into(),
                            token.clone().into(),
                            body.device_id.into(),
                            port.into(),
                            protocol.into(),
                            now.into(),
                            now.into(),
                            public_url.clone().into(),
                            cf_tunnel_id.into(),
                        ])?
                        .run()
                        .await;

                    match res {
                        Ok(_) => Response::from_json(&serde_json::json!({
                            "success": true,
                            "tunnel": {
                                "id": tunnel_id,
                                "name": tunnel_name,
                                "token": token,
                                "public_url": public_url
                            }
                        })),
                        Err(e) => json_error(format!("DB error: {e}"), 500),
                    }
                }
                _ => json_error("No tunnels available", 503),
            }
        }
    }
}

async fn handle_heartbeat(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: DeviceRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => return json_error(format!("Invalid JSON: {}", e), 400),
    };
    let db = ctx.env.d1("DB")?;
    let now = (Date::now().as_millis() / 1000) as f64;

    let res = db
        .prepare("UPDATE tunnels SET last_heartbeat = ? WHERE device_id = ? AND status = 'BUSY'")
        .bind(&[now.into(), body.device_id.into()])?
        .run()
        .await
        .map_err(|e| {
            console_log!("[Heartbeat] Update failed: {}", e);
            e
        })?;

    let changes = res.meta()?.and_then(|m| m.changes).unwrap_or(0);
    if changes > 0 {
        Response::from_json(&serde_json::json!({"success": true, "timestamp": now}))
    } else {
        json_error("No active session", 404)
    }
}

async fn handle_release(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: DeviceRequest = req.json().await?;
    let db = ctx.env.d1("DB")?;

    // Look up the tunnel before releasing to check if it was dynamic
    let tunnel: Option<Tunnel> = db
        .prepare("SELECT * FROM tunnels WHERE device_id = ? AND status = 'BUSY'")
        .bind(&[body.device_id.clone().into()])?
        .first::<Tunnel>(None)
        .await
        .unwrap_or(None);

    if let Some(ref t) = tunnel {
        if t.dynamic.unwrap_or(0) == 1 {
            // Delete the dynamic tunnel from DB entirely
            db.prepare("DELETE FROM tunnels WHERE id = ?")
                .bind(&[t.id.clone().into()])?
                .run()
                .await?;

            // Clean up from Cloudflare API asynchronously (best-effort)
            let api_token = ctx.env.var("CF_API_TOKEN").map(|v| v.to_string()).ok();
            let account_id = ctx.env.var("CF_ACCOUNT_ID").map(|v| v.to_string()).ok();
            if let (Some(api_token), Some(account_id), Some(cf_tunnel_id)) =
                (api_token, account_id, t.cf_tunnel_id.clone())
            {
                if let Err(e) = cf_delete_tunnel(&account_id, &api_token, &cf_tunnel_id).await {
                    console_log!("[Release] CF delete tunnel failed (non-fatal): {}", e);
                }
            }
        } else {
            // Static pool tunnel: mark as available
            db.prepare("UPDATE tunnels SET status = 'AVAILABLE', device_id = NULL, port = NULL, last_heartbeat = NULL WHERE id = ?")
                .bind(&[t.id.clone().into()])?
                .run()
                .await?;
        }
    }

    Response::from_json(&serde_json::json!({"success": true}))
}

pub fn calculate_stats(rows: Vec<serde_json::Value>) -> (u64, u64, u64) {
    let mut busy = 0;
    let mut available = 0;

    for row in rows {
        let status = row.get("status").and_then(|v| v.as_str()).unwrap_or("");
        let count = row.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
        if status == "BUSY" {
            busy = count;
        } else if status == "AVAILABLE" {
            available = count;
        }
    }

    (busy, available, busy + available)
}

async fn handle_stats(_: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.env.d1("DB")?;
    let res = db
        .prepare("SELECT status, COUNT(*) as count FROM tunnels GROUP BY status")
        .all()
        .await?;

    let rows: Vec<serde_json::Value> = res.results()?;
    let (busy, available, total) = calculate_stats(rows);

    Response::from_json(&serde_json::json!({
        "total": total,
        "busy": busy,
        "available": available
    }))
}

async fn handle_telemetry(mut req: Request, _: RouteContext<()>) -> Result<Response> {
    let body: serde_json::Value = req.json().await?;
    console_log!("[Telemetry] Received report: {:?}", body);
    Response::ok("Report received")
}

#[event(scheduled)]
pub async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    let db = env.d1("DB").expect("D1 Database not found");
    let sixty_mins_ago = (Date::now().as_millis() / 1000) as f64 - 3600.0;

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
            created_at: None,
            public_url: None,
            dynamic: None,
            cf_tunnel_id: None,
        };
        let json = serde_json::to_string(&tunnel).unwrap();
        assert!(json.contains("\"id\":\"t1\""));
        assert!(json.contains("\"status\":\"AVAILABLE\""));
    }

    #[test]
    fn test_constants() {
        assert_eq!(MIN_CLI_VERSION, "0.4.17");
        assert!(RUNNING_MESSAGE.contains("Key Server"));
    }

    #[test]
    fn test_api_responses() {
        let config = ServerConfigResponse {
            min_cli_version: MIN_CLI_VERSION.to_string(),
            recommended_version: RECOMMENDED_VERSION.to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("min_cli_version"));
    }

    #[test]
    fn test_banned_keywords() {
        assert!(!is_safe_name("bank-login"));
        assert!(!is_safe_name("Google-Auth"));
        assert!(!is_safe_name("PAYPAL-VERIFY"));
        assert!(!is_safe_name("Microsoft-Office-365"));
        assert!(!is_safe_name("binance-wallet"));
        assert!(!is_safe_name("COINBASE-login"));
        assert!(!is_safe_name("Metamask-Access"));
        assert!(!is_safe_name("icloud-bypass"));
        assert!(!is_safe_name("netflix-free"));
        assert!(!is_safe_name("steam-gift-card"));
        assert!(is_safe_name("my-app-tunnel"));
        assert!(is_safe_name("dev-box"));
    }

    #[test]
    fn test_is_port_allowed() {
        assert!(is_port_allowed(80));
        assert!(is_port_allowed(443));
        assert!(is_port_allowed(3000));
        assert!(is_port_allowed(8080));
        assert!(is_port_allowed(9000));
        // Test defaults
        assert!(!is_port_allowed(25));
        assert!(!is_port_allowed(3306));
    }

    #[test]
    fn test_is_safe_name() {
        assert!(is_safe_name("my-tunnel"));
        assert!(!is_safe_name("my-admin-panel"));
        assert!(!is_safe_name("bank-secure"));
    }

    #[test]
    fn test_dto_serialization() {
        let resp = AuthInitResponse {
            session_id: "s1".to_string(),
            auth_token: "t1".to_string(),
            verify_url: "http://test".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"session_id\":\"s1\""));
    }

    #[test]
    fn test_server_config_response() {
        let resp = ServerConfigResponse {
            min_cli_version: "0.1.0".to_string(),
            recommended_version: "0.2.0".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"min_cli_version\":\"0.1.0\""));
    }

    #[test]
    fn test_get_server_config_logic() {
        let config = get_server_config();
        assert_eq!(config.min_cli_version, MIN_CLI_VERSION);
    }

    /*
       Note: is_authorized_admin test is hard without a mock Request
       from the worker crate, so we skip it for now unless we add
       more infra. We focus on pure functions.
    */

    #[test]
    fn test_html_templates() {
        let sid = "test-sid";
        let html = get_verify_html(sid);
        assert!(html.contains(sid));
        assert!(html.contains("Confirm Connection"));
    }
    #[test]
    fn test_get_verify_url() {
        let url = Url::parse("https://api.xpose.cloud/api/auth/init").unwrap();
        let sid = "session-123";
        let verify_url = get_verify_url(&url, sid);
        assert_eq!(
            verify_url,
            "https://api.xpose.cloud/api/auth/verify?s=session-123"
        );
    }

    #[test]
    fn test_validate_tunnel_request() {
        // Allowed port
        let body = RequestTunnelRequest {
            device_id: "d1".to_string(),
            port: Some(80),
            protocol: None,
            session_id: None,
            auth_token: None,
        };
        assert!(validate_tunnel_request(&body).is_ok());

        // Forbidden port
        let body = RequestTunnelRequest {
            device_id: "d1".to_string(),
            port: Some(25),
            protocol: None,
            session_id: None,
            auth_token: None,
        };
        let res = validate_tunnel_request(&body);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().1, 403);

        // No port (should be allowed)
        let body = RequestTunnelRequest {
            device_id: "d1".to_string(),
            port: None,
            protocol: None,
            session_id: None,
            auth_token: None,
        };
        assert!(validate_tunnel_request(&body).is_ok());
    }
    #[test]
    fn test_calculate_stats() {
        let rows = vec![
            serde_json::json!({"status": "BUSY", "count": 5}),
            serde_json::json!({"status": "AVAILABLE", "count": 10}),
        ];
        let (busy, available, total) = calculate_stats(rows);
        assert_eq!(busy, 5);
        assert_eq!(available, 10);
        assert_eq!(total, 15);

        // Empty rows
        let (busy, available, total) = calculate_stats(vec![]);
        assert_eq!(busy, 0);
        assert_eq!(available, 0);
        assert_eq!(total, 0);
    }

    #[test]
    fn test_is_safe_name_more() {
        assert!(!is_safe_name("my-bank-app"));
        assert!(!is_safe_name("safe-login-here"));
        assert!(is_safe_name("my-wonderful-app"));
        assert!(!is_safe_name("microsoft-updates"));
        assert!(is_safe_name("rust-cli-tool"));
    }

    #[test]
    fn test_version_logic() {
        // Simple version compatibility check simulation
        let current = "0.4.18";
        let min = MIN_CLI_VERSION;
        assert!(current >= min);
    }
}
