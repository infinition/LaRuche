//! Memory change-sync (disk-to-SQL skill sync, OKF change import/export, mesh pull) and state version endpoint - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use std::sync::Arc;

/// Delete `capacities.skills.*` nodes that hold NO skill document.
///
/// These are empty shells: the writer and the reader disagreed on `-` versus `_`
/// for a long time, so the same skill was created twice and one of the two copies
/// never received a body. The base carried 88 children for 73 folders. Listing a
/// name that `skill_view` cannot open is worse than not listing it, and the
/// self-improvement loop keeps piling them up if nothing ever sweeps.
///
/// Deliberately CONSERVATIVE: only a node whose every active item lacks
/// `type: skill` is removed. A skill living only in memory (created by the curator,
/// never written to disk) has a document, so it is never touched.
async fn reconcilier_skills_orphelines(memoire: &Arc<dyn laruche_memoire::MemoireCognitive>) {
    let Ok(racine) = memoire.read_node("capacities.skills").await else {
        return;
    };
    let Some(enfants) = racine["children"].as_array() else {
        return;
    };
    let ids: Vec<String> = enfants
        .iter()
        .filter_map(|e| e.get("id").or_else(|| e.get("node_id")))
        .filter_map(|v| v.as_str())
        .map(str::to_string)
        .collect();

    let mut supprimes = 0usize;
    for id in ids {
        let Ok(node) = memoire.read_node(&id).await else {
            continue;
        };
        let a_document = node["items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .any(|it| it["content"].as_str().is_some_and(|c| c.contains("type: skill")))
            })
            .unwrap_or(false);
        if a_document {
            continue;
        }
        // delete_node reparents to `orphans.*`, so remove that residue too.
        if memoire.delete_node(&id).await.is_ok() {
            let suffixe = id.rsplit('.').next().unwrap_or_default();
            let _ = memoire.delete_node(&format!("orphans.{suffixe}")).await;
            supprimes += 1;
        }
    }
    if supprimes > 0 {
        tracing::info!(count = supprimes, "empty skill nodes swept from the catalog");
    }
}

/// Phase 1 - DISK -> SQL sync: scans `skills/*/SKILL.md` and upserts each skill into
/// `capacities.skills.<slug>` (single item), using the SAME id function as the reader.
/// Additive for content (an SQL-only skill is never dropped); it does sweep nodes left
/// without any document, which are shells rather than skills.
pub(crate) async fn sync_skills_disk_to_sql(memoire: &Arc<dyn laruche_memoire::MemoireCognitive>) {
    let dir = std::path::Path::new("skills");
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut n = 0usize;
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(p.join("SKILL.md")) else {
            continue;
        };
        let content = content.replace("\r\n", "\n"); // normalize (SQL in LF)
        if !content.contains("type: skill") {
            continue; // only real OKF skills
        }
        let Some(slug) = p.file_name().and_then(|x| x.to_str()).filter(|s| !s.is_empty()) else {
            continue;
        };
        // Same function as the READER (`skill_view`). Formatting this id by hand is
        // exactly how the two drifted: the folder name went in verbatim while the
        // reader normalised it, and 40 of the 73 shipped skills became unreachable.
        let node_id = laruche_skills::skill_node_id(slug);
        let existing = memoire.read_node(&node_id).await.ok();
        // INCREMENTAL: skip when the SQL copy already matches the disk file. The
        // unconditional delete+rewrite re-embedded every skill at every boot and
        // could consult the write arbiter (aux LLM) per skill; with the LLM busy,
        // startup crawled for minutes. Unchanged file = untouched row.
        let identique = existing
            .as_ref()
            .and_then(|node| node.get("items").and_then(|i| i.as_array()))
            .map(|items| {
                items.len() == 1
                    && items[0].get("content").and_then(|c| c.as_str())
                        == Some(content.as_str())
            })
            .unwrap_or(false);
        if identique {
            continue;
        }
        // Replace the existing item (skill = single item).
        if let Some(node) = existing {
            if let Some(items) = node.get("items").and_then(|i| i.as_array()) {
                for it in items {
                    if let Some(id) = it.get("id").and_then(|x| x.as_str()) {
                        let _ = memoire.delete_item(id, Some("skill-file-sync")).await;
                    }
                }
            }
        }
        let _ = memoire
            .write(
                laruche_memoire::MemoryItem::new(node_id, content).with_source("skill-file"),
            )
            .await;
        n += 1;
    }
    if n > 0 {
        tracing::info!(count = n, "skills synchronized from disk (SKILL.md -> SQL)");
    }
    reconcilier_skills_orphelines(memoire).await;
    // Targeted purge of META-SKILLS from other agent frameworks (third-party/Claude Code/Codex...),
    // wrongly imported: they describe ANOTHER agent, not LaRuche. Explicit DENYLIST: definitely
    // NOT a disk diff "delete everything not on disk" (that would destroy skills
    // created by the agent or seeded in code, like arxiv_search / web_research). Hard-delete:
    // delete_node reparents to `orphans.*`, so we also delete the resulting orphan.
    const META_SKILLS_A_PURGER: &[&str] = &[
        "third-party agent",
        "third-party agent-skill-authoring",
        "claude-code",
        "codex",
        "opencode",
    ];
    let mut purges = 0usize;
    for slug in META_SKILLS_A_PURGER {
        let node_id = laruche_skills::skill_node_id(slug);
        if memoire.read_node(&node_id).await.is_err() {
            continue; // absent -> nothing to do
        }
        if let Ok(r) = memoire.delete_node(&node_id).await {
            purges += 1;
            // delete_node moved it to orphans.<base>_<ts> -> hard-delete this orphan.
            if let Some(orphan) = r.get("relocated_to").and_then(|v| v.as_str()) {
                let _ = memoire.delete_node(orphan).await;
            }
        }
    }
    if purges > 0 {
        tracing::info!(count = purges, "meta-skills from other frameworks purged (denylist)");
    }
}

