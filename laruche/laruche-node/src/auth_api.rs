//! Authentication endpoints (passkey enroll/challenge, login/logout, password, model selection, QR scan, permanent link) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use axum::http::StatusCode;
use std::sync::Arc;

// ======================== Auth Endpoints ========================

/// POST /api/auth/enroll: Create a new user identity.
pub(crate) async fn api_auth_enroll(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<
    (
        axum::http::StatusCode,
        axum::http::HeaderMap,
        Json<serde_json::Value>,
    ),
    StatusCode,
> {
    let display_name = body["display_name"]
        .as_str()
        .unwrap_or("Utilisateur")
        .trim();
    if display_name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // First user ever registered becomes admin, others are regular users
    let role = {
        let users = state.users.read().await;
        if users.is_empty() {
            auth_user::UserRole::Admin
        } else {
            auth_user::UserRole::User
        }
    };
    let password = body["password"].as_str().filter(|p| !p.is_empty());
    let user = auth_user::create_user(display_name, role, password);
    let users_dir = std::path::Path::new("users");
    if let Err(e) = auth_user::save_user(&user, users_dir) {
        warn!(error = %e, "Failed to save user");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Build permanent auth link QR
    let manifest = state.manifest.read().await;
    let host = manifest.api_endpoint.host.clone();
    let port = manifest.api_endpoint.port;
    drop(manifest);

    let auth_url = auth_user::build_auth_link(&host, port, user.id, &user.auth_secret);
    let qr_svg = auth_user::generate_qr_svg(&auth_url);

    // Set auth cookie
    let cookie_value = auth_user::create_auth_cookie(user.id, &state.cookie_secret);
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        format!(
            "laruche_auth={}; HttpOnly; Path=/; SameSite=Lax; Max-Age=2592000",
            cookie_value
        )
        .parse()
        .unwrap(),
    );

    // Store user in memory
    state.users.write().await.insert(user.id, user.clone());
    // Sync to peers
    let sync_state = state.clone();
    let sync_user = user.clone();
    tokio::spawn(async move {
        sync::push_user_to_peers(&sync_user, &sync_state).await;
    });

    info!(user_id = %user.id, name = %user.display_name, "New user enrolled");

    Ok((
        axum::http::StatusCode::OK,
        headers,
        Json(serde_json::json!({
            "user_id": user.id.to_string(),
            "display_name": user.display_name,
            "role": user.role,
            "qr_svg": qr_svg,
            "auth_url": auth_url,
        })),
    ))
}

/// GET /api/auth/me: Return current user info (from cookie).
pub(crate) async fn api_auth_me(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user_id = auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let users = state.users.read().await;
    let user = users.get(&user_id).ok_or(StatusCode::UNAUTHORIZED)?;
    Ok(Json(serde_json::json!({
        "user_id": user.id.to_string(),
        "display_name": user.display_name,
        "role": user.role,
        "created_at": user.created_at.to_rfc3339(),
    })))
}

/// GET /api/auth/challenge: Generate ephemeral login QR.
pub(crate) async fn api_auth_challenge(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // Cleanup expired challenges
    {
        let mut challenges = state.auth_challenges.write().await;
        challenges.retain(|_, c| !c.is_expired());
    }

    let challenge = auth_user::AuthChallenge::new();
    let challenge_id = challenge.challenge_id;

    let manifest = state.manifest.read().await;
    let host = manifest.api_endpoint.host.clone();
    let port = manifest.api_endpoint.port;
    drop(manifest);
    let scan_url = auth_user::build_challenge_url(&host, port, challenge_id);

    let qr_svg = auth_user::generate_qr_svg(&scan_url);

    state
        .auth_challenges
        .write()
        .await
        .insert(challenge_id, challenge);

    Json(serde_json::json!({
        "challenge_id": challenge_id.to_string(),
        "qr_svg": qr_svg,
        "expires_in": 60,
    }))
}

/// GET /api/auth/status/:id: Poll challenge status.
pub(crate) async fn api_auth_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let challenge_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return Json(serde_json::json!({"status": "invalid"})),
    };

    let challenges = state.auth_challenges.read().await;
    match challenges.get(&challenge_id) {
        Some(c) if c.is_expired() => Json(serde_json::json!({"status": "expired"})),
        Some(c) if c.resolved_user_id.is_some() => {
            let user_id = c.resolved_user_id.unwrap();
            let users = state.users.read().await;
            let display_name = users
                .get(&user_id)
                .map(|u| u.display_name.clone())
                .unwrap_or_default();
            let token = auth_user::create_auth_cookie(user_id, &state.cookie_secret);
            Json(serde_json::json!({
                "status": "authenticated",
                "token": token,
                "user_id": user_id.to_string(),
                "display_name": display_name,
            }))
        }
        Some(_) => Json(serde_json::json!({"status": "pending"})),
        None => Json(serde_json::json!({"status": "not_found"})),
    }
}

