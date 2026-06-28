//! Knowledge endpoints - split out of main.rs.

use crate::*;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Json};
use axum::http::StatusCode;
use std::sync::Arc;

// ======================== Knowledge Endpoints ========================

/// GET /api/knowledge: list knowledge base entries.
pub(crate) async fn api_list_knowledge(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Json<serde_json::Value> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let is_admin = if let Some(uid) = caller {
        state
            .users
            .read()
            .await
            .get(&uid)
            .map(|u| u.role == auth_user::UserRole::Admin)
            .unwrap_or(false)
    } else {
        false
    };
    let kb = state.essaim_kb.read().await;
    let entries: Vec<serde_json::Value> = kb
        .entries
        .iter()
        .filter(|e| {
            // Admin sees all, users see global + own
            is_admin || e.user_id.is_none() || e.user_id == caller
        })
        .map(|e| {
            serde_json::json!({
                "id": e.id,
                "text": e.text,
                "source": e.source,
                "created_at": e.created_at,
                "user_id": e.user_id,
            })
        })
        .collect();
    Json(serde_json::json!({
        "count": entries.len(),
        "entries": entries,
    }))
}

/// POST /api/knowledge: add a knowledge entry.
pub(crate) async fn api_add_knowledge(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let text = body["text"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    let source = body["source"].as_str();
    // Admin entries are global (user_id=None), user entries are private
    let is_admin = if let Some(uid) = caller {
        state
            .users
            .read()
            .await
            .get(&uid)
            .map(|u| u.role == auth_user::UserRole::Admin)
            .unwrap_or(false)
    } else {
        false
    };
    let entry_user_id = if is_admin { None } else { caller };

    let mut kb = state.essaim_kb.write().await;
    match kb.add_with_user(text, source, entry_user_id).await {
        Ok(id) => Ok(Json(serde_json::json!({"id": id, "status": "added"}))),
        Err(e) => Ok(Json(serde_json::json!({"error": e.to_string()}))),
    }
}

/// PUT /api/knowledge/:id: update a knowledge entry.
pub(crate) async fn api_update_knowledge(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let text = body["text"].as_str().unwrap_or("");
    let source = body["source"].as_str();
    if text.is_empty() {
        return Json(serde_json::json!({"error": "text is required"}));
    }
    let mut kb = state.essaim_kb.write().await;
    match kb.update(&id, text, source).await {
        Ok(true) => Json(serde_json::json!({"status": "updated", "id": id})),
        Ok(false) => Json(serde_json::json!({"error": "Entry not found"})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

/// DELETE /api/knowledge/:id: remove a knowledge entry.
pub(crate) async fn api_delete_knowledge(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    let mut kb = state.essaim_kb.write().await;
    if kb.remove(&id) {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}
