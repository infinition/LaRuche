//! Mesh messaging (Phase 4 DM between instances/users): identity/peers, mesh skills sync, send/receive, local inbox storage - split out of main.rs.

use crate::*;
use axum::extract::{Path, State};
use axum::response::Json;
use std::sync::Arc;

// ===================== Phase 4 - Mesh messaging (DM between instances/users) =====================
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub(crate) struct InboxMessage {
    pub(crate) id: String,
    pub(crate) peer_id: String,
    pub(crate) peer_name: String,
    pub(crate) dir: String, // "in" (received) | "out" (sent)
    pub(crate) text: String,
    pub(crate) ts: i64,
    pub(crate) read: bool,
}
fn inbox_path() -> std::path::PathBuf {
    std::path::PathBuf::from("inbox.json")
}
pub(crate) fn read_inbox() -> Vec<InboxMessage> {
    std::fs::read_to_string(inbox_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
fn write_inbox(msgs: &[InboxMessage]) {
    if let Ok(s) = serde_json::to_string_pretty(msgs) {
        let _ = std::fs::write(inbox_path(), s);
    }
}
fn append_inbox(m: InboxMessage) {
    let mut v = read_inbox();
    v.push(m);
    if v.len() > 1000 {
        let drop_n = v.len() - 1000;
        v.drain(0..drop_n);
    }
    write_inbox(&v);
}

/// GET /api/mesh/code - indicates whether a mesh code is configured (never the secret itself).
pub(crate) async fn api_mesh_code_get() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "set": sync::load_mesh_code().is_some() }))
}
/// POST /api/mesh/code {code} - sets/clears the shared mesh code (auth + encryption base).
pub(crate) async fn api_mesh_code_set(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let code = body["code"].as_str().unwrap_or("");
    sync::save_mesh_code(code);
    Json(serde_json::json!({ "status": "ok", "set": !code.trim().is_empty() }))
}

/// GET /api/mesh/identity - node_id + this node's ed25519 PUBLIC key (hex). Peers fetch it
/// and cache it to verify signatures (strong identity, `restricted`).
pub(crate) async fn api_mesh_identity() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "node_id": sync::my_node_id(), "pubkey": sync::my_pubkey_hex() }))
}

/// GET /api/mesh/whoami - identity of THIS instance (laruche ID + name).
pub(crate) async fn api_mesh_whoami(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let m = state.manifest.read().await;
    Json(serde_json::json!({
        "id": m.node_id.to_string(),
        "name": m.node_name,
        // Cette ruche accepte-t-elle les connexions venant du reseau ?
        //
        // Le mDNS est du multicast: elle s'annonce avec son adresse LAN meme quand
        // l'API n'ecoute que sur 127.0.0.1. Elle est donc VISIBLE sans etre JOIGNABLE,
        // et c'est la premiere cause de « l'autre ruche me voit mais rien ne repond ».
        // L'interface a besoin de le savoir pour le dire, au lieu de laisser chercher.
        "joignable_reseau": std::env::var("LARUCHE_BIND_LAN").as_deref() == Ok("1"),
        "code_mesh": crate::sync::load_mesh_code().is_some(),
    }))
}

/// GET /api/mesh/peers - other LaRuche instances discovered on the network (directory).
pub(crate) async fn api_mesh_peers(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;
    let m = state.manifest.read().await;
    let peers: Vec<serde_json::Value> = nodes
        .values()
        .filter(|n| {
            n.manifest.node_id != Some(m.node_id) && n.manifest.host != m.api_endpoint.host
        })
        .filter_map(|n| {
            n.manifest.node_id.map(|id| {
                serde_json::json!({
                    "id": id.to_string(),
                    "name": n.manifest.node_name,
                    "host": n.manifest.host,
                    "port": n.manifest.port.or(n.manifest.dashboard_port),
                    // Vu pour la derniere fois il y a combien de secondes. L'interface
                    // peut ainsi montrer une presence stable plutot qu'un pair qui
                    // clignote: l'eviction n'a lieu qu'a 90 s.
                    "vu_il_y_a_s": (chrono::Utc::now() - n.last_seen).num_seconds(),
                })
            })
        })
        .collect();

    // Sonde HTTP de chaque pair, en parallele. Une ruche s'annonce avec son adresse LAN
    // tout en n'ecoutant peut-etre que sur 127.0.0.1: elle est alors visible sans etre
    // joignable, et cliquer dessus ne donnait rien sans explication.
    let sondes = peers.iter().map(|p| {
        let hote = p["host"].as_str().unwrap_or("").to_string();
        let port = p["port"].as_u64().unwrap_or(0);
        async move {
            if hote.is_empty() || port == 0 {
                return false;
            }
            let url = format!("http://{hote}:{port}/manifest.json");
            reqwest::Client::new()
                .get(&url)
                .timeout(std::time::Duration::from_millis(800))
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
        }
    });
    let joignables = futures_util::future::join_all(sondes).await;
    let peers: Vec<serde_json::Value> = peers
        .into_iter()
        .zip(joignables)
        .map(|(mut p, ok)| {
            if let Some(o) = p.as_object_mut() {
                o.insert("joignable".into(), serde_json::json!(ok));
            }
            p
        })
        .collect();

    Json(serde_json::json!({ "peers": peers }))
}

