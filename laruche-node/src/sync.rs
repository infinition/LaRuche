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
            if let Err(e) = client_clone
                .post(&url)
                .header("Content-Type", "application/json")
                .body(json_clone)
                .send()
                .await
            {
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
            if let Err(e) = client_clone
                .post(&url)
                .header("Content-Type", "application/json")
                .body(json_clone)
                .send()
                .await
            {
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
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;
    let mut ips = HashSet::new();
    for node in nodes.values() {
        ips.insert(node.manifest.host.clone());
    }
    // Also allow localhost
    ips.insert("127.0.0.1".into());
    ips.insert("::1".into());
    ips
}

/// POST /api/internal/sync/session — Receive a session from a peer.
pub async fn handle_session_sync(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<SyncSessionPayload>,
) -> StatusCode {
    // Verify peer
    let known = get_known_peer_ips(&state).await;
    if !is_known_peer(&addr.ip().to_string(), &known) {
        warn!(ip = %addr.ip(), "Rejected session sync from unknown peer");
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
    Json(payload): Json<SyncUserPayload>,
) -> StatusCode {
    let known = get_known_peer_ips(&state).await;
    if !is_known_peer(&addr.ip().to_string(), &known) {
        warn!(ip = %addr.ip(), "Rejected user sync from unknown peer");
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
) -> Result<Json<BulkSyncResponse>, StatusCode> {
    let known = get_known_peer_ips(&state).await;
    if !is_known_peer(&addr.ip().to_string(), &known) {
        warn!(ip = %addr.ip(), "Rejected bulk sync from unknown peer");
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
pub async fn fetch_bulk_from_peer(
    host: &str,
    port: u16,
    state: &Arc<AppState>,
) {
    let url = format!("http://{}:{}/api/internal/sync/bulk", host, port);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let response = match client.get(&url).send().await {
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
