//! Cross-node session and user sync.
//!
//! Sync strategy:
//! - Push: when a session/user is created or updated, fire-and-forget POST to all known peers
//! - Pull: when a new peer is discovered, bulk-fetch sessions/users updated since last sync
//! - Conflict resolution: last-write-wins (compare `updated_at`)
//! - Protection: only accept sync from known peer IPs (discovered via mDNS)

use crate::{auth_user, AppState};
use chrono::{DateTime, Utc};
use laruche_essaim::Session;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::Json,
};

// ─── Payloads ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncSessionPayload {
    pub session: Session,
    pub origin_node_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncUserPayload {
    pub user: auth_user::User,
    pub origin_node_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkSyncResponse {
    pub sessions: Vec<Session>,
    pub users: Vec<auth_user::User>,
    pub cookie_secret: Option<String>,
}

// ─── Authentification par CODE DE MESH (MAC blake3 keyed) ───────────────────
// Modèle « passphrase WiFi » : un secret partagé entre les ruches d'un même mesh. Les requêtes
// internes sont signées (ts + chemin) → on authentifie par le SECRET, plus par l'IP (qui clignote).
// Tant qu'aucun code n'est configuré, on retombe sur l'allowlist IP (rétro-compatible).

fn mesh_code_path() -> std::path::PathBuf {
    std::path::PathBuf::from("mesh-secret.json")
}
pub fn load_mesh_code() -> Option<String> {
    std::fs::read_to_string(mesh_code_path())
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| {
            v.get("code")
                .and_then(|x| x.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
        })
}
pub fn save_mesh_code(code: &str) {
    let _ = std::fs::write(
        mesh_code_path(),
        serde_json::json!({ "code": code.trim() }).to_string(),
    );
}
fn mesh_key() -> Option<[u8; 32]> {
    load_mesh_code().map(|c| *blake3::hash(c.as_bytes()).as_bytes())
}

// ─── Chiffrement authentifié des payloads mesh (5.3b) ───────────────────────
// Clé dérivée du code de mesh (KDF blake3). Construction encrypt-then-MAC, 100 % blake3 (aucune
// nouvelle dépendance) : keystream = XOF blake3 keyé(nonce), puis MAC blake3 keyé(nonce+ct).
fn aead_key() -> Option<[u8; 32]> {
    load_mesh_code().map(|c| blake3::derive_key("laruche-mesh-aead-v1", c.as_bytes()))
}
fn hex_decode_var(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok())
        .collect()
}
/// Chiffre+authentifie un texte → "nonce.ct.mac" (hex). None si aucun code de mesh (envoi clair).
pub fn seal(plaintext: &str) -> Option<String> {
    let key = aead_key()?;
    let mut nonce = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut nonce);
    let pt = plaintext.as_bytes();
    let mut ks = vec![0u8; pt.len()];
    let mut h = blake3::Hasher::new_keyed(&key);
    h.update(b"ks");
    h.update(&nonce);
    h.finalize_xof().fill(&mut ks);
    let ct: Vec<u8> = pt.iter().zip(ks.iter()).map(|(p, k)| p ^ k).collect();
    let mut m = blake3::Hasher::new_keyed(&key);
    m.update(b"mac");
    m.update(&nonce);
    m.update(&ct);
    let mac = m.finalize();
    Some(format!(
        "{}.{}.{}",
        hex_encode(&nonce),
        hex_encode(&ct),
        hex_encode(&mac.as_bytes()[..16])
    ))
}
/// Déchiffre "nonce.ct.mac". None si code absent / MAC invalide / format invalide.
pub fn open(sealed: &str) -> Option<String> {
    let key = aead_key()?;
    let parts: Vec<&str> = sealed.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let nonce = hex_decode(parts[0], 16)?;
    let ct = hex_decode_var(parts[1])?;
    let mac_given = hex_decode(parts[2], 16)?;
    let mut m = blake3::Hasher::new_keyed(&key);
    m.update(b"mac");
    m.update(&nonce);
    m.update(&ct);
    let mac = m.finalize();
    let ok = mac.as_bytes()[..16]
        .iter()
        .zip(mac_given.iter())
        .fold(0u8, |a, (x, y)| a | (x ^ y))
        == 0;
    if !ok {
        return None;
    }
    let mut ks = vec![0u8; ct.len()];
    let mut h = blake3::Hasher::new_keyed(&key);
    h.update(b"ks");
    h.update(&nonce);
    h.finalize_xof().fill(&mut ks);
    let pt: Vec<u8> = ct.iter().zip(ks.iter()).map(|(c, k)| c ^ k).collect();
    String::from_utf8(pt).ok()
}
/// En-têtes d'auth pour un appel SORTANT vers un pair. Vide si aucun code configuré.
fn mesh_auth_headers(path: &str) -> Vec<(&'static str, String)> {
    match mesh_key() {
        Some(k) => {
            let ts = Utc::now().timestamp();
            let mac = blake3::keyed_hash(&k, format!("{ts}:{path}").as_bytes())
                .to_hex()
                .to_string();
            vec![("X-Miel-Ts", ts.to_string()), ("X-Miel-Mac", mac)]
        }
        None => vec![],
    }
}
/// En-têtes de signature mesh pour un appel sortant : MAC d'appartenance (code de mesh) +
/// signature d'IDENTITÉ ed25519 (X-Miel-From + X-Miel-Sig) prouvant quelle ruche appelle.
/// Exposé pour le « signer global » consommé par le chemin d'inférence (providers).
pub fn sign_headers(path: &str) -> Vec<(String, String)> {
    let mut h: Vec<(String, String)> = mesh_auth_headers(path)
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    let from = my_node_id();
    if !from.is_empty() {
        let ts = Utc::now().timestamp();
        let sig = sign_hex(&format!("{ts}:{path}:{from}"));
        h.push(("X-Miel-From".to_string(), from));
        h.push(("X-Miel-Sig-Ts".to_string(), ts.to_string()));
        h.push(("X-Miel-Sig".to_string(), sig));
    }
    h
}
/// Applique `sign_headers` à une requête reqwest sortante.
pub fn sign_request(mut rb: reqwest::RequestBuilder, path: &str) -> reqwest::RequestBuilder {
    for (k, v) in sign_headers(path) {
        rb = rb.header(k, v);
    }
    rb
}

