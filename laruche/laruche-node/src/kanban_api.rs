//! Kanban board endpoints (task list/create/update/status/dependency/delete, default channel, known channels) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use axum::http::StatusCode;
use std::sync::Arc;

/// GET /api/kanban - list all tasks
pub(crate) async fn api_kanban_list(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let board = state.kanban_board.read().await;
    Json(serde_json::json!(board.list()))
}

/// POST /api/kanban - create task
pub(crate) async fn api_kanban_create(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    let title = body["title"].as_str().unwrap_or("").to_string();
    let description = body["description"].as_str().unwrap_or("").to_string();
    let idempotency_key = body["idempotency_key"].as_str().map(|s| s.to_string());

    let profile_id = body["profile_id"].as_str().map(|s| s.to_string());
    let model = body["model"].as_str().map(|s| s.to_string());
    let channel = body["channel"]
        .as_str()
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty());
    let log_title = title.clone();
    let mut board = state.kanban_board.write().await;
    board.create(title, description, idempotency_key, profile_id, model, channel);
    drop(board);
    laruche_essaim::feed_journal::record(
        "User",
        "kanban",
        "created the kanban task",
        log_title,
        chrono::Utc::now(),
    );
    StatusCode::CREATED
}

/// GET /api/channels/known - known REAL channels (to populate the dropdowns).
/// Aggregates: home channel + cron channels + kanban default/tasks + watchers. Deduplicated.
pub(crate) async fn api_channels_known(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    use std::collections::BTreeSet;
    let mut set: BTreeSet<String> = BTreeSet::new();
    let mut push = |c: Option<String>| {
        if let Some(c) = c {
            let c = c.trim().to_string();
            if !c.is_empty() {
                set.insert(c);
            }
        }
    };
    let home = state.essaim_config.read().await.home_channel.clone();
    push(home.clone());
    for t in state.essaim_cron.read().await.list() {
        push(t.channel.clone());
    }
    {
        let board = state.kanban_board.read().await;
        push(board.default_channel());
        for t in board.list() {
            push(t.channel.clone());
        }
    }
    for w in state.watchers.read().await.list() {
        push(w.channel.clone());
    }

    // Channels CONFIGURED in channels-config.json, on top of those already in use. Without
    // them the list was empty on a fresh install: nothing referenced a channel yet, so the
    // picker offered nothing, so no task could be given one. Slack stayed invisible this
    // way even once configured, and `memory` never appeared at all.
    if let Ok(contenu) = std::fs::read_to_string("channels-config.json") {
        if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&contenu) {
            for nom in ["telegram", "discord", "slack"] {
                let bloc = &cfg[nom];
                // A block counts as configured when it exists and is not explicitly off.
                let present = bloc.is_object() && bloc["enabled"].as_bool().unwrap_or(true);
                if present {
                    push(Some(nom.to_string()));
                }
            }
        }
    }
    // Always offered: it needs no external service and no configuration, LaRuche writes
    // the result into its own cognitive memory.
    push(Some(crate::CANAL_MEMOIRE.to_string()));

    Json(serde_json::json!({
        "channels": set.into_iter().collect::<Vec<_>>(),
        "home": home,
    }))
}

/// GET /api/kanban/default_channel - board's default channel.
pub(crate) async fn api_kanban_default_channel_get(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let ch = state.kanban_board.read().await.default_channel();
    Json(serde_json::json!({ "channel": ch }))
}

/// GET /api/kanban/interval - secondes entre deux releves de la colonne Ready.
pub(crate) async fn api_kanban_interval_get(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let secs = state.kanban_board.read().await.delai_secs();
    Json(serde_json::json!({ "seconds": secs }))
}

/// POST /api/kanban/interval {seconds} - regle ce delai.
pub(crate) async fn api_kanban_interval_set(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let demande = body["seconds"].as_u64().unwrap_or(laruche_kanban::DELAI_DEFAUT);
    state.kanban_board.write().await.set_delai_secs(demande);
    // On rend la valeur RETENUE et pas celle demandee: elle est bornee, et
    // l'interface doit afficher ce qui s'applique vraiment.
    let secs = state.kanban_board.read().await.delai_secs();
    Json(serde_json::json!({ "seconds": secs }))
}

/// POST /api/kanban/default_channel {channel} - sets the board's default channel.
pub(crate) async fn api_kanban_default_channel_set(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    let ch = body["channel"].as_str().map(|s| s.to_string());
    state.kanban_board.write().await.set_default_channel(ch);
    StatusCode::OK
}

/// PUT /api/kanban/:id/status - update status
pub(crate) async fn api_kanban_update_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    if let Ok(uuid) = Uuid::parse_str(&id) {
        let status_str = body["status"].as_str().unwrap_or("");
        let status = match status_str {
            "Triage" => laruche_kanban::TaskStatus::Triage,
            "Todo" => laruche_kanban::TaskStatus::Todo,
            "Ready" => laruche_kanban::TaskStatus::Ready,
            "Running" => laruche_kanban::TaskStatus::Running,
            "Blocked" => laruche_kanban::TaskStatus::Blocked,
            "Done" => laruche_kanban::TaskStatus::Done,
            "Archived" => laruche_kanban::TaskStatus::Archived,
            _ => return StatusCode::BAD_REQUEST,
        };
        let mut board = state.kanban_board.write().await;
        if board.change_status(uuid, status) {
            return StatusCode::OK;
        }
    }
    StatusCode::NOT_FOUND
}

/// PUT /api/kanban/:id - update title/description.
pub(crate) async fn api_kanban_update(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    if let Ok(uuid) = Uuid::parse_str(&id) {
        let title = body["title"].as_str().map(|s| s.to_string());
        let description = body["description"].as_str().map(|s| s.to_string());
        let mut board = state.kanban_board.write().await;
        // Per-task channel: present in the body (even empty) -> apply it (empty = inherit default).
        if body.get("channel").is_some() {
            board.set_channel(uuid, body["channel"].as_str().map(|s| s.to_string()));
        }
        if board.update(uuid, title, description).is_some() {
            return StatusCode::OK;
        }
    }
    StatusCode::NOT_FOUND
}

/// POST /api/kanban/:id/dependency - block child by parent
pub(crate) async fn api_kanban_add_dependency(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    if let Ok(child_uuid) = Uuid::parse_str(&id) {
        if let Some(parent_str) = body["parent_id"].as_str() {
            if let Ok(parent_uuid) = Uuid::parse_str(parent_str) {
                let mut board = state.kanban_board.write().await;
                if board.add_dependency(child_uuid, parent_uuid) {
                    return StatusCode::OK;
                }
            }
        }
    }
    StatusCode::NOT_FOUND
}

/// DELETE /api/kanban/:id
pub(crate) async fn api_kanban_delete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    if let Ok(uuid) = Uuid::parse_str(&id) {
        let mut board = state.kanban_board.write().await;
        if board.remove(&uuid) {
            return StatusCode::OK;
        }
    }
    StatusCode::NOT_FOUND
}
