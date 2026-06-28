//! Plugins API (plugin CRUD + plugin file browser) - split out of main.rs.

use crate::*;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Json};
use axum::http::StatusCode;
use std::sync::Arc;

// ======================== Plugins API ========================

pub(crate) async fn api_plugin_get(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let path = std::path::Path::new("plugins").join(format!("{}.json", name));
    if !path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "content": content })))
}

pub(crate) async fn api_plugin_save(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let content = body["content"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    let path = std::path::Path::new("plugins").join(format!("{}.json", name));
    tokio::fs::create_dir_all("plugins").await.ok();
    tokio::fs::write(&path, content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Reload plugins
    let plugins_dir = std::path::Path::new("plugins");
    laruche_essaim::abeilles::plugins::charger_plugins(plugins_dir, &state.essaim_registry);

    Ok(Json(serde_json::json!({ "status": "ok", "name": name })))
}

pub(crate) async fn api_plugin_delete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let path = std::path::Path::new("plugins").join(format!("{}.json", name));
    if path.exists() {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Reload plugins
    let plugins_dir = std::path::Path::new("plugins");
    laruche_essaim::abeilles::plugins::charger_plugins(plugins_dir, &state.essaim_registry);

    Ok(Json(serde_json::json!({ "status": "ok", "name": name })))
}

// ─── File browser for the plugins/ folder (+ scripts/) ──────────────────────────────
// View/edit/delete/drop your own scripts (.py/.ps1/.sh/.json...) in addition to JSON.
// Anti-traversal guard: every path is confined to plugins/.

/// Resolves a relative path INSIDE plugins/, rejecting any escape (`..`, absolute).
fn plugin_safe_path(rel: &str) -> Option<std::path::PathBuf> {
    let rel = rel.trim_start_matches(['/', '\\']);
    if rel.is_empty() {
        return None;
    }
    for comp in std::path::Path::new(rel).components() {
        use std::path::Component::*;
        match comp {
            Normal(_) | CurDir => {}
            _ => return None, // ParentDir, RootDir, Prefix → refus
        }
    }
    Some(std::path::Path::new("plugins").join(rel))
}

/// GET /api/plugins/files: flat tree of plugins/ files (recursive, bounded depth).
pub(crate) async fn api_plugin_files() -> Json<serde_json::Value> {
    fn walk(
        dir: &std::path::Path,
        base: &std::path::Path,
        depth: usize,
        out: &mut Vec<serde_json::Value>,
    ) {
        if depth > 3 {
            return;
        }
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let rel = p
                .strip_prefix(base)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            if p.is_dir() {
                if e.file_name().to_string_lossy() == "__pycache__" {
                    continue;
                }
                out.push(serde_json::json!({ "path": rel, "dir": true }));
                walk(&p, base, depth + 1, out);
            } else {
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                out.push(serde_json::json!({ "path": rel, "dir": false, "size": size }));
            }
        }
    }
    let base = std::path::Path::new("plugins");
    let mut out = Vec::new();
    if base.exists() {
        walk(base, base, 0, &mut out);
    }
    out.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    Json(serde_json::json!({ "files": out }))
}

/// GET /api/plugins/file/*path: content of a file (text, ≤ 512 KiB).
pub(crate) async fn api_plugin_file_get(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let p = plugin_safe_path(&path).ok_or(StatusCode::BAD_REQUEST)?;
    let meta = tokio::fs::metadata(&p).await.map_err(|_| StatusCode::NOT_FOUND)?;
    if meta.len() > 512 * 1024 {
        return Ok(Json(serde_json::json!({ "binary": true, "size": meta.len() })));
    }
    match tokio::fs::read_to_string(&p).await {
        Ok(content) => Ok(Json(serde_json::json!({ "path": path, "content": content }))),
        Err(_) => Ok(Json(serde_json::json!({ "binary": true }))),
    }
}

/// POST /api/plugins/file/*path {content}: creates/writes a file. Reloads the plugins.
pub(crate) async fn api_plugin_file_save(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(path): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let p = plugin_safe_path(&path).ok_or(StatusCode::BAD_REQUEST)?;
    let content = body["content"].as_str().unwrap_or("");
    if let Some(parent) = p.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    tokio::fs::write(&p, content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    laruche_essaim::abeilles::plugins::charger_plugins(
        std::path::Path::new("plugins"),
        &state.essaim_registry,
    );
    Ok(Json(serde_json::json!({ "status": "ok", "path": path })))
}

/// DELETE /api/plugins/file/*path: deletes a file. Reloads the plugins.
pub(crate) async fn api_plugin_file_delete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let p = plugin_safe_path(&path).ok_or(StatusCode::BAD_REQUEST)?;
    if p.is_file() {
        tokio::fs::remove_file(&p)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    laruche_essaim::abeilles::plugins::charger_plugins(
        std::path::Path::new("plugins"),
        &state.essaim_registry,
    );
    Ok(Json(serde_json::json!({ "status": "ok", "path": path })))
}