/// Vérifie l'IDENTITÉ d'une requête entrante (signature ed25519 du pair). Retourne le node_id
/// VÉRIFIÉ de l'appelant, ou None (signature absente/invalide). `peer_pubkey` doit provenir de
/// /api/mesh/identity du pair (mis en cache par l'appelant). Base de l'enforcement `restricted`.
pub fn verified_caller(headers: &axum::http::HeaderMap, path: &str, peer_pubkey: &str) -> Option<String> {
    let from = headers.get("X-Miel-From").and_then(|v| v.to_str().ok())?;
    let ts = headers
        .get("X-Miel-Sig-Ts")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok())?;
    let sig = headers.get("X-Miel-Sig").and_then(|v| v.to_str().ok())?;
    if (Utc::now().timestamp() - ts).abs() > 300 {
        return None;
    }
    if verify_sig(peer_pubkey, &format!("{ts}:{path}:{from}"), sig) {
        Some(from.to_string())
    } else {
        None
    }
}
/// Vérifie une requête ENTRANTE. `None` si aucun code configuré → l'appelant retombe sur l'IP.
pub fn mesh_auth_ok(headers: &axum::http::HeaderMap, path: &str) -> Option<bool> {
    let key = mesh_key()?; // pas de code → None (pas d'auth MAC, on laisse l'allowlist IP décider)
    let ts = headers
        .get("X-Miel-Ts")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok());
    let mac = headers.get("X-Miel-Mac").and_then(|v| v.to_str().ok());
    let (ts, mac) = match (ts, mac) {
        (Some(t), Some(m)) => (t, m),
        _ => return Some(false),
    };
    if (Utc::now().timestamp() - ts).abs() > 300 {
        return Some(false); // anti-rejeu : fenêtre 5 min
    }
    let expected = blake3::keyed_hash(&key, format!("{ts}:{path}").as_bytes())
        .to_hex()
        .to_string();
    // comparaison à temps constant
    let ok = expected.len() == mac.len()
        && expected
            .bytes()
            .zip(mac.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0;
    Some(ok)
}

