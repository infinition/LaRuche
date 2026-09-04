//! Forged Tools API (manifest CRUD + source file browser).

use crate::*;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use std::collections::BTreeSet;
use std::sync::Arc;

// ======================== Forged Tools API ========================

/// Manifest of a forged tool: `forged_tools/<name>/tool.json`. The name is a single path
/// component, so a crafted one cannot climb out of forged_tools/.
fn manifeste(name: &str) -> Option<std::path::PathBuf> {
    if name.is_empty() || name.contains(['/', '\\', ':']) || name.contains("..") {
        return None;
    }
    Some(laruche_essaim::abeilles::forged_tools::chemin_manifeste(
        std::path::Path::new("forged_tools"),
        name,
    ))
}

fn manifeste_lecture(name: &str) -> Option<std::path::PathBuf> {
    let canonical = manifeste(name)?;
    if canonical.exists() {
        return Some(canonical);
    }
    let legacy = laruche_essaim::abeilles::forged_tools::dossier_outil_forge(
        std::path::Path::new("plugins"),
        name,
    )
    .join(laruche_essaim::abeilles::forged_tools::MANIFESTE_HERITE);
    legacy.exists().then_some(legacy)
}

fn recharger(state: &AppState) {
    use laruche_essaim::abeille::ToolOrigin;
    state
        .essaim_registry
        .supprimer_par_origine(ToolOrigin::Forged);
    laruche_essaim::abeilles::forged_tools::charger_outils_herites(
        std::path::Path::new("plugins"),
        &state.essaim_registry,
    );
    laruche_essaim::abeilles::forged_tools::charger_outils_forges(
        std::path::Path::new("forged_tools"),
        &state.essaim_registry,
    );
}

pub(crate) async fn api_forged_tools_list() -> Json<serde_json::Value> {
    let mut names = BTreeSet::new();
    for (root, manifest) in [
        (
            "forged_tools",
            laruche_essaim::abeilles::forged_tools::MANIFESTE,
        ),
        (
            "plugins",
            laruche_essaim::abeilles::forged_tools::MANIFESTE_HERITE,
        ),
    ] {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                if entry.path().is_dir() && entry.path().join(manifest).is_file() {
                    names.insert(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
    }
    Json(serde_json::json!(names
        .into_iter()
        .map(|name| serde_json::json!({ "name": name }))
        .collect::<Vec<_>>()))
}

pub(crate) async fn api_forged_tool_get(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let path = manifeste_lecture(&name).ok_or(StatusCode::NOT_FOUND)?;
    if !path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "content": content })))
}

pub(crate) async fn api_forged_tool_save(
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

    recharger(&state);

    Ok(Json(serde_json::json!({ "status": "ok", "name": name })))
}

