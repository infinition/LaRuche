//! Runtime settings endpoints (channel/notify/permission/curateur config, secrets vault HTTP layer, MCP server RPC) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use axum::http::StatusCode;
use std::sync::Arc;

/// GET /api/config/channels - read channel configuration. Bot tokens are secrets, so they
/// are only returned to an authenticated admin; other callers get them masked.
pub(crate) async fn api_get_channels_config(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Json<serde_json::Value> {
    let path = std::path::Path::new("channels-config.json");
    let mut config = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .unwrap_or_else(|| {
            serde_json::json!({
                "telegram": {"bot_token": "", "allowed_chats": "", "enabled": false},
                "discord": {"bot_token": "", "allowed_channels": "", "enabled": false},
                "slack": {"bot_token": "", "app_token": "", "enabled": false},
            })
        });
    if !auth_user::require_admin(&state, &headers).await {
        for ch in ["telegram", "discord", "slack"] {
            for field in ["bot_token", "app_token"] {
                if let Some(v) = config.get_mut(ch).and_then(|c| c.get_mut(field)) {
                    if v.is_string() {
                        *v = serde_json::json!("");
                    }
                }
            }
        }
    }
    Json(config)
}

/// POST /api/config/channels - save channel configuration.
pub(crate) async fn api_save_channels_config(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> StatusCode {
    let users = state.users.read().await;
    let (_, is_admin) = auth_user::check_admin(&headers, &state.cookie_secret, &users);
    drop(users);
    if !is_admin {
        return StatusCode::FORBIDDEN;
    }
    let path = std::path::Path::new("channels-config.json");
    match serde_json::to_string_pretty(&body) {
        Ok(json) => {
            if std::fs::write(path, json).is_ok() {
                StatusCode::OK
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
        Err(_) => StatusCode::BAD_REQUEST,
    }
}

pub(crate) async fn api_get_notify_config() -> Json<serde_json::Value> {
    let path = std::path::Path::new("channels-config.json");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(notify) = config.get("notify") {
                    return Json(notify.clone());
                }
            }
        }
    }
    Json(serde_json::json!({
        "enabled": false
    }))
}

pub(crate) async fn api_set_notify_config(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> StatusCode {
    let users = state.users.read().await;
    let (_, is_admin) = auth_user::check_admin(&headers, &state.cookie_secret, &users);
    drop(users);
    if !is_admin {
        return StatusCode::FORBIDDEN;
    }
    let path = std::path::Path::new("channels-config.json");
    let mut config: serde_json::Value = if path.exists() {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    config["notify"] = body;
    if std::fs::write(
        path,
        serde_json::to_string_pretty(&config).unwrap_or_default(),
    )
    .is_ok()
    {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

// --- Permission mode (Always ask / Auto / Plan...) --------------------------

pub(crate) fn permission_mode_to_str(m: laruche_essaim::PermissionMode) -> &'static str {
    use laruche_essaim::PermissionMode::*;
    match m {
        Default => "default",
        Plan => "plan",
        AcceptEdits => "acceptEdits",
        Auto => "auto",
        Bubble => "bubble",
    }
}

pub(crate) fn permission_mode_from_str(s: &str) -> Option<laruche_essaim::PermissionMode> {
    use laruche_essaim::PermissionMode::*;
    match s.trim().to_lowercase().as_str() {
        "default" => Some(Default),
        "plan" => Some(Plan),
        "acceptedits" | "accept_edits" => Some(AcceptEdits),
        "auto" | "yolo" => Some(Auto),
        "bubble" | "always" | "ask" => Some(Bubble),
        _ => None,
    }
}

/// GET /api/config/permission - current permission mode + available options.
pub(crate) async fn api_get_permission_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let mode = state.essaim_config.read().await.permission_mode;
    Json(serde_json::json!({
        "mode": permission_mode_to_str(mode),
        "modes": [
            {"id": "default",     "label": "Ask when necessary (default)"},
            {"id": "acceptEdits", "label": "Accept file edits"},
            {"id": "plan",        "label": "Plan - read-only"},
            {"id": "bubble",      "label": "Always ask"},
            {"id": "auto",        "label": "Allow everything (ignore permissions)"},
        ],
    }))
}

/// POST /api/config/permission - set the permission mode (auth required, persisted).
pub(crate) async fn api_set_permission_config(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let mode_str = body["mode"].as_str().unwrap_or("");
    let mode = permission_mode_from_str(mode_str).ok_or(StatusCode::BAD_REQUEST)?;
    {
        let mut ec = state.essaim_config.write().await;
        ec.permission_mode = mode;
    }
    save_persistent_state(&state).await;
    Ok(Json(serde_json::json!({
        "status": "ok",
        "mode": permission_mode_to_str(mode),
    })))
}

/// GET /api/config/curateur - curateur state (auto-skills/tools).
pub(crate) async fn api_get_curateur_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let ec = state.essaim_config.read().await;
    let env_force = std::env::var("RUCHE_CURATEUR").as_deref() == Ok("1");
    Json(serde_json::json!({
        "enabled": ec.curateur_actif,
        // if the env forces activation, flag it so the UI can explain it
        "env_forced": env_force,
        // co-located toggle: dynamic tool selection (lightweight prompt / small models)
        "dynamic_tools": ec.dynamic_tool_selection,
        // Exposing LaRuche's own tools to an external MCP client. Off by default.
        "mcp_server": ec.mcp_server_actif,
        "mcp_firewall": ec.mcp_pare_feu_actif,
        "mcp_allowlist": ec.mcp_ip_autorisees,
    }))
}

/// POST /api/config/curateur - enables/disables the curateur (auth, persisted).
pub(crate) async fn api_set_curateur_config(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    {
        let mut ec = state.essaim_config.write().await;
        if let Some(v) = body["enabled"].as_bool() {
            ec.curateur_actif = v;
        }
        if let Some(v) = body["dynamic_tools"].as_bool() {
            ec.dynamic_tool_selection = v;
        }
        if let Some(v) = body["mcp_server"].as_bool() {
            ec.mcp_server_actif = v;
        }
        if let Some(v) = body["mcp_firewall"].as_bool() {
            ec.mcp_pare_feu_actif = v;
        }
        if let Some(v) = body["mcp_allowlist"].as_array() {
            // Kept as written, minus blanks: an entry that does not parse is ignored at
            // match time, never treated as a wildcard, so a typo narrows rather than opens.
            ec.mcp_ip_autorisees = v
                .iter()
                .filter_map(|x| x.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
        }
    }
    save_persistent_state(&state).await;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

/// GET /api/secrets - lists secret NAMES (NEVER the values).
pub(crate) async fn api_secrets_list() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "names": laruche_essaim::secrets::noms() }))
}

