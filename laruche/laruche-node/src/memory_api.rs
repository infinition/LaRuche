//! Memory endpoints (cognitive-map tree, stats, mutations, OKF and zip export, OKF import) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use axum::http::StatusCode;
use std::sync::Arc;

/// GET /api/memory/tree - lightweight cognitive-map snapshot for the SPA.
pub(crate) async fn api_memory_tree(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let health = state.memoire.health().await.unwrap_or(false);
    let dream = state
        .memoire
        .dream()
        .await
        .unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() }));
    // Real tree from the backend (the Obsidian UI rebuilds the hierarchy on the
    // dotted ids). Fallback to the base roots if the backend does not list (sidecar).
    let mut nodes = state
        .memoire
        .list_nodes()
        .await
        .unwrap_or_else(|_| serde_json::json!([]));
    if nodes.as_array().map(|a| a.is_empty()).unwrap_or(true) {
        nodes = serde_json::json!([
            { "id": "people", "label": "People", "one_liner": "People and preferences" },
            { "id": "projects", "label": "Projects", "one_liner": "Active projects" },
            { "id": "decisions", "label": "Decisions", "one_liner": "Durable choices" },
            { "id": "capacities", "label": "Capacites", "one_liner": "Tools, plugins, MCP, skills" },
            { "id": "missions", "label": "Missions", "one_liner": "Long-running research" },
            { "id": "sessions", "label": "Sessions", "one_liner": "Conversational context" },
            { "id": "knowledge", "label": "Knowledge", "one_liner": "Imported knowledge" }
        ]);
    }
    // Mark system-managed nodes (tools.*/system.*): the UI shows a 🔒 and
    // the agent cannot mutate them (`noeud_reserve` guard). The admin can edit them.
    if let Some(arr) = nodes.as_array_mut() {
        for n in arr.iter_mut() {
            if let Some(id) = n.get("id").and_then(|v| v.as_str()) {
                let prot = id == "capacities"
                    || id == "system"
                    || id == "tools"
                    || id.starts_with("capacities.")
                    || id.starts_with("system.")
                    || id.starts_with("tools.");
                n["protected"] = serde_json::json!(prot);
            }
        }
    }
    Json(serde_json::json!({
        "health": health,
        "nodes": nodes,
        "review": dream.get("suggestions").cloned().unwrap_or_else(|| serde_json::json!([])),
        "audit": dream
    }))
}

/// GET /api/memory/stats - counts (items/nodes/mutations) for the SPA.
pub(crate) async fn api_memory_stats(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(
        state
            .memoire
            .stats()
            .await
            .unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() })),
    )
}

/// GET /api/memory/mutations?limit=50 - audit log of recent memory mutations.
pub(crate) async fn api_memory_mutations(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let limit = q.get("limit").and_then(|s| s.parse::<u8>().ok());
    Json(
        state
            .memoire
            .mutations(limit)
            .await
            .unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() })),
    )
}

/// GET /api/memory/export_okf?dir=okf-export&node=<id> - exports an OKF bundle into a server
/// folder. Optional `node` = exports only that node + its subtree (otherwise the whole map).
pub(crate) async fn api_memory_export_okf(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let dir = q
        .get("dir")
        .cloned()
        .unwrap_or_else(|| "okf-export".to_string());
    let node = q.get("node").map(|s| s.as_str()).filter(|s| !s.is_empty());
    match state
        .memoire
        .export_okf(std::path::Path::new(&dir), node)
        .await
    {
        Ok(n) => Json(serde_json::json!({ "ok": true, "files": n, "dir": dir })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
}

/// Recursively zips a folder's content into an in-memory buffer (entries relative to `base`).
fn zip_dir_to_bytes(base: &std::path::Path) -> std::io::Result<Vec<u8>> {
    use std::io::{Cursor, Write};
    let mut buf = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(Cursor::new(&mut buf));
        let opts: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut stack = vec![base.to_path_buf()];
        while let Some(d) = stack.pop() {
            for entry in std::fs::read_dir(&d)? {
                let path = entry?.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Ok(rel) = path.strip_prefix(base) {
                    let name = rel.to_string_lossy().replace('\\', "/");
                    zw.start_file(name, opts)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                    zw.write_all(&std::fs::read(&path)?)?;
                }
            }
        }
        zw.finish()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    }
    Ok(buf)
}

/// GET /api/memory/export.zip?node=<id> - exports the OKF bundle and returns it as a
/// DOWNLOADABLE .zip (Content-Disposition), without writing anything durable on the project side.
/// Optional `node` = current node + subtree; otherwise the whole memory.
pub(crate) async fn api_memory_export_zip(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Response, StatusCode> {
    use axum::response::IntoResponse;
    let node = q.get("node").map(|s| s.as_str()).filter(|s| !s.is_empty());
    // Isolated temporary folder (cleaned up after reading).
    let tmp = std::env::temp_dir().join(format!("laruche-okf-{}", Uuid::new_v4()));
    let result = state
        .memoire
        .export_okf(&tmp, node)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    let bytes =
        result.and_then(|_| zip_dir_to_bytes(&tmp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
    let _ = std::fs::remove_dir_all(&tmp); // best-effort cleanup
    let bytes = bytes?;
    let fname = match node {
        Some(n) => format!("okf-{}.zip", n.replace('.', "_")),
        None => "okf-memoire.zip".to_string(),
    };
    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/zip".to_string(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{fname}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}

/// POST /api/memory/import_okf?dir=okf-export - imports an OKF bundle.
pub(crate) async fn api_memory_import_okf(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let dir = q
        .get("dir")
        .cloned()
        .unwrap_or_else(|| "okf-export".to_string());
    match state.memoire.import_okf(std::path::Path::new(&dir)).await {
        Ok(n) => Json(serde_json::json!({ "ok": true, "imported": n, "dir": dir })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
}