// --- Gap A - FEDERATION OF VERIFIED SKILLS BETWEEN NODES ----------------------------
// A swarm that learns collectively: when a node has (created/verified) a skill, the others
// can fetch it. Mechanics: each node ANNOUNCES its skills (slug + content hash),
// and SYNCHRONIZES by pulling from peers the skills it lacks (or whose hash differs).

/// Lists local skills on disk (`skills/<slug>/SKILL.md`) with a content hash.
pub(crate) fn lister_skills_locaux() -> Vec<(String, String, String)> {
    // (slug, hash, content)
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir("skills") else {
        return out;
    };
    for e in rd.flatten() {
        if !e.path().is_dir() {
            continue;
        }
        let slug = e.file_name().to_string_lossy().to_string();
        let md = e.path().join("SKILL.md");
        if let Ok(content) = std::fs::read_to_string(&md) {
            let hash = blake3::hash(content.as_bytes()).to_hex().to_string();
            out.push((slug, hash, content));
        }
    }
    out
}

/// GET /api/mesh/skills - announces THIS node's verified skills (slug + hash, without content).
pub(crate) async fn api_mesh_skills_list() -> Json<serde_json::Value> {
    let skills: Vec<serde_json::Value> = lister_skills_locaux()
        .into_iter()
        .map(|(slug, hash, _)| serde_json::json!({ "slug": slug, "hash": hash }))
        .collect();
    Json(serde_json::json!({ "skills": skills }))
}

/// GET /api/mesh/skills/:slug - returns a skill's SKILL.md content (for a peer to pull).
pub(crate) async fn api_mesh_skill_get(Path(slug): Path<String>) -> Json<serde_json::Value> {
    // Anti-traversal guard: slug = a single alphanumeric/_/- segment.
    if slug.is_empty() || !slug.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Json(serde_json::json!({ "status": "error", "error": "invalid slug" }));
    }
    match std::fs::read_to_string(format!("skills/{slug}/SKILL.md")) {
        Ok(content) => Json(serde_json::json!({ "slug": slug, "content": content })),
        Err(_) => Json(serde_json::json!({ "status": "error", "error": "not found" })),
    }
}

/// POST /api/mesh/sync - pulls from all active peers the missing/different verified skills,
/// writes them to disk then re-indexes them in memory. Returns the report. **Additive**: overwrites
/// a local skill only if the remote hash differs; never deletes.
pub(crate) async fn api_mesh_skills_sync(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let locaux: std::collections::HashMap<String, String> = lister_skills_locaux()
        .into_iter()
        .map(|(slug, hash, _)| (slug, hash))
        .collect();

    let nodes = state.listener.read().await.get_nodes().await;
    let m = state.manifest.read().await;
    let self_id = m.node_id;
    let self_host = m.api_endpoint.host.clone();
    drop(m);

    let http = reqwest::Client::builder()
        .timeout(Duration::from_millis(PEER_FETCH_TIMEOUT_MS))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut importes: Vec<String> = Vec::new();
    let mut vus_pairs = 0usize;

    for node in nodes.values() {
        if node.manifest.node_id == Some(self_id)
            || node.manifest.host == self_host
            || is_stale(node.last_seen)
        {
            continue;
        }
        let host = node.manifest.host.clone();
        let port = node.manifest.port.unwrap_or(miel_protocol::DEFAULT_API_PORT);
        let base = format!("http://{host}:{port}");
        vus_pairs += 1;

        // 1) peer announcement
        let Ok(resp) = http.get(format!("{base}/api/mesh/skills")).send().await else {
            continue;
        };
        let Ok(val) = resp.json::<serde_json::Value>().await else {
            continue;
        };
        let Some(liste) = val.get("skills").and_then(|s| s.as_array()) else {
            continue;
        };

        for sk in liste {
            let slug = sk.get("slug").and_then(|s| s.as_str()).unwrap_or("");
            let hash = sk.get("hash").and_then(|s| s.as_str()).unwrap_or("");
            if slug.is_empty()
                || !slug.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                continue;
            }
            // already up to date?
            if locaux.get(slug).map(|h| h == hash).unwrap_or(false) {
                continue;
            }
            // 2) pull the content
            let Ok(r) = http
                .get(format!("{base}/api/mesh/skills/{slug}"))
                .send()
                .await
            else {
                continue;
            };
            let Ok(body) = r.json::<serde_json::Value>().await else {
                continue;
            };
            let Some(content) = body.get("content").and_then(|c| c.as_str()) else {
                continue;
            };
            // 3) write to disk
            let dir = format!("skills/{slug}");
            let peer_name = node
                .manifest
                .node_name
                .clone()
                .unwrap_or_else(|| host.clone());
            if std::fs::create_dir_all(&dir).is_ok()
                && std::fs::write(format!("{dir}/SKILL.md"), content).is_ok()
            {
                importes.push(format!("{slug} ⇐ {peer_name}"));
                laruche_essaim::feed_journal::record(
                    "LaRuche",
                    "mesh",
                    "federated the skill",
                    format!("{slug} (from {peer_name})"),
                    chrono::Utc::now(),
                );
            }
        }
    }

    // 4) re-index disk -> SQL to make the pulled skills immediately usable.
    if !importes.is_empty() {
        changes_api::sync_skills_disk_to_sql(&state.memoire).await;
    }

    Json(serde_json::json!({
        "status": "ok",
        "peers_scanned": vus_pairs,
        "imported": importes,
        "count": importes.len(),
    }))
}