/// POST /api/secrets - sets/updates a secret {name, value} (auth, encrypted at rest).
pub(crate) async fn api_secrets_set(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> StatusCode {
    if auth_user::extract_user_from_headers(&headers, &state.cookie_secret).is_none() {
        return StatusCode::UNAUTHORIZED;
    }
    let name = body["name"].as_str().unwrap_or("").trim().to_string();
    let value = body["value"].as_str().unwrap_or("").to_string();
    // Clean name for `${NAME}`: letters/digits/underscore only.
    if name.is_empty()
        || value.is_empty()
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return StatusCode::BAD_REQUEST;
    }
    laruche_essaim::secrets::definir(&name, &value);
    let mut map = secrets_vault::charger();
    map.insert(name, value);
    secrets_vault::sauver(&map);
    StatusCode::OK
}

/// DELETE /api/secrets/:name - deletes a secret (auth).
pub(crate) async fn api_secrets_delete(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> StatusCode {
    if auth_user::extract_user_from_headers(&headers, &state.cookie_secret).is_none() {
        return StatusCode::UNAUTHORIZED;
    }
    laruche_essaim::secrets::retirer(&name);
    let mut map = secrets_vault::charger();
    map.remove(&name);
    secrets_vault::sauver(&map);
    StatusCode::OK
}

/// POST /mcp - **MCP server** (JSON-RPC, "Streamable HTTP" transport). Exposes LaRuche's abeilles
/// as MCP tools -> any MCP client (Claude Code, Cursor...)
/// can drive LaRuche. Opt-in security: if `LARUCHE_MCP_TOKEN` is set, requires the matching
/// `X-LaRuche-MCP-Token` header (otherwise open - local POC usage).
pub(crate) async fn api_mcp_server(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    connect_info: Option<axum::extract::ConnectInfo<std::net::SocketAddr>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let id = req.get("id").cloned().unwrap_or(serde_json::Value::Null);
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let err = |code: i64, msg: String| {
        Json(serde_json::json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":msg}}))
    };
    // Security: this executes tools (incl. shell/file). The whole door policy lives in
    // `mcp_pare_feu::controler` so this surface and `/api/mcp` cannot drift apart. They
    // had: this one ignored the Settings switch entirely, so turning the MCP server off
    // still left the full registry reachable from loopback.
    let ip = connect_info.as_ref().map(|ci| ci.0.ip());
    let jeton = headers.get("x-laruche-mcp-token").and_then(|v| v.to_str().ok());
    let outil = req["params"]["name"].as_str().map(|s| s.to_string());
    if let Err(refus) = crate::mcp_pare_feu::controler(&state, ip, jeton).await {
        crate::mcp_pare_feu::journaliser(
            &state, ip, "/mcp", method, outil.as_deref(), Some(&refus), None,
        )
        .await;
        return err(-32000, refus.message());
    }
    crate::mcp_pare_feu::journaliser(&state, ip, "/mcp", method, outil.as_deref(), None, None).await;
    // ONE implementation, shared with `/api/mcp`. This surface used to carry its own copy
    // of initialize / tools/list / tools/call, and the two drifted exactly as the comment
    // above feared: the `disabled_tools` filter was added to the shared dispatch, so
    // `/api/mcp` honoured it while `/mcp` kept listing AND executing a tool the user had
    // switched off. Delegating removes the possibility rather than fixing the symptom.
    let jsonrpc = crate::mcp::JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: req.get("id").cloned(),
        method: method.to_string(),
        params: req["params"].clone(),
    };
    let desactives = state.essaim_config.read().await.disabled_tools.clone();
    // The work indicator, which the shared dispatch does not own: an outside client
    // running shell_exec on this machine is exactly what it exists to show.
    let _garde = match (method, jsonrpc.params["name"].as_str()) {
        ("tools/call", Some(nom)) => {
            let cfg = state.essaim_config.read().await.clone();
            Some(ouvrir_travail(&state, "mcp", nom, &cfg, ip.map(|a| a.to_string())))
        }
        _ => None,
    };
    let reponse = crate::mcp::handle_mcp_request(&state.essaim_registry, &desactives, jsonrpc).await;
    Json(serde_json::to_value(reponse).unwrap_or_else(
        |_| serde_json::json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":"internal error"}}),
    ))
}