/// Imports a list of facts `{node_id, content}` into memory (exact dedup). (imported, skipped).
pub(crate) async fn importer_changes(
    state: &Arc<AppState>,
    items: &[serde_json::Value],
    src: &str,
) -> (usize, usize) {
    let (mut imported, mut skipped) = (0usize, 0usize);
    for it in items {
        let node = it["node_id"].as_str().unwrap_or("").trim();
        let content = it["content"].as_str().unwrap_or("");
        if node.is_empty() || content.trim().is_empty() {
            continue;
        }
        // Exact dedup: if an identical item already exists in this node, skip.
        let exists = state
            .memoire
            .grep(content, Some(8))
            .await
            .ok()
            .and_then(|g| {
                g["items"].as_array().map(|a| {
                    a.iter().any(|x| {
                        x["node_id"].as_str() == Some(node) && x["content"].as_str() == Some(content)
                    })
                })
            })
            .unwrap_or(false);
        if exists {
            skipped += 1;
            continue;
        }
        let _ = state
            .memoire
            .write(
                laruche_memoire::MemoryItem::new(node.to_string(), content.to_string())
                    .with_source(src),
            )
            .await;
        imported += 1;
    }
    (imported, skipped)
}

/// GET /api/memory/export_changes?since=<ts> - facts (op=write) written since `since`, for
/// mesh federation (Lever 3, first slice). Excludes system/capacities projections.
pub(crate) async fn api_memory_export_changes(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let since = q.get("since").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let muts = state
        .memoire
        .mutations(Some(250))
        .await
        .unwrap_or_else(|_| serde_json::json!({}));
    let items: Vec<serde_json::Value> = muts["mutations"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|m| {
                    m["op"].as_str() == Some("write")
                        && m["ts"].as_i64().unwrap_or(0) > since
                        && {
                            let n = m["node_id"].as_str().unwrap_or("");
                            !n.starts_with("capacities") && !n.starts_with("system")
                        }
                })
                .map(|m| serde_json::json!({ "node_id": m["node_id"], "content": m["content"], "ts": m["ts"] }))
                .collect()
        })
        .unwrap_or_default();
    let count = items.len();
    Json(serde_json::json!({ "items": items, "count": count }))
}

/// POST /api/memory/import_changes {items:[{node_id,content}], source?} - applies facts (dedup).
pub(crate) async fn api_memory_import_changes(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let src = body["source"].as_str().unwrap_or("mesh").to_string();
    let empty: Vec<serde_json::Value> = vec![];
    let items = body["items"].as_array().unwrap_or(&empty);
    let (imported, skipped) = importer_changes(&state, items, &src).await;
    Json(serde_json::json!({ "imported": imported, "skipped": skipped }))
}

/// POST /api/memory/mesh_pull {peer, since?} - pulls facts from a PEER node (Miel) and imports them
/// locally. First building block of the mesh's COLLECTIVE memory (Lever 3).
pub(crate) async fn api_memory_mesh_pull(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let peer = body["peer"]
        .as_str()
        .unwrap_or("")
        .trim()
        .trim_end_matches('/')
        .to_string();
    if peer.is_empty() {
        return Json(serde_json::json!({ "error": "missing peer (e.g. http://192.168.1.20:8419)" }));
    }
    let since = body["since"].as_i64().unwrap_or(0);
    let url = format!("{peer}/api/memory/export_changes?since={since}");
    let data: serde_json::Value = match reqwest::get(&url).await {
        Ok(r) => match r.json().await {
            Ok(v) => v,
            Err(e) => return Json(serde_json::json!({ "error": format!("peer json: {e}") })),
        },
        Err(e) => return Json(serde_json::json!({ "error": format!("peer contact: {e}") })),
    };
    let empty: Vec<serde_json::Value> = vec![];
    let items = data["items"].as_array().unwrap_or(&empty);
    let src = format!("mesh:{peer}");
    let (imported, skipped) = importer_changes(&state, items, &src).await;
    Json(serde_json::json!({ "pulled_from": peer, "imported": imported, "skipped": skipped }))
}

/// GET /api/state/version - ts of the last memory mutation (P7 lite: the UI polls to know
/// whether to refresh, without a push channel).
pub(crate) async fn api_state_version(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let v = state
        .memoire
        .mutations(Some(1))
        .await
        .ok()
        .and_then(|m| {
            m["mutations"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|x| x["ts"].as_i64())
        })
        .unwrap_or(0);
    Json(serde_json::json!({ "version": v }))
}
