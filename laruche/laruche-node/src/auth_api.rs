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
            return axum::response::Html(r#"<!DOCTYPE html>
<html><head><meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{background:#1a1a2e;color:#e0e0e0;font-family:system-ui;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0}
.card{background:#16213e;padding:2rem;border-radius:16px;text-align:center;max-width:320px}
h2{color:#ffbf00}</style></head>
<body><div class="card">
<h2>Not authenticated</h2>
<p>Open your enrollment link on this phone first.</p>
</div></body></html>"#.to_string());
        }
    };

    // Resolve the challenge
    let mut challenges = state.auth_challenges.write().await;
    if let Some(challenge) = challenges.get_mut(&challenge_id) {
        if challenge.is_expired() {
            return axum::response::Html(r#"<!DOCTYPE html>
<html><head><meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{background:#1a1a2e;color:#e0e0e0;font-family:system-ui;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0}
.card{background:#16213e;padding:2rem;border-radius:16px;text-align:center;max-width:320px}
h2{color:#ef4444}</style></head>
<body><div class="card">
<h2>QR expired</h2>
<p>Go back to the browser and refresh the QR code.</p>
</div></body></html>"#.to_string());
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
        for challenge in challenges.values_mut() {
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

/// Taille maximale d'une photo de profil, en octets de la data URL. Le client
/// redimensionne en 128x128 avant l'envoi; ce plafond est la pour que le fichier
/// du compte et la synchronisation entre ruches restent legers meme si l'appel
/// vient d'ailleurs que de l'interface.
const AVATAR_MAX: usize = 200_000;

// ======================== Admin: user management ========================

/// GET /api/admin/users - list all accounts (admin only).
/// The SUPER-ADMIN is the oldest ADMIN, and it is derived rather than stored: there is
/// no flag anyone can flip, and no way to end up with zero of them. It exists so the
/// instance always keeps one account that cannot be demoted, deleted or locked out by
/// another admin - including one promoted by mistake.
///
/// C'etait le compte le plus ancien tous roles confondus, et cette regle avait un
/// defaut qu'on ne voit qu'une fois qu'il est arrive: un compte cree en passant pour
/// une verification, ou par un canal Telegram, prime pour toujours sur le proprietaire
/// de la machine. Un compte de simple utilisateur devenait le seul intouchable de
/// l'instance, et son proprietaire ne pouvait plus rien y faire. L'anciennete se
/// mesure donc parmi ceux qui portent deja la responsabilite.
///
/// Le repli sur le plus ancien tout court demeure: sans lui, une instance sans aucun
/// admin n'aurait plus de super-admin du tout, ce qui est precisement ce que cette
/// fonction existe pour empecher.
///
/// Ties on `created_at` (two accounts made in the same instant) are broken by id so the
/// answer is stable across restarts instead of depending on map iteration order.
pub(crate) fn super_admin_id(
    users: &std::collections::HashMap<Uuid, auth_user::User>,
) -> Option<Uuid> {
    fn plus_ancien<'a>(
        mut it: impl Iterator<Item = &'a auth_user::User>,
    ) -> Option<&'a auth_user::User> {
        it.next().map(|premier| {
            it.fold(premier, |meilleur, u| {
                match u.created_at.cmp(&meilleur.created_at).then_with(|| u.id.cmp(&meilleur.id)) {
                    std::cmp::Ordering::Less => u,
                    _ => meilleur,
                }
            })
        })
    }

    plus_ancien(
        users
            .values()
            .filter(|u| u.role == auth_user::UserRole::Admin),
    )
    .or_else(|| plus_ancien(users.values()))
    .map(|u| u.id)
}

#[cfg(test)]
mod tests_super_admin {
    use super::*;
    use chrono::{Duration, Utc};

    fn compte(nom: &str, role: auth_user::UserRole, age_jours: i64) -> auth_user::User {
        let mut u = auth_user::create_user(nom, role, None);
        u.created_at = Utc::now() - Duration::days(age_jours);
        u
    }

    fn table(v: Vec<auth_user::User>) -> std::collections::HashMap<Uuid, auth_user::User> {
        v.into_iter().map(|u| (u.id, u)).collect()
    }

    /// Le cas qui a motive la regle: un compte de verification cree avant tout le
    /// monde, mais simple utilisateur, ne doit pas prendre le pas sur l'admin.
    #[test]
    fn un_utilisateur_plus_ancien_ne_prime_pas_sur_un_admin() {
        let vieux = compte("Codex Verif", auth_user::UserRole::User, 90);
        let patron = compte("infinition", auth_user::UserRole::Admin, 60);
        let attendu = patron.id;
        let m = table(vec![vieux, patron]);
        assert_eq!(super_admin_id(&m), Some(attendu));
    }

    #[test]
    fn entre_deux_admins_c_est_le_plus_ancien() {
        let ancien = compte("premier", auth_user::UserRole::Admin, 90);
        let recent = compte("second", auth_user::UserRole::Admin, 10);
        let attendu = ancien.id;
        let m = table(vec![recent, ancien]);
        assert_eq!(super_admin_id(&m), Some(attendu));
    }

    /// Sans aucun admin, l'instance garde quand meme un super-admin: c'est toute la
    /// raison d'etre de cette fonction.
    #[test]
    fn sans_admin_on_retombe_sur_le_plus_ancien_compte() {
        let a = compte("a", auth_user::UserRole::User, 90);
        let b = compte("b", auth_user::UserRole::User, 10);
        let attendu = a.id;
        let m = table(vec![b, a]);
        assert_eq!(super_admin_id(&m), Some(attendu));
    }

    #[test]
    fn une_instance_vide_n_a_pas_de_super_admin() {
        assert_eq!(super_admin_id(&table(vec![])), None);
    }
}

pub(crate) async fn api_admin_list_users(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let users = state.users.read().await;
    let (uid, is_admin) = auth_user::check_admin(&headers, &state.cookie_secret, &users);
    if !is_admin {
        return Err(StatusCode::FORBIDDEN);
    }
    let super_id = super_admin_id(&users);
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
                "is_super": Some(u.id) == super_id,
                "avatar": u.avatar,
            })
        })
        .collect();
    list.sort_by(|a, b| a["display_name"].as_str().cmp(&b["display_name"].as_str()));
    Ok(Json(serde_json::json!({ "users": list })))
}

/// DELETE /api/admin/users/:id - delete an account (admin only; cannot delete yourself).
/// POST /api/admin/users/:id/password - set an account's password WITHOUT knowing the
/// current one. Reserved to the super-admin: an ordinary admin resetting another admin's
/// password would be a silent takeover of that account.
///
/// The old password is never read, compared or returned; only a fresh hash is written,
/// and only the new value is ever accepted from the caller.
pub(crate) async fn api_admin_set_password(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let target = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let password = body["password"].as_str().unwrap_or("");
    if password.len() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut users = state.users.write().await;
    let (uid, is_admin) = auth_user::check_admin(&headers, &state.cookie_secret, &users);
    if !is_admin || uid.is_none() || uid != super_admin_id(&users) {
        return Err(StatusCode::FORBIDDEN);
    }
    let user = users.get_mut(&target).ok_or(StatusCode::NOT_FOUND)?;
    user.password_hash = Some(auth_user::hash_password(password));
    let _ = auth_user::save_user(user, std::path::Path::new("users"));
    drop(users);
    crate::log_activite(
        &state,
        "warn",
        "ADMIN",
        format!("Super-admin reset the password of {target}"),
        uid,
    )
    .await;
    Ok(Json(serde_json::json!({ "status": "ok", "id": id })))
}

/// POST /api/admin/users/:id/avatar {avatar} - la photo d'un autre compte.
///
/// Reserve au super-admin, comme la reinitialisation de mot de passe: changer
/// l'image de quelqu'un est un geste sur SON compte, pas sur le sien. `null`
/// efface et fait revenir l'initiale.
pub(crate) async fn api_admin_set_avatar(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let target = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut users = state.users.write().await;
    let (uid, is_admin) = auth_user::check_admin(&headers, &state.cookie_secret, &users);
    if !is_admin || uid.is_none() || uid != super_admin_id(&users) {
        return Err(StatusCode::FORBIDDEN);
    }
    let user = users.get_mut(&target).ok_or(StatusCode::NOT_FOUND)?;
    match body.get("avatar") {
        Some(serde_json::Value::Null) | None => user.avatar = None,
        Some(serde_json::Value::String(s)) => {
            // Meme garde que sur son propre compte: une image est une data URL
            // bornee, jamais une adresse distante que la page irait chercher.
            if !s.starts_with("data:image/") || s.len() > AVATAR_MAX {
                return Err(StatusCode::BAD_REQUEST);
            }
            user.avatar = Some(s.clone());
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    }
    let _ = auth_user::save_user(user, std::path::Path::new("users"));
    drop(users);
    crate::log_activite(
        &state,
        "info",
        "ADMIN",
        format!("Super-admin changed the avatar of {target}"),
        uid,
    )
    .await;
    Ok(Json(serde_json::json!({ "status": "ok", "id": id })))
}

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
    // The founding account is not deletable: it is the last resort for getting back in.
    if Some(target) == super_admin_id(&users) {
        return Err(StatusCode::FORBIDDEN);
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
    // The super-admin stays an admin. Otherwise a second admin could demote the founding
    // account and take the instance over.
    if Some(target) == super_admin_id(&users) && role == auth_user::UserRole::User {
        return Err(StatusCode::FORBIDDEN);
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
            // Une data URL d'image, et rien d'autre: cette valeur finit dans le
            // `src` d'une balise img, ou une adresse distante ferait fuiter
            // l'adresse IP de chaque personne qui affiche la liste des comptes.
            if s.starts_with("data:image/") && s.len() <= AVATAR_MAX {
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

/// GET /api/reseau/bind-lan - la ruche est-elle servie sur tout le reseau ?
///
/// Rend l'etat EN COURS et l'etat VOULU, qui peuvent differer: la liaison se decide
/// a l'ouverture du port et ne se change pas a chaud. Les distinguer evite la
/// question suivante, "j'ai coche, pourquoi mon telephone ne repond toujours pas".
pub(crate) async fn api_bind_lan_get(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let force = std::env::var("LARUCHE_BIND_LAN").ok();
    let voulu = *state.bind_lan.read().await;
    let en_cours = match force.as_deref() {
        Some("1") => true,
        Some("0") => false,
        _ => voulu,
    };
    Json(serde_json::json!({
        "en_cours": en_cours,
        "voulu": voulu,
        // Une variable posee gele le reglage: le dire, plutot que d'offrir un bouton
        // qui ne changerait rien au prochain demarrage.
        "impose_par_env": force.is_some(),
    }))
}

/// POST /api/reseau/bind-lan {actif}
pub(crate) async fn api_bind_lan_set(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let actif = body["actif"].as_bool().unwrap_or(false);
    *state.bind_lan.write().await = actif;
    crate::state::save_persistent_state(&state).await;
    Json(serde_json::json!({ "status": "ok", "voulu": actif }))
}

/// GET /api/reseau/qr - de quoi ouvrir LaRuche sur un telephone.
///
/// Le QR etait imprime au demarrage du noeud puis efface par son TUI, et il n'a
/// jamais existe dans l'interface web: quelqu'un qui lance l'application de bureau
/// n'avait aucun moyen d'atteindre sa ruche depuis son telephone, alors que la
/// fonction etait la, a deux lignes de distance.
///
/// L'adresse LOCALE ne sert a rien ici: un telephone qui scannerait un code vers
/// `localhost` ouvrirait son propre navigateur sur lui-meme. Sans adresse de
/// reseau, on le dit plutot que de rendre un code inutile.
pub(crate) async fn api_reseau_qr(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let ip = crate::detect_local_ip();
    let port = state.config.api_port;
    match ip {
        Some(ip) => {
            let url = format!("http://{ip}:{port}");
            let svg = auth_user::generate_qr_svg(&url);
            Json(serde_json::json!({
                "disponible": true,
                "url": url,
                "qr_svg": svg,
                // Le code peut etre parfait et la ruche muette: elle n'ecoute que sur
                // la boucle locale tant qu'on ne le lui demande pas. Le dire ici evite
                // de chercher du cote du telephone un probleme qui est cote serveur.
                "bind_lan": std::env::var("LARUCHE_BIND_LAN").as_deref() == Ok("1"),
            }))
        }
        None => Json(serde_json::json!({
            "disponible": false,
            "raison": "no_lan_address",
            "bind_lan": std::env::var("LARUCHE_BIND_LAN").as_deref() == Ok("1"),
        })),
    }
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
