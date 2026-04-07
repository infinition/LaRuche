//! User identity, QR-code login, and cookie-based auth.
//!
//! Flow:
//! 1. Enrollment: user picks a display name → gets a permanent QR (URL with auth secret)
//! 2. Login: browser shows ephemeral QR → phone scans it → challenge resolved → cookie set
//! 3. Cookie: `laruche_auth={user_id}:{timestamp}:{blake3_hmac}` — validated per-request

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

// ─── Data types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    User,
}

impl Default for UserRole {
    fn default() -> Self { Self::User }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub display_name: String,
    /// Base64-encoded 32 random bytes — the user's permanent auth secret (for QR login)
    pub auth_secret: String,
    pub created_at: DateTime<Utc>,
    /// User role: admin (full access) or user (own data only)
    #[serde(default)]
    pub role: UserRole,
    /// Optional password hash (BLAKE3 keyed hash with salt). None = QR-only login.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password_hash: Option<String>,
    /// Per-user preferred model (overrides global default). None = use global.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_model: Option<String>,
    /// Per-user preferred provider. None = use global.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_provider: Option<String>,
}

/// A pending login challenge (ephemeral, 60s TTL).
pub struct AuthChallenge {
    pub challenge_id: Uuid,
    pub created_at: tokio::time::Instant,
    /// Set when the phone scans the QR and validates
    pub resolved_user_id: Option<Uuid>,
}

impl AuthChallenge {
    pub fn new() -> Self {
        Self {
            challenge_id: Uuid::new_v4(),
            created_at: tokio::time::Instant::now(),
            resolved_user_id: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() > 60
    }
}

// ─── User persistence ───────────────────────────────────────────────────────

pub fn load_all_users(dir: &Path) -> HashMap<Uuid, User> {
    let mut users = HashMap::new();
    if !dir.exists() {
        return users;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().extension().map_or(false, |e| e == "json") {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(user) = serde_json::from_str::<User>(&content) {
                        users.insert(user.id, user);
                    }
                }
            }
        }
    }
    users
}

pub fn save_user(user: &User, dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}.json", user.id));
    let json = serde_json::to_string_pretty(user)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn create_user(display_name: &str, role: UserRole, password: Option<&str>) -> User {
    use base64::Engine;
    use rand::RngCore;

    let mut secret_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut secret_bytes);
    let auth_secret = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(secret_bytes);

    let password_hash = password.map(|pw| hash_password(pw));

    User {
        id: Uuid::new_v4(),
        display_name: display_name.to_string(),
        auth_secret,
        created_at: Utc::now(),
        role,
        password_hash,
        preferred_model: None,
        preferred_provider: None,
    }
}

/// Hash a password using BLAKE3 with a random salt.
/// Format: `{salt_hex}:{hash_hex}` (salt is 16 bytes).
pub fn hash_password(password: &str) -> String {
    use rand::RngCore;
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    let salted = format!("{}:{}", hex_encode(&salt), password);
    let hash = blake3::hash(salted.as_bytes());
    format!("{}:{}", hex_encode(&salt), hash.to_hex())
}

/// Verify a password against a stored hash.
pub fn verify_password(password: &str, stored: &str) -> bool {
    let parts: Vec<&str> = stored.splitn(2, ':').collect();
    if parts.len() != 2 { return false; }
    let salt = parts[0];
    let salted = format!("{}:{}", salt, password);
    let hash = blake3::hash(salted.as_bytes());
    hash.to_hex().as_str() == parts[1]
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Find a user by display_name (case-insensitive).
pub fn find_user_by_name<'a>(users: &'a HashMap<Uuid, User>, name: &str) -> Option<&'a User> {
    let lower = name.to_lowercase();
    users.values().find(|u| u.display_name.to_lowercase() == lower)
}

// ─── Cookie auth (BLAKE3 HMAC) ──────────────────────────────────────────────