// ─── Identité forte par nœud : keypair ed25519 (5.3) ────────────────────────
// Le code de mesh prouve l'APPARTENANCE (symétrique). La keypair prouve QUELLE ruche signe
// (asymétrique, non-forgeable) → base du `restricted` sûr et du chiffrement / hors-LAN.
// Persistée dans identity.json (champ `secret` = seed 32 octets hex), à côté du node_id.

fn hex_encode(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}
fn hex_decode(s: &str, n: usize) -> Option<Vec<u8>> {
    if s.len() != n * 2 {
        return None;
    }
    (0..n)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok())
        .collect()
}

fn load_or_create_signing_key() -> ed25519_dalek::SigningKey {
    use ed25519_dalek::SigningKey;
    let path = std::path::Path::new("identity.json");
    let mut v: serde_json::Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    // Clé existante ?
    if let Some(seed) = v
        .get("secret")
        .and_then(|x| x.as_str())
        .and_then(|s| hex_decode(s, 32))
    {
        if let Ok(arr) = <[u8; 32]>::try_from(seed.as_slice()) {
            return SigningKey::from_bytes(&arr);
        }
    }
    // Génère + persiste (en conservant node_id s'il est déjà là).
    let sk = SigningKey::generate(&mut rand::rngs::OsRng);
    v["secret"] = serde_json::Value::String(hex_encode(&sk.to_bytes()));
    let _ = std::fs::write(path, v.to_string());
    sk
}
fn signing_key() -> &'static ed25519_dalek::SigningKey {
    static K: std::sync::OnceLock<ed25519_dalek::SigningKey> = std::sync::OnceLock::new();
    K.get_or_init(load_or_create_signing_key)
}
/// Clé PUBLIQUE de ce nœud (hex) — partagée via /api/mesh/identity.
pub fn my_pubkey_hex() -> String {
    hex_encode(signing_key().verifying_key().as_bytes())
}
/// node_id de ce nœud (depuis identity.json).
pub fn my_node_id() -> String {
    std::fs::read_to_string("identity.json")
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("node_id").and_then(|x| x.as_str()).map(String::from))
        .unwrap_or_default()
}
fn sign_hex(msg: &str) -> String {
    use ed25519_dalek::Signer;
    hex_encode(&signing_key().sign(msg.as_bytes()).to_bytes())
}
/// Vérifie une signature ed25519 d'un pair (pubkey hex + signature hex sur `msg`).
pub fn verify_sig(pubkey_hex: &str, msg: &str, sig_hex: &str) -> bool {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let pk = match hex_decode(pubkey_hex, 32)
        .and_then(|b| <[u8; 32]>::try_from(b.as_slice()).ok())
        .and_then(|a| VerifyingKey::from_bytes(&a).ok())
    {
        Some(k) => k,
        None => return false,
    };
    let sig = match hex_decode(sig_hex, 64)
        .and_then(|b| <[u8; 64]>::try_from(b.as_slice()).ok())
        .map(|a| Signature::from_bytes(&a))
    {
        Some(s) => s,
        None => return false,
    };
    pk.verify(msg.as_bytes(), &sig).is_ok()
}

// ─── Push to peers (fire-and-forget) ────────────────────────────────────────