/// POST /api/mesh/send {to_id, text} - sends a DM to a peer (resolves the host by ID, POSTs to its
/// /api/mesh/receive). Keeps a local copy (dir=out) for the conversation thread.
pub(crate) async fn api_mesh_send(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let to_id = body["to_id"].as_str().unwrap_or("").to_string();
    let text = body["text"].as_str().unwrap_or("").trim().to_string();
    if to_id.is_empty() || text.is_empty() {
        return Json(serde_json::json!({ "status": "error", "error": "to_id/text required" }));
    }
    let (host, peer_name) = {
        let listener = state.listener.read().await;
        let nodes = listener.get_nodes().await;
        nodes
            .values()
            .find(|n| n.manifest.node_id.map(|i| i.to_string()).as_deref() == Some(to_id.as_str()))
            .map(|n| (n.manifest.host.clone(), n.manifest.node_name.clone().unwrap_or_default()))
            .unwrap_or((String::new(), to_id.clone()))
    };
    if host.is_empty() {
        return Json(serde_json::json!({ "status": "error", "error": "peer not found" }));
    }
    let (my_id, my_name) = {
        let m = state.manifest.read().await;
        (m.node_id.to_string(), m.node_name.clone())
    };
    let client = reqwest::Client::new();
    let url = format!("http://{host}:8419/api/mesh/receive");
    // Encrypt the content if a mesh code is configured (otherwise plaintext, backward-compatible).
    let payload = match sync::seal(&text) {
        Some(enc) => serde_json::json!({ "from_id": my_id, "from_name": my_name, "enc": enc }),
        None => serde_json::json!({ "from_id": my_id, "from_name": my_name, "text": text }),
    };
    let ok = sync::sign_request(client.post(&url), "/api/mesh/receive")
        .json(&payload)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    append_inbox(InboxMessage {
        id: Uuid::new_v4().to_string(),
        peer_id: to_id,
        peer_name,
        dir: "out".into(),
        text,
        ts: chrono::Utc::now().timestamp(),
        read: true,
    });
    Json(serde_json::json!({ "status": if ok { "ok" } else { "local_only" } }))
}

/// POST /api/mesh/receive {from_id, from_name, text} - receives a DM from another LaRuche.
/// Mesh-code auth if configured (otherwise open, historical LAN behavior).
pub(crate) async fn api_mesh_receive(
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if let Some(false) = sync::mesh_auth_ok(&headers, "/api/mesh/receive") {
        return Json(serde_json::json!({ "status": "error", "error": "invalid mesh auth" }));
    }
    let from_id = body["from_id"].as_str().unwrap_or("unknown").to_string();
    let from_name = body["from_name"].as_str().unwrap_or("LaRuche").to_string();
    // Encrypted content (`enc`) -> decrypt; otherwise plaintext (`text`).
    let text = if let Some(enc) = body["enc"].as_str() {
        match sync::open(enc) {
            Some(t) => t.trim().to_string(),
            None => return Json(serde_json::json!({ "status": "error", "error": "decryption failed" })),
        }
    } else {
        body["text"].as_str().unwrap_or("").trim().to_string()
    };
    if text.is_empty() {
        return Json(serde_json::json!({ "status": "error" }));
    }
    append_inbox(InboxMessage {
        id: Uuid::new_v4().to_string(),
        peer_id: from_id,
        peer_name: from_name,
        dir: "in".into(),
        text,
        ts: chrono::Utc::now().timestamp(),
        read: false,
    });
    Json(serde_json::json!({ "status": "ok" }))
}

/// GET /api/inbox - all messages (the client groups by peer).
pub(crate) async fn api_inbox_get() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "messages": read_inbox() }))
}

/// POST /api/inbox/read {peer_id} - marks a peer's messages as read.
pub(crate) async fn api_inbox_read(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let peer = body["peer_id"].as_str().unwrap_or("");
    let mut v = read_inbox();
    for m in v.iter_mut() {
        if m.peer_id == peer {
            m.read = true;
        }
    }
    write_inbox(&v);
    Json(serde_json::json!({ "status": "ok" }))
}
