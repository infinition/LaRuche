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

/// Flat listing of `racines` resolved under `socle`, sorted so a path always follows
/// its parent, which is what makes the tree readable once the client indents by depth.
///
/// Every entry carries its root ("plugins/x", "mcp/y"): one browser serves both, and a
/// write comes back naming the root it belongs to. The roots themselves are pushed here
/// rather than by the recursion, which only emits a folder when it DESCENDS into one:
/// the directory it is handed never appeared, so mcp/computer_use.py showed up
/// parentless at the top and the plugin folders sat where the roots should have been.
fn lister_fichiers(socle: &std::path::Path, racines: &[&str]) -> Vec<serde_json::Value> {
    fn walk(
        dir: &std::path::Path,
        socle: &std::path::Path,
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
                .strip_prefix(socle)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            if p.is_dir() {
                if e.file_name().to_string_lossy() == "__pycache__" {
                    continue;
                }
                out.push(serde_json::json!({ "path": rel, "dir": true }));
                walk(&p, socle, depth + 1, out);
            } else {
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                out.push(serde_json::json!({ "path": rel, "dir": false, "size": size }));
            }
        }
    }

    let mut out = Vec::new();
    for racine in racines {
        let dossier = socle.join(racine);
        if dossier.exists() {
            out.push(serde_json::json!({ "path": *racine, "dir": true }));
            walk(&dossier, socle, 0, &mut out);
        }
    }
    out.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    out
}

/// GET /api/plugins/files: flat tree of plugins/ and mcp/ (recursive, bounded depth).
pub(crate) async fn api_plugin_files() -> Json<serde_json::Value> {
    let out = lister_fichiers(std::path::Path::new(""), &RACINES);
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


#[cfg(test)]
mod tests {
    use super::{lister_fichiers, plugin_safe_path};

    fn chemins(v: &[serde_json::Value]) -> Vec<String> {
        v.iter()
            .map(|e| {
                format!(
                    "{}{}",
                    e["path"].as_str().unwrap_or(""),
                    if e["dir"].as_bool().unwrap_or(false) { "/" } else { "" }
                )
            })
            .collect()
    }

    #[test]
    fn les_racines_apparaissent_et_chaque_chemin_suit_son_parent() {
        let socle = std::env::temp_dir().join(format!("laruche-listing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&socle);
        std::fs::create_dir_all(socle.join("plugins").join("example_hello")).unwrap();
        std::fs::write(socle.join("plugins/example_hello/plugin.json"), "{}").unwrap();
        std::fs::write(socle.join("plugins/example_hello/run.py"), "x").unwrap();
        std::fs::create_dir_all(socle.join("mcp")).unwrap();
        std::fs::write(socle.join("mcp/computer_use.py"), "y").unwrap();

        let listing = chemins(&lister_fichiers(&socle, &["plugins", "mcp"]));

        assert_eq!(
            listing,
            vec![
                "mcp/",
                "mcp/computer_use.py",
                "plugins/",
                "plugins/example_hello/",
                "plugins/example_hello/plugin.json",
                "plugins/example_hello/run.py",
            ]
        );
        let _ = std::fs::remove_dir_all(&socle);
    }

    #[test]
    fn une_racine_absente_n_apparait_pas() {
        let socle = std::env::temp_dir().join(format!("laruche-vide-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&socle);
        std::fs::create_dir_all(socle.join("plugins")).unwrap();

        let listing = chemins(&lister_fichiers(&socle, &["plugins", "mcp"]));

        assert_eq!(listing, vec!["plugins/"]);
        let _ = std::fs::remove_dir_all(&socle);
    }

    #[test]
    fn un_chemin_sans_racine_connue_est_refuse() {
        assert!(plugin_safe_path("plugins/example_hello/run.py").is_some());
        assert!(plugin_safe_path("mcp/computer_use.py").is_some());
        // No root, escape attempt, or a root that is not whitelisted.
        assert!(plugin_safe_path("run.py").is_none());
        assert!(plugin_safe_path("../secrets.enc").is_none());
        assert!(plugin_safe_path("users/admin.json").is_none());
    }
}