/// Push a session update to all known peer nodes.
pub async fn push_session_to_peers(session: &Session, state: &Arc<AppState>) {
    let manifest = state.manifest.read().await;
    let origin_node_id = manifest.node_id;
    drop(manifest);

    let payload = SyncSessionPayload {
        session: session.clone(),
        origin_node_id,
        timestamp: Utc::now(),
    };

    let peers = get_peer_endpoints(state).await;
    if peers.is_empty() {
        return;
    }

    let json = match serde_json::to_string(&payload) {
        Ok(j) => j,
        Err(_) => return,
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    for (host, port) in peers {
        let url = format!("http://{}:{}/api/internal/sync/session", host, port);
        let json_clone = json.clone();
        let client_clone = client.clone();
        tokio::spawn(async move {
            let req = sign_request(client_clone.post(&url), "/api/internal/sync/session")
                .header("Content-Type", "application/json")
                .body(json_clone);
            if let Err(e) = req.send().await {
                debug!(peer = %url, error = %e, "Session sync push failed");
            }
        });
    }
}

/// Push a user to all known peer nodes.
pub async fn push_user_to_peers(user: &auth_user::User, state: &Arc<AppState>) {
    let manifest = state.manifest.read().await;
    let origin_node_id = manifest.node_id;
    drop(manifest);

    let payload = SyncUserPayload {
        user: user.clone(),
        origin_node_id,
        timestamp: Utc::now(),
    };

    let peers = get_peer_endpoints(state).await;
    if peers.is_empty() {
        return;
    }

    let json = match serde_json::to_string(&payload) {
        Ok(j) => j,
        Err(_) => return,
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    for (host, port) in peers {
        let url = format!("http://{}:{}/api/internal/sync/user", host, port);
        let json_clone = json.clone();
        let client_clone = client.clone();
        tokio::spawn(async move {
            let req = sign_request(client_clone.post(&url), "/api/internal/sync/user")
                .header("Content-Type", "application/json")
                .body(json_clone);
            if let Err(e) = req.send().await {
                debug!(peer = %url, error = %e, "User sync push failed");
            }
        });
    }
}

/// Get (host, port) of all known peer nodes (excluding self).
async fn get_peer_endpoints(state: &Arc<AppState>) -> Vec<(String, u16)> {
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;
    let manifest = state.manifest.read().await;
    let my_id = manifest.node_id;
    let my_host = manifest.api_endpoint.host.clone();
    drop(manifest);

    let mut peers = Vec::new();
    for node in nodes.values() {
        if node.manifest.node_id == Some(my_id) || node.manifest.host == my_host {
            continue;
        }
        let port = node
            .manifest
            .port
            .unwrap_or(miel_protocol::DEFAULT_API_PORT);
        peers.push((node.manifest.host.clone(), port));
    }
    peers
}

// ─── Receive handlers ───────────────────────────────────────────────────────

/// Verify the request comes from a known peer IP.
fn is_known_peer(remote_ip: &str, known_peers: &HashSet<String>) -> bool {
    known_peers.contains(remote_ip)
}

async fn get_known_peer_ips(state: &Arc<AppState>) -> HashSet<String> {
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};
    // Cache COLLANT : une IP vue par la découverte reste autorisée GRACE après sa dernière
    // apparition. Évite que des nœuds qui clignotent (découverte mDNS intermittente, WiFi) se
    // fassent rejeter leur sync (« Rejected ... from unknown peer ») le temps d'un trou.
    static CACHE: OnceLock<Mutex<std::collections::HashMap<String, Instant>>> = OnceLock::new();
    const GRACE: Duration = Duration::from_secs(600); // 10 min
    let cache = CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));

    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;
    let now = Instant::now();
    let mut ips = HashSet::new();
    {
        let mut c = cache.lock().unwrap();
        for node in nodes.values() {
            c.insert(node.manifest.host.clone(), now);
        }
        c.retain(|_, t| now.duration_since(*t) < GRACE);
        for ip in c.keys() {
            ips.insert(ip.clone());
        }
    }
    ips.insert("127.0.0.1".into());
    ips.insert("::1".into());
    ips
}

/// POST /api/internal/sync/session — Receive a session from a peer.
pub async fn handle_session_sync(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<SyncSessionPayload>,
) -> StatusCode {
    // Auth : code de mesh (MAC) si configuré, sinon allowlist IP (rétro-compatible).
    let authed = match mesh_auth_ok(&headers, "/api/internal/sync/session") {
        Some(ok) => ok,
        None => is_known_peer(&addr.ip().to_string(), &get_known_peer_ips(&state).await),
    };
    if !authed {
        warn!(ip = %addr.ip(), "Rejected session sync (auth)");
        return StatusCode::FORBIDDEN;
    }

    let session = payload.session;
    let session_id = session.id;

    let mut sessions = state.essaim_sessions.write().await;
    let should_update = match sessions.get(&session_id) {
        Some(existing) => session.updated_at > existing.updated_at,
        None => true,
    };

    if should_update {
        // Save to disk
        if let Err(e) = session.sauvegarder() {
            warn!(error = %e, "Failed to save synced session to disk");
        }
        debug!(session_id = %session_id, from = %payload.origin_node_id, "Session synced from peer");
        sessions.insert(session_id, session);
    }

    StatusCode::OK
}