pub(crate) async fn api_forged_tool_delete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Remove both representations when the same tool exists in the canonical and
    // legacy roots. Otherwise deleting the canonical copy would reveal the old one
    // again on the next reload.
    let canonical = manifeste(&name).ok_or(StatusCode::BAD_REQUEST)?;
    let legacy = laruche_essaim::abeilles::forged_tools::dossier_outil_forge(
        std::path::Path::new("plugins"),
        &name,
    )
    .join(laruche_essaim::abeilles::forged_tools::MANIFESTE_HERITE);
    let manifests: Vec<_> = [canonical, legacy]
        .into_iter()
        .filter(|path| path.is_file())
        .collect();
    if manifests.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }
    for path in manifests {
        if let Some(dossier) = path.parent() {
            tokio::fs::remove_dir_all(dossier)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    recharger(&state);

    Ok(Json(serde_json::json!({ "status": "ok", "name": name })))
}

// ─── File browser for forged_tools/ and mcp/ ─────────────────────────────────────────────
// View/edit/delete/drop any file under either root. forged_tools/ holds one folder per
// Forged Tool; mcp/ holds the scripts the MCP servers in mcp_servers.json launch. They are
// browsed together because both are code the user maintains by hand, and an MCP script
// parked in forged_tools/ was the only way to reach it from the interface.

/// Roots the browser may touch. Anything outside them is refused.
const RACINES: [&str; 2] = ["forged_tools", "mcp"];

/// Resolves a browser path, which starts with one of `RACINES`, rejecting any escape
/// (`..`, absolute). A path with no recognised root is refused rather than guessed at:
/// silently defaulting to forged_tools/ would let `mcp/x.py` create a stray forged_tools/mcp/x.py.
fn forged_tool_safe_path(rel: &str) -> Option<std::path::PathBuf> {
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
/// Every entry carries its root ("forged_tools/x", "mcp/y"): one browser serves both, and a
/// write comes back naming the root it belongs to. The roots themselves are pushed here
/// rather than by the recursion, which only emits a folder when it DESCENDS into one:
/// the directory it is handed never appeared, so mcp/computer_use.py showed up
/// parentless at the top and the forged_tool folders sat where the roots should have been.
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
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
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

/// GET /api/forged-tool-files: flat tree of forged_tools/ and mcp/ (recursive, bounded depth).
pub(crate) async fn api_forged_tool_files() -> Json<serde_json::Value> {
    let out = lister_fichiers(std::path::Path::new(""), &RACINES);
    Json(serde_json::json!({ "files": out }))
}

/// GET /api/forged-tool-file/*path: content of a file (text, at most 512 KiB).
pub(crate) async fn api_forged_tool_file_get(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let p = forged_tool_safe_path(&path).ok_or(StatusCode::BAD_REQUEST)?;
    let meta = tokio::fs::metadata(&p)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if meta.len() > 512 * 1024 {
        return Ok(Json(
            serde_json::json!({ "binary": true, "size": meta.len() }),
        ));
    }
    match tokio::fs::read_to_string(&p).await {
        Ok(content) => Ok(Json(
            serde_json::json!({ "path": path, "content": content }),
        )),
        Err(_) => Ok(Json(serde_json::json!({ "binary": true }))),
    }
}

/// POST /api/forged-tool-file/*path {content}: creates/writes a file and reloads Forged Tools.
pub(crate) async fn api_forged_tool_file_save(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(path): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let p = forged_tool_safe_path(&path).ok_or(StatusCode::BAD_REQUEST)?;
    let content = body["content"].as_str().unwrap_or("");
    if let Some(parent) = p.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    tokio::fs::write(&p, content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    recharger(&state);
    Ok(Json(serde_json::json!({ "status": "ok", "path": path })))
}

/// DELETE /api/forged-tool-file/*path: deletes a file and reloads Forged Tools.
pub(crate) async fn api_forged_tool_file_delete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let p = forged_tool_safe_path(&path).ok_or(StatusCode::BAD_REQUEST)?;
    if p.is_file() {
        tokio::fs::remove_file(&p)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    recharger(&state);
    Ok(Json(serde_json::json!({ "status": "ok", "path": path })))
}

#[cfg(test)]
mod tests {
    use super::{forged_tool_safe_path, lister_fichiers};

    fn chemins(v: &[serde_json::Value]) -> Vec<String> {
        v.iter()
            .map(|e| {
                format!(
                    "{}{}",
                    e["path"].as_str().unwrap_or(""),
                    if e["dir"].as_bool().unwrap_or(false) {
                        "/"
                    } else {
                        ""
                    }
                )
            })
            .collect()
    }

    #[test]
    fn les_racines_apparaissent_et_chaque_chemin_suit_son_parent() {
        let socle = std::env::temp_dir().join(format!("laruche-listing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&socle);
        std::fs::create_dir_all(socle.join("forged_tools").join("example_hello")).unwrap();
        std::fs::write(socle.join("forged_tools/example_hello/tool.json"), "{}").unwrap();
        std::fs::write(socle.join("forged_tools/example_hello/run.py"), "x").unwrap();
        std::fs::create_dir_all(socle.join("mcp")).unwrap();
        std::fs::write(socle.join("mcp/computer_use.py"), "y").unwrap();

        let listing = chemins(&lister_fichiers(&socle, &["forged_tools", "mcp"]));

        assert_eq!(
            listing,
            vec![
                "forged_tools/",
                "forged_tools/example_hello/",
                "forged_tools/example_hello/run.py",
                "forged_tools/example_hello/tool.json",
                "mcp/",
                "mcp/computer_use.py",
            ]
        );
        let _ = std::fs::remove_dir_all(&socle);
    }

    #[test]
    fn une_racine_absente_n_apparait_pas() {
        let socle = std::env::temp_dir().join(format!("laruche-vide-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&socle);
        std::fs::create_dir_all(socle.join("forged_tools")).unwrap();

        let listing = chemins(&lister_fichiers(&socle, &["forged_tools", "mcp"]));

        assert_eq!(listing, vec!["forged_tools/"]);
        let _ = std::fs::remove_dir_all(&socle);
    }

    #[test]
    fn un_chemin_sans_racine_connue_est_refuse() {
        assert!(forged_tool_safe_path("forged_tools/example_hello/run.py").is_some());
        assert!(forged_tool_safe_path("mcp/computer_use.py").is_some());
        // No root, escape attempt, or a root that is not whitelisted.
        assert!(forged_tool_safe_path("run.py").is_none());
        assert!(forged_tool_safe_path("../secrets.enc").is_none());
        assert!(forged_tool_safe_path("users/admin.json").is_none());
    }
}
