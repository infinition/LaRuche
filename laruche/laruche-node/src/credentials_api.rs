//! Credential pool API (list, add, delete shared provider credentials) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use axum::http::StatusCode;
use std::sync::Arc;

// ======================== Credential Pool API ========================

/// GET /api/credentials
pub(crate) async fn api_get_credentials(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let pool = state.credential_pool.read().await;
    Ok(Json(serde_json::json!({
        "credentials": pool.entries
    })))
}

/// POST /api/credentials
pub(crate) async fn api_add_credential(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let provider = body["provider"].as_str().unwrap_or("").trim().to_string();
    let api_key = body["api_key"].as_str().unwrap_or("").trim().to_string();
    let label = body["label"].as_str().map(|s| s.trim().to_string());

    if provider.is_empty() || api_key.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut entry =
        laruche_essaim::credential_pool::CredentialEntry::new(&provider, &api_key, None);
    entry.label = label;

    {
        let mut pool = state.credential_pool.write().await;
        pool.entries.push(entry);
        let _ = std::fs::write(
            &state.credentials_path,
            serde_json::to_string_pretty(&*pool).unwrap(),
        );
    }

    Ok(Json(serde_json::json!({"status": "ok"})))
}

/// DELETE /api/credentials
pub(crate) async fn api_delete_credential(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let provider = body["provider"].as_str().unwrap_or("").trim();
    let api_key = body["api_key"].as_str().unwrap_or("").trim();

    {
        let mut pool = state.credential_pool.write().await;
        let initial_len = pool.entries.len();
        pool.entries
            .retain(|e| !(e.provider == provider && e.api_key == api_key));
        if pool.entries.len() < initial_len {
            let _ = std::fs::write(
                &state.credentials_path,
                serde_json::to_string_pretty(&*pool).unwrap(),
            );
        }
    }

    Ok(Json(serde_json::json!({"status": "ok"})))
}