/// Create a signed auth cookie value: `{user_id}:{timestamp_secs}:{blake3_hex}`
pub fn create_auth_cookie(user_id: Uuid, cookie_secret: &[u8; 32]) -> String {
    let ts = Utc::now().timestamp();
    let payload = format!("{}:{}", user_id, ts);
    let hash = blake3::keyed_hash(cookie_secret, payload.as_bytes());
    format!("{}:{}:{}", user_id, ts, hash.to_hex())
}

/// Validate an auth cookie, returning the user_id if valid.
/// Cookie expires after 30 days.
pub fn validate_auth_cookie(cookie_value: &str, cookie_secret: &[u8; 32]) -> Option<Uuid> {
    let parts: Vec<&str> = cookie_value.splitn(3, ':').collect();
    if parts.len() != 3 {
        return None;
    }

    let user_id = Uuid::parse_str(parts[0]).ok()?;
    let ts: i64 = parts[1].parse().ok()?;
    let provided_hash = parts[2];

    // Check expiry (30 days)
    let now = Utc::now().timestamp();
    if now - ts > 30 * 24 * 3600 {
        return None;
    }

    // Verify BLAKE3 HMAC
    let payload = format!("{}:{}", user_id, ts);
    let expected_hash = blake3::keyed_hash(cookie_secret, payload.as_bytes());
    if expected_hash.to_hex().as_str() != provided_hash {
        return None;
    }

    Some(user_id)
}

/// Extract user_id from the `laruche_auth` cookie in request headers.
pub fn extract_user_from_headers(
    headers: &axum::http::HeaderMap,
    cookie_secret: &[u8; 32],
) -> Option<Uuid> {
    let cookie_header = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for pair in cookie_header.split(';') {
        let pair = pair.trim();
        if let Some(value) = pair.strip_prefix("laruche_auth=") {
            return validate_auth_cookie(value, cookie_secret);
        }
    }
    None
}

/// Check if the request is from an admin user. Returns (user_id, is_admin).
pub fn check_admin(
    headers: &axum::http::HeaderMap,
    cookie_secret: &[u8; 32],
    users: &HashMap<Uuid, User>,
) -> (Option<Uuid>, bool) {
    let uid = extract_user_from_headers(headers, cookie_secret);
    let is_admin = uid
        .and_then(|id| users.get(&id))
        .map(|u| u.role == UserRole::Admin)
        .unwrap_or(false);
    (uid, is_admin)
}

// ─── QR Code generation ─────────────────────────────────────────────────────

/// Generate an SVG QR code with LaRuche branding (amber on dark).
pub fn generate_qr_svg(url: &str) -> String {
    use qrcode::QrCode;
    use qrcode::render::svg;

    let code = match QrCode::new(url.as_bytes()) {
        Ok(c) => c,
        Err(_) => return String::from("<svg></svg>"),
    };

    let svg_str = code
        .render::<svg::Color>()
        .min_dimensions(200, 200)
        .max_dimensions(300, 300)
        .dark_color(svg::Color("#ffbf00")) // LaRuche amber
        .light_color(svg::Color("#1a1a2e")) // Dark background
        .quiet_zone(true)
        .build();

    svg_str
}

/// Build the permanent auth link URL for enrollment QR.
pub fn build_auth_link(host: &str, port: u16, user_id: Uuid, auth_secret: &str) -> String {
    format!(
        "http://{}:{}/auth/link/{}/{}",
        host, port, user_id, auth_secret
    )
}

/// Build the challenge scan URL for login QR.
pub fn build_challenge_url(host: &str, port: u16, challenge_id: Uuid) -> String {
    format!("http://{}:{}/auth/scan/{}", host, port, challenge_id)
}

// ─── Cookie secret persistence ──────────────────────────────────────────────

pub fn generate_cookie_secret() -> [u8; 32] {
    use rand::RngCore;
    let mut secret = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut secret);
    secret
}

pub fn cookie_secret_to_base64(secret: &[u8; 32]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(secret)
}

pub fn cookie_secret_from_base64(b64: &str) -> Option<[u8; 32]> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Some(arr)
}