/// POST /api/internal/sync/user — Receive a user from a peer.
pub async fn handle_user_sync(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<SyncUserPayload>,
) -> StatusCode {
    let authed = match mesh_auth_ok(&headers, "/api/internal/sync/user") {
        Some(ok) => ok,
        None => is_known_peer(&addr.ip().to_string(), &get_known_peer_ips(&state).await),
    };
    if !authed {
        warn!(ip = %addr.ip(), "Rejected user sync (auth)");
        return StatusCode::FORBIDDEN;
    }

    let user = payload.user;
    let user_id = user.id;

    let mut users = state.users.write().await;
    if !users.contains_key(&user_id) {
        let users_dir = std::path::Path::new("users");
        if let Err(e) = auth_user::save_user(&user, users_dir) {
            warn!(error = %e, "Failed to save synced user to disk");
        }
        debug!(user_id = %user_id, name = %user.display_name, from = %payload.origin_node_id, "User synced from peer");
        users.insert(user_id, user);
    }

    StatusCode::OK
}

/// GET /api/internal/sync/bulk — Return all sessions + users (for new peer joining).
pub async fn handle_bulk_sync(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
) -> Result<Json<BulkSyncResponse>, StatusCode> {
    let authed = match mesh_auth_ok(&headers, "/api/internal/sync/bulk") {
        Some(ok) => ok,
        None => is_known_peer(&addr.ip().to_string(), &get_known_peer_ips(&state).await),
    };
    if !authed {
        warn!(ip = %addr.ip(), "Rejected bulk sync (auth)");
        return Err(StatusCode::FORBIDDEN);
    }

    let sessions = state.essaim_sessions.read().await;
    let users = state.users.read().await;

    let all_sessions: Vec<Session> = sessions.values().cloned().collect();
    let all_users: Vec<auth_user::User> = users.values().cloned().collect();

    let cookie_secret_b64 = Some(auth_user::cookie_secret_to_base64(&state.cookie_secret));

    info!(
        sessions = all_sessions.len(),
        users = all_users.len(),
        peer = %addr.ip(),
        "Bulk sync served to peer"
    );

    Ok(Json(BulkSyncResponse {
        sessions: all_sessions,
        users: all_users,
        cookie_secret: cookie_secret_b64,
    }))
}

/// Fetch bulk sync from a peer and merge into local state.
pub async fn fetch_bulk_from_peer(host: &str, port: u16, state: &Arc<AppState>) {
    let url = format!("http://{}:{}/api/internal/sync/bulk", host, port);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let response = match sign_request(client.get(&url), "/api/internal/sync/bulk").send().await {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            debug!(peer = %url, status = %r.status(), "Bulk sync fetch failed");
            return;
        }
        Err(e) => {
            debug!(peer = %url, error = %e, "Bulk sync fetch failed");
            return;
        }
    };

    let bulk: BulkSyncResponse = match response.json().await {
        Ok(b) => b,
        Err(e) => {
            debug!(error = %e, "Failed to parse bulk sync response");
            return;
        }
    };

    // Merge sessions (last-write-wins)
    {
        let mut sessions = state.essaim_sessions.write().await;
        let mut added = 0usize;
        let mut updated = 0usize;
        for session in bulk.sessions {
            let id = session.id;
            let should_insert = match sessions.get(&id) {
                Some(existing) => session.updated_at > existing.updated_at,
                None => true,
            };
            if should_insert {
                if sessions.contains_key(&id) {
                    updated += 1;
                } else {
                    added += 1;
                }
                let _ = session.sauvegarder();
                sessions.insert(id, session);
            }
        }
        if added > 0 || updated > 0 {
            info!(added, updated, peer = %host, "Sessions merged from peer");
        }
    }

    // Merge users
    {
        let mut users = state.users.write().await;
        let mut added = 0usize;
        let users_dir = std::path::Path::new("users");
        for user in bulk.users {
            if !users.contains_key(&user.id) {
                let _ = auth_user::save_user(&user, users_dir);
                users.insert(user.id, user);
                added += 1;
            }
        }
        if added > 0 {
            info!(added, peer = %host, "Users merged from peer");
        }
    }

    info!(peer = %host, "Bulk sync completed");
}