/// GET /auth/scan/:challenge_id: Phone scans this to resolve challenge.
pub(crate) async fn auth_scan_challenge(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(challenge_id_str): axum::extract::Path<String>,
    headers: axum::http::HeaderMap,
) -> axum::response::Html<String> {
    let challenge_id = match Uuid::parse_str(&challenge_id_str) {
        Ok(u) => u,
        Err(_) => return axum::response::Html("<h1>Invalid challenge</h1>".into()),
    };

    // Extract user from phone's cookie
    let user_id = match auth_user::extract_user_from_headers(&headers, &state.cookie_secret) {
        Some(uid) => uid,
        None => {
            return axum::response::Html(format!(
                r#"<!DOCTYPE html>
<html><head><meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{{background:#1a1a2e;color:#e0e0e0;font-family:system-ui;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0}}
.card{{background:#16213e;padding:2rem;border-radius:16px;text-align:center;max-width:320px}}
h2{{color:#ffbf00}}</style></head>
<body><div class="card">
<h2>Not authenticated</h2>
<p>Open your enrollment link on this phone first.</p>
</div></body></html>"#
            ));
        }
    };

    // Resolve the challenge
    let mut challenges = state.auth_challenges.write().await;
    if let Some(challenge) = challenges.get_mut(&challenge_id) {
        if challenge.is_expired() {
            return axum::response::Html(format!(
                r#"<!DOCTYPE html>
<html><head><meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{{background:#1a1a2e;color:#e0e0e0;font-family:system-ui;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0}}
.card{{background:#16213e;padding:2rem;border-radius:16px;text-align:center;max-width:320px}}
h2{{color:#ef4444}}</style></head>
<body><div class="card">
<h2>QR expired</h2>
<p>Go back to the browser and refresh the QR code.</p>
</div></body></html>"#
            ));
        }
        challenge.resolved_user_id = Some(user_id);
    }
    drop(challenges);

    let users = state.users.read().await;
    let display_name = users
        .get(&user_id)
        .map(|u| u.display_name.clone())
        .unwrap_or_else(|| "Utilisateur".into());

    info!(user_id = %user_id, name = %display_name, "Login challenge resolved via QR scan");

    axum::response::Html(format!(
        r#"<!DOCTYPE html>
<html><head><meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{{background:#1a1a2e;color:#e0e0e0;font-family:system-ui;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0}}
.card{{background:#16213e;padding:2rem;border-radius:16px;text-align:center;max-width:320px}}
h2{{color:#22c55e}}.check{{font-size:3rem;margin-bottom:1rem}}</style></head>
<body><div class="card">
<div class="check">&#x2714;</div>
<h2>Connecte !</h2>
<p>Bienvenue <strong>{}</strong>.<br>Vous pouvez fermer cet onglet.</p>
</div></body></html>"#,
        display_name
    ))
}

/// GET /auth/link/:user_id/:secret: Permanent auth link (from enrollment QR).
pub(crate) async fn auth_permanent_link(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((user_id_str, secret)): axum::extract::Path<(String, String)>,
) -> Result<
    (
        axum::http::StatusCode,
        axum::http::HeaderMap,
        axum::response::Html<String>,
    ),
    StatusCode,
> {
    let user_id = Uuid::parse_str(&user_id_str).map_err(|_| StatusCode::BAD_REQUEST)?;
    let users = state.users.read().await;
    let user = users.get(&user_id).ok_or(StatusCode::NOT_FOUND)?;

    if user.auth_secret != secret {
        return Err(StatusCode::FORBIDDEN);
    }

    let display_name = user.display_name.clone();
    drop(users);

    // Set auth cookie on this device (phone)
    let cookie_value = auth_user::create_auth_cookie(user_id, &state.cookie_secret);
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        format!(
            "laruche_auth={}; HttpOnly; Path=/; SameSite=Lax; Max-Age=2592000",
            cookie_value
        )
        .parse()
        .unwrap(),
    );

    // Also check if there's a pending challenge to resolve
    // (phone scans enrollment QR which also resolves any open challenge)
    {
        let mut challenges = state.auth_challenges.write().await;
        for (_, challenge) in challenges.iter_mut() {
            if !challenge.is_expired() && challenge.resolved_user_id.is_none() {
                challenge.resolved_user_id = Some(user_id);
                break; // resolve the first pending one
            }
        }
    }

    info!(user_id = %user_id, name = %display_name, "Auth via permanent link");

    Ok((
        axum::http::StatusCode::OK,
        headers,
        axum::response::Html(format!(
            r#"<!DOCTYPE html>
<html><head><meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{{background:#1a1a2e;color:#e0e0e0;font-family:system-ui;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0}}
.card{{background:#16213e;padding:2rem;border-radius:16px;text-align:center;max-width:320px}}
h2{{color:#ffbf00}}.bee{{font-size:3rem;margin-bottom:1rem}}</style></head>
<body><div class="card">
<div class="bee">&#x1F41D;</div>
<h2>Identite confirmee</h2>
<p>Bienvenue <strong>{}</strong>.<br>Ce telephone est maintenant votre cle d'acces LaRuche.</p>
</div></body></html>"#,
            display_name
        )),
    ))
}

/// POST /api/auth/logout: Clear auth cookie.
pub(crate) async fn api_auth_logout() -> (axum::http::StatusCode, axum::http::HeaderMap) {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        "laruche_auth=; HttpOnly; Path=/; SameSite=Lax; Max-Age=0"
            .parse()
            .unwrap(),
    );
    (axum::http::StatusCode::OK, headers)
}

/// POST /api/auth/login: Login with display_name + password.
pub(crate) async fn api_auth_login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<
    (
        axum::http::StatusCode,
        axum::http::HeaderMap,
        Json<serde_json::Value>,
    ),
    StatusCode,
> {
    let name = body["display_name"].as_str().unwrap_or("").trim();
    let password = body["password"].as_str().unwrap_or("");
    if name.is_empty() || password.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let users = state.users.read().await;
    let user = auth_user::find_user_by_name(&users, name).ok_or(StatusCode::UNAUTHORIZED)?;

    match &user.password_hash {
        Some(hash) if auth_user::verify_password(password, hash) => {
            let cookie_value = auth_user::create_auth_cookie(user.id, &state.cookie_secret);
            let mut headers = axum::http::HeaderMap::new();
            headers.insert(
                axum::http::header::SET_COOKIE,
                format!(
                    "laruche_auth={}; HttpOnly; Path=/; SameSite=Lax; Max-Age=2592000",
                    cookie_value
                )
                .parse()
                .unwrap(),
            );
            info!(user_id = %user.id, name = %user.display_name, "Login via password");
            Ok((
                axum::http::StatusCode::OK,
                headers,
                Json(serde_json::json!({
                    "user_id": user.id.to_string(),
                    "display_name": user.display_name,
                    "role": user.role,
                })),
            ))
        }
        _ => {
            warn!(name = %name, "Failed login attempt");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// POST /api/auth/password: Set or change password (requires auth).
pub(crate) async fn api_auth_set_password(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user_id = auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let password = body["password"].as_str().unwrap_or("");
    if password.len() < 4 {
        return Ok(Json(
            serde_json::json!({"error": "Password must be at least 4 characters"}),
        ));
    }

    let mut users = state.users.write().await;
    if let Some(user) = users.get_mut(&user_id) {
        user.password_hash = Some(auth_user::hash_password(password));
        let users_dir = std::path::Path::new("users");
        let _ = auth_user::save_user(user, users_dir);
        info!(user_id = %user_id, "Password set/changed");
        Ok(Json(serde_json::json!({"status": "ok"})))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// POST /api/auth/model: Set per-user preferred model (doesn't touch global config).
pub(crate) async fn api_auth_set_model(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user_id = auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let model = body["model"].as_str().unwrap_or("").to_string();
    let provider = body["provider"].as_str().map(|s| s.to_string());

    let mut users = state.users.write().await;
    if let Some(user) = users.get_mut(&user_id) {
        user.preferred_model = if model.is_empty() {
            None
        } else {
            Some(model.clone())
        };
        user.preferred_provider = provider;
        let users_dir = std::path::Path::new("users");
        let _ = auth_user::save_user(user, users_dir);
        Ok(Json(serde_json::json!({"status": "ok", "model": model})))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
