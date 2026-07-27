//! Plugins API (plugin CRUD + plugin file browser) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use axum::http::StatusCode;
use std::sync::Arc;

// ======================== Plugins API ========================

/// Manifest of a plugin: `plugins/<name>/plugin.json`. The name is a single path
/// component, so a crafted one cannot climb out of plugins/.
fn manifeste(name: &str) -> Option<std::path::PathBuf> {
    if name.is_empty() || name.contains(['/', '\\', ':']) || name.contains("..") {
        return None;
    }
    Some(laruche_essaim::abeilles::plugins::chemin_manifeste(
        std::path::Path::new("plugins"),
        name,
    ))
}

pub(crate) async fn api_plugin_get(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let path = manifeste(&name).ok_or(StatusCode::BAD_REQUEST)?;
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
    let path = manifeste(&name).ok_or(StatusCode::BAD_REQUEST)?;
    if let Some(dossier) = path.parent() {
        tokio::fs::create_dir_all(dossier).await.ok();
    }
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
    // The folder is the plugin: manifest and scripts go together.
    let path = manifeste(&name).ok_or(StatusCode::BAD_REQUEST)?;
    if let Some(dossier) = path.parent() {
        if dossier.is_dir() {
            tokio::fs::remove_dir_all(dossier)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    // Reload plugins
    let plugins_dir = std::path::Path::new("plugins");
    laruche_essaim::abeilles::plugins::charger_plugins(plugins_dir, &state.essaim_registry);

    Ok(Json(serde_json::json!({ "status": "ok", "name": name })))
}

// ─── File browser for plugins/ and mcp/ ─────────────────────────────────────────────
// View/edit/delete/drop any file under either root. plugins/ holds one folder per
// plugin; mcp/ holds the scripts the MCP servers in mcp_servers.json launch. They are
// browsed together because both are code the user maintains by hand, and an MCP script
// parked in plugins/ was the only way to reach it from the interface.

/// Roots the browser may touch. Anything outside them is refused.
const RACINES: [&str; 2] = ["plugins", "mcp"];
// Anti-traversal guard: every path is confined to plugins/.

/// Resolves a browser path, which starts with one of `RACINES`, rejecting any escape
/// (`..`, absolute). A path with no recognised root is refused rather than guessed at:
/// silently defaulting to plugins/ would let `mcp/x.py` create a stray plugins/mcp/x.py.
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
    let racine = rel.split(['/', '\\']).next()?;
    if !RACINES.contains(&racine) {
        return None;
    }
    Some(std::path::PathBuf::from(rel))
}

/// GET /api/plugins/files: flat tree of plugins/ and mcp/ (recursive, bounded depth).
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
    // Paths are emitted root-first ("plugins/x", "mcp/y") so one browser can serve both
    // and every write comes back naming the root it belongs to.
    let mut out = Vec::new();
    for racine in RACINES {
        let base = std::path::Path::new(racine);
        if base.exists() {
            walk(base, std::path::Path::new(""), 0, &mut out);
        }
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

