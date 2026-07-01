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
    // A password is now mandatory: "name + empty/any password" can no longer mint an account.
    let password = body["password"].as_str().unwrap_or("");
    if password.len() < 6 {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    // Reject a name that already exists, so re-typing a name does not silently create a
    // duplicate account (the previous behaviour). Returning users must use /api/auth/login.
    // The first user ever registered becomes admin, others are regular users.
    let role = {
        let users = state.users.read().await;
        if auth_user::find_user_by_name(&users, display_name).is_some() {
            return Err(StatusCode::CONFLICT);
        }
        if users.is_empty() {
            auth_user::UserRole::Admin
        } else {
            auth_user::UserRole::User
        }
    };
    let user = auth_user::create_user(display_name, role, Some(password));
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
    crate::log_activite(&state, "info", "AUTH", format!("New account: {}", user.display_name), Some(user.id)).await;

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
    // Diagnostic 401s: each failure mode logs its exact cause, so "re-login every
    // launch" is attributable in one glance at the console (no cookie sent by the
    // browser? signature mismatch = secret changed? user files missing?).
    let user_id = match auth_user::extract_user_from_headers(&headers, &state.cookie_secret) {
        Some(id) => id,
        None => {
            let a_cookie = headers
                .get(axum::http::header::COOKIE)
                .and_then(|v| v.to_str().ok())
                .map(|c| c.contains("laruche_auth="))
                .unwrap_or(false);
            if a_cookie {
                warn!("auth/me 401: laruche_auth cookie PRESENT but invalid (secret changed since it was issued, or expired >30d)");
            } else {
                warn!("auth/me 401: no laruche_auth cookie sent by the browser (cookie never stored, or cleared client-side)");
            }
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    let users = state.users.read().await;
    let user = match users.get(&user_id) {
        Some(u) => u,
        None => {
            warn!(user_id = %user_id, users_charges = users.len(),
                "auth/me 401: cookie VALID but user unknown (users/ dir not loaded from this cwd?)");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    Ok(Json(serde_json::json!({
        "user_id": user.id.to_string(),
        "display_name": user.display_name,
        "role": user.role,
        "created_at": user.created_at.to_rfc3339(),
        "avatar": user.avatar,
        "has_password": user.password_hash.is_some(),
        "totp_enabled": user.totp_secret.is_some(),
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
            // Second factor: if TOTP is enabled, a valid current code is required. When the
            // password is right but no code was supplied, tell the client to prompt for one.
            if let Some(secret) = &user.totp_secret {
                let code = body["totp_code"].as_str().unwrap_or("").trim();
                if code.is_empty() {
                    return Ok((
                        axum::http::StatusCode::OK,
                        axum::http::HeaderMap::new(),
                        Json(serde_json::json!({ "totp_required": true })),
                    ));
                }
                let now = chrono::Utc::now().timestamp() as u64;
                if !crate::totp::verify(secret, code, now) {
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }
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
            crate::log_activite(&state, "info", "AUTH", format!("Login: {}", user.display_name), Some(user.id)).await;
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
            crate::log_activite(&state, "warn", "AUTH", format!("Failed login: {}", name), None).await;
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

// ======================== Admin: user management ========================

/// GET /api/admin/users - list all accounts (admin only).
pub(crate) async fn api_admin_list_users(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let users = state.users.read().await;
    let (uid, is_admin) = auth_user::check_admin(&headers, &state.cookie_secret, &users);
    if !is_admin {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut list: Vec<serde_json::Value> = users
        .values()
        .map(|u| {
            serde_json::json!({
                "id": u.id.to_string(),
                "display_name": u.display_name,
                "role": u.role,
                "has_password": u.password_hash.is_some(),
                "created_at": u.created_at,
                "is_self": Some(u.id) == uid,
            })
        })
        .collect();
    list.sort_by(|a, b| a["display_name"].as_str().cmp(&b["display_name"].as_str()));
    Ok(Json(serde_json::json!({ "users": list })))
}

/// DELETE /api/admin/users/:id - delete an account (admin only; cannot delete yourself).
pub(crate) async fn api_admin_delete_user(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let target = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut users = state.users.write().await;
    let (uid, is_admin) = auth_user::check_admin(&headers, &state.cookie_secret, &users);
    if !is_admin {
        return Err(StatusCode::FORBIDDEN);
    }
    if Some(target) == uid {
        return Err(StatusCode::BAD_REQUEST); // cannot delete yourself
    }
    if users.remove(&target).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    let _ = std::fs::remove_file(std::path::Path::new("users").join(format!("{target}.json")));
    info!(target = %target, "Admin deleted user");
    drop(users);
    crate::log_activite(&state, "warn", "ADMIN", format!("Admin deleted account {target}"), uid).await;
    Ok(Json(serde_json::json!({ "status": "deleted", "id": id })))
}

/// POST /api/admin/users/:id/role {role} - change a user's role (admin only).
pub(crate) async fn api_admin_set_role(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let target = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let role = match body["role"].as_str() {
        Some("admin") => auth_user::UserRole::Admin,
        Some("user") => auth_user::UserRole::User,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let mut users = state.users.write().await;
    let (uid, is_admin) = auth_user::check_admin(&headers, &state.cookie_secret, &users);
    if !is_admin {
        return Err(StatusCode::FORBIDDEN);
    }
    if Some(target) == uid && role == auth_user::UserRole::User {
        return Err(StatusCode::BAD_REQUEST); // do not demote yourself (avoid lockout)
    }
    let user = users.get_mut(&target).ok_or(StatusCode::NOT_FOUND)?;
    user.role = role;
    let _ = auth_user::save_user(user, std::path::Path::new("users"));
    let new_role = user.role;
    drop(users);
    crate::log_activite(&state, "warn", "ADMIN", format!("Admin set role of {target} to {new_role:?}"), uid).await;
    Ok(Json(serde_json::json!({ "status": "ok", "id": id, "role": new_role })))
}

/// POST /api/auth/account {display_name?, avatar?} - update your own profile.
pub(crate) async fn api_auth_update_account(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uid = auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let mut users = state.users.write().await;
    // Reject renaming to a name another account already uses.
    if let Some(name) = body["display_name"].as_str() {
        let name = name.trim();
        if !name.is_empty()
            && users
                .values()
                .any(|u| u.id != uid && u.display_name.to_lowercase() == name.to_lowercase())
        {
            return Err(StatusCode::CONFLICT);
        }
    }
    let user = users.get_mut(&uid).ok_or(StatusCode::NOT_FOUND)?;
    if let Some(name) = body["display_name"].as_str() {
        let name = name.trim();
        if !name.is_empty() && name.chars().count() <= 60 {
            user.display_name = name.to_string();
        }
    }
    if let Some(av) = body.get("avatar") {
        if av.is_null() {
            user.avatar = None;
        } else if let Some(s) = av.as_str() {
            // Cap the data URL (client resizes to a small thumbnail) to keep the user file
            // and peer sync light.
            if s.len() <= 200_000 {
                user.avatar = Some(s.to_string());
            }
        }
    }
    let _ = auth_user::save_user(user, std::path::Path::new("users"));
    Ok(Json(serde_json::json!({
        "status": "ok",
        "display_name": user.display_name,
        "avatar": user.avatar,
    })))
}

// ======================== TOTP (2FA) ========================

/// POST /api/auth/totp/setup - generate a fresh TOTP secret (not yet enabled). Returns the
/// secret plus an otpauth QR for the authenticator app.
pub(crate) async fn api_totp_setup(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uid = auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let users = state.users.read().await;
    let user = users.get(&uid).ok_or(StatusCode::UNAUTHORIZED)?;
    let secret = crate::totp::generate_secret();
    let url = crate::totp::otpauth_url(&secret, &user.display_name, "LaRuche");
    let qr = auth_user::generate_qr_svg(&url);
    Ok(Json(serde_json::json!({ "secret": secret, "otpauth_url": url, "qr_svg": qr })))
}

/// POST /api/auth/totp/enable {secret, code} - verify a code against the pending secret, then
/// turn 2FA on for the account.
pub(crate) async fn api_totp_enable(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uid = auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let secret = body["secret"].as_str().unwrap_or("");
    let code = body["code"].as_str().unwrap_or("");
    let now = chrono::Utc::now().timestamp() as u64;
    if secret.is_empty() || !crate::totp::verify(secret, code, now) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let mut users = state.users.write().await;
    let user = users.get_mut(&uid).ok_or(StatusCode::NOT_FOUND)?;
    user.totp_secret = Some(secret.to_string());
    let _ = auth_user::save_user(user, std::path::Path::new("users"));
    Ok(Json(serde_json::json!({ "status": "enabled" })))
}

/// POST /api/auth/totp/disable {code} - verify a current code, then turn 2FA off.
pub(crate) async fn api_totp_disable(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uid = auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let code = body["code"].as_str().unwrap_or("");
    let now = chrono::Utc::now().timestamp() as u64;
    let mut users = state.users.write().await;
    let user = users.get_mut(&uid).ok_or(StatusCode::NOT_FOUND)?;
    match &user.totp_secret {
        Some(secret) if crate::totp::verify(secret, code, now) => {}
        _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
    }
    user.totp_secret = None;
    let _ = auth_user::save_user(user, std::path::Path::new("users"));
    Ok(Json(serde_json::json!({ "status": "disabled" })))
}
