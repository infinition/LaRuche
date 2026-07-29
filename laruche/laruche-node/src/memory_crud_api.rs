//! Cognitive memory CRUD (search, write, enrich, node create/update/move/delete, review, dream, consolidate, grep) - split out of main.rs.

use crate::*;
use axum::extract::{Path, State};
use axum::response::Json;
use axum::http::StatusCode;
use std::sync::Arc;

/// GET /api/memory/search?q=...&limit=8 - search cognitive memory.
pub(crate) async fn api_memory_search(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let query = params.get("q").map(String::as_str).unwrap_or("").trim();
    if query.is_empty() {
        return Ok(Json(serde_json::json!({
            "query": query,
            "raw": { "nodes": [], "items": [] },
            "prompt_text": ""
        })));
    }
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<u8>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(8);
    match state
        .memoire
        .search(
            query,
            laruche_memoire::SearchOpts {
                depth: None,
                limit: Some(limit),
                sans_trace: false,
            },
        )
        .await
    {
        Ok(pack) => {
            let prompt_text = pack.to_prompt_text();
            Ok(Json(serde_json::json!({
                "query": query,
                "raw": pack.raw,
                "prompt_text": prompt_text
            })))
        }
        Err(e) => Ok(Json(serde_json::json!({ "error": e.to_string() }))),
    }
}

/// POST /api/memory/write - write a durable memory item.
pub(crate) async fn api_memory_write(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let node_id = body["node_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let content = body["content"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let propose = body["propose"].as_bool().unwrap_or(false);

    let mut item = laruche_memoire::MemoryItem::new(node_id, content);
    if let Some(source) = body["source"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        item = item.with_source(source);
    }
    if let Some(tags) = body["tags"].as_array() {
        let tags = tags
            .iter()
            .filter_map(|v| v.as_str().map(str::trim))
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        item = item.with_tags(tags);
    }

    let result = if propose {
        state.memoire.propose_write(item).await
    } else {
        state.memoire.write(item).await
    };
    match result {
        Ok(value) => {
            let _ = state.events.write().await.emit(
                laruche_events::EventKind::MemorySaved,
                "api_memory",
                serde_json::json!({ "node_id": node_id, "content": content, "propose": propose }),
            );
            Ok(Json(serde_json::json!({ "status": "ok", "result": value })))
        }
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

/// POST /api/memory/enrich - Spawn an agent to enrich a node
pub(crate) async fn api_memory_enrich(
    State(state): State<Arc<AppState>>,
    _headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let node_id = body["node_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_string();
    let prompt = body["prompt"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_string();
    let item_id = body["item_id"].as_str().map(|s| s.to_string());

    let mut config = state.essaim_config.read().await.clone();
    if let Some(review_model) = &config.review_model {
        if !review_model.trim().is_empty() {
            config.model = review_model.clone();
        }
    }
    // An explicit choice for this usage wins over the review model, which is only a default.
    apply_channel_model(&state, "memory-enrich", &mut config).await;

    let agent_id = uuid::Uuid::new_v4();
    let registry = state.essaim_registry.clone();
    let state_clone = state.clone();

    let task = format!(
        "You must enrich the cognitive node '{}'.\nHere is the user's request: '{}'.\nRead the node with 'memory_read_node', perform the necessary research, then use 'memory_write' to add your findings to this node.",
        node_id, prompt
    );
    let context = Some(node_id.to_string());

    // The Feed only ever showed the memory MUTATIONS ("added an item to episodes"), so an
    // @LaRuche exchange was invisible as an exchange: the question never appeared and the
    // answer looked like an anonymous write. Both sides are recorded here, like a chat turn.
    laruche_essaim::feed_journal::record(
        "User",
        "memory",
        "asked LaRuche about",
        format!("{node_id}: {prompt}"),
        chrono::Utc::now(),
    );

    tokio::spawn(async move {
        // @LaRuche launched from a memory node: it works on the node, so the node is the
        // subject shown in the indicator.
        let _garde = ouvrir_travail(&state_clone, "memoire", &node_id, &config, None);
        tracing::info!(agent_id = %agent_id, task = %task, "Subagent spawned for memory enrichment");
        let _ = state_clone.events.write().await.emit(
            laruche_events::EventKind::AgentStarted,
            "api_memory_enrich",
            serde_json::json!({ "agent_id": agent_id, "node_id": node_id, "item_id": item_id }),
        );

        match laruche_essaim::subagent::lancer_sous_agent(
            &task,
            context.as_deref(),
            registry,
            &config,
        )
        .await
        {
            Ok(result) => {
                tracing::info!(agent_id = %agent_id, "Memory enrichment agent finished");
                if let Some(id) = item_id {
                    let new_content =
                        format!("{}\n\n**LaRuche summary:**\n{}", prompt, result.summary);
                    let _ = state_clone.memoire.update_item(&id, &new_content).await;
                    let _ = state_clone.events.write().await.emit(
                        laruche_events::EventKind::AgentFinished,
                        "api_memory_enrich",
                        serde_json::json!({ "agent_id": agent_id, "item_id": id, "status": "ok" }),
                    );
                    laruche_essaim::feed_journal::record(
                        "LaRuche",
                        "memory",
                        "answered about",
                        format!("{node_id}: {}", result.summary),
                        chrono::Utc::now(),
                    );
                }
            }
            Err(e) => {
                tracing::error!(agent_id = %agent_id, error = %e, "Memory enrichment agent failed");
                if let Some(id) = item_id {
                    let new_content = format!("{}\n\n**LaRuche error:**\n{}", prompt, e);
                    let _ = state_clone.memoire.update_item(&id, &new_content).await;
                    let _ = state_clone.events.write().await.emit(
                        laruche_events::EventKind::AgentFinished,
                        "api_memory_enrich",
                        serde_json::json!({ "agent_id": agent_id, "item_id": id, "status": "error" }),
                    );
                    // A failed run left no trace anywhere: the item just span forever.
                    laruche_essaim::feed_journal::record(
                        "LaRuche",
                        "memory",
                        "failed on",
                        format!("{node_id}: {e}"),
                        chrono::Utc::now(),
                    );
                }
            }
        }
    });

    Ok(Json(
        serde_json::json!({ "status": "ok", "agent_id": agent_id }),
    ))
}

/// GET /api/memory/node/:id - read a cognitive-map node with children and active items.
pub(crate) async fn api_memory_node(
    State(state): State<Arc<AppState>>,
    Path(node_id): Path<String>,
) -> Json<serde_json::Value> {
    match state.memoire.read_node(&node_id).await {
        Ok(value) => Json(serde_json::json!({ "status": "ok", "node": value })),
        Err(e) => Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
    }
}

pub(crate) async fn api_memory_update(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let item_id = body["item_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let content = body["content"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    match state.memoire.update_item(item_id, content).await {
        Ok(value) => {
            // Editing a skill from the memory view has to reach `skills/<slug>/SKILL.md`
            // as well. The disk is the master: the boot sync reads it back into SQL and
            // overwrites what differs, so an edit that stopped at the database looked
            // saved and was gone at the next start.
            if let Some(node_id) = value
                .get("node_id")
                .and_then(|v| v.as_str())
                .filter(|id| id.starts_with("capacities.skills."))
            {
                laruche_essaim::abeilles::memoire::ecrire_skill_md(node_id, content);
            }
            Ok(Json(serde_json::json!({ "status": "ok", "result": value })))
        }
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

pub(crate) async fn api_memory_delete(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let item_id = body["item_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let reason = body["reason"].as_str();
    match state.memoire.delete_item(item_id, reason).await {
        Ok(value) => Ok(Json(serde_json::json!({ "status": "ok", "result": value }))),
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

pub(crate) async fn api_memory_node_delete(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let node_id = body["node_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    // Same rule as the skills API: the folder goes with the node, or the boot sync reads
    // it back in and the deleted skill returns.
    if let Some(slug) = node_id.strip_prefix("capacities.skills.") {
        if !slug.is_empty() && !slug.contains(['/', '\\', ':', '.']) && !slug.contains("..") {
            let _ = std::fs::remove_dir_all(std::path::Path::new("skills").join(slug));
        }
    }
    match state.memoire.delete_node(node_id).await {
        Ok(value) => Ok(Json(serde_json::json!({ "status": "ok", "result": value }))),
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

pub(crate) async fn api_memory_node_create(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let node_id = body["node_id"].as_str().unwrap_or("");
    let label = body["label"].as_str().unwrap_or("");
    let one_liner = body["one_liner"].as_str();
    let importance = body["importance"].as_f64().map(|f| f as f32);
    let source = body["source"].as_str();
    if node_id.is_empty() || label.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    match state
        .memoire
        .create_node(node_id, label, one_liner, importance, source)
        .await
    {
        Ok(value) => Ok(Json(serde_json::json!({ "status": "ok", "result": value }))),
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

pub(crate) async fn api_memory_node_update(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let node_id = body["node_id"].as_str().unwrap_or("");
    if node_id.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let label = body["label"].as_str();
    let one_liner = body["one_liner"].as_str();
    let importance = body["importance"].as_f64().map(|f| f as f32);

    match state
        .memoire
        .update_node(node_id, label, one_liner, importance)
        .await
    {
        Ok(value) => Ok(Json(serde_json::json!({ "status": "ok", "result": value }))),
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

/// POST /api/memory/node/move - reparents a node (drag&drop in the tree). body
/// `{node_id, new_parent}`; empty `new_parent` => root node. Moves the whole subtree
/// (id rename). Rejects system nodes and cycles (moving into its own subtree).
pub(crate) async fn api_memory_node_move(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let old = body["node_id"]
        .as_str()
        .map(|s| s.trim().trim_matches('.'))
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let new_parent = body["new_parent"]
        .as_str()
        .unwrap_or("")
        .trim()
        .trim_matches('.');
    let last = old.rsplit('.').next().unwrap_or(old);
    let new_id = if new_parent.is_empty() {
        last.to_string()
    } else {
        format!("{new_parent}.{last}")
    };
    let prot = |s: &str| {
        s == "system"
            || s == "capacities"
            || s.starts_with("system.")
            || s.starts_with("capacities.")
    };
    if prot(old) || prot(&new_id) {
        return Ok(Json(
            serde_json::json!({ "status": "error", "error": "system node cannot be moved" }),
        ));
    }
    if new_id == old || new_id.starts_with(&format!("{old}.")) {
        return Ok(Json(
            serde_json::json!({ "status": "error", "error": "invalid move (cycle or identical)" }),
        ));
    }
    match state.memoire.renommer_sous_arbre(old, &new_id).await {
        Ok(n) => Ok(Json(
            serde_json::json!({ "status": "ok", "result": { "moved_to": new_id, "nodes": n } }),
        )),
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

pub(crate) async fn api_memory_move(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let item_id = body["item_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let node_id = body["node_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    match state.memoire.move_item(item_id, node_id).await {
        Ok(value) => Ok(Json(serde_json::json!({ "status": "ok", "result": value }))),
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

pub(crate) async fn api_memory_review(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let item_id = body["item_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let action = body["action"]
        .as_str()
        .map(str::trim)
        .filter(|s| matches!(*s, "accept" | "reject"))
        .ok_or(StatusCode::BAD_REQUEST)?;
    let reason = body["reason"].as_str();
    match state.memoire.review_item(item_id, action, reason).await {
        Ok(value) => {
            let _ = state.events.write().await.emit(
                laruche_events::EventKind::MemoryReviewed,
                "api_memory",
                serde_json::json!({ "item_id": item_id, "action": action }),
            );
            Ok(Json(serde_json::json!({ "status": "ok", "result": value })))
        }
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

pub(crate) async fn api_memory_proposed(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<u8>().ok())
        .filter(|v| *v > 0);
    match state.memoire.list_proposed(limit).await {
        Ok(value) => Json(serde_json::json!({ "status": "ok", "result": value })),
        Err(e) => Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
    }
}

pub(crate) async fn api_memory_suggest(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let query = params.get("q").map(String::as_str).unwrap_or("").trim();
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<u8>().ok())
        .filter(|v| *v > 0);
    match state.memoire.suggest_nodes(query, limit).await {
        Ok(value) => Json(serde_json::json!({ "status": "ok", "result": value })),
        Err(e) => Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
    }
}

/// POST /api/memory/dream - trigger active memory consolidation.
pub(crate) async fn api_memory_dream(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let mut dream = state
        .memoire
        .dream()
        .await
        .unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() }));
    // Same wiring as the periodic pass: duplicate suggestions become actionable
    // proposals in the Reine queue (human click = dedup applied).
    let mut enqueued = 0usize;
    for s in dream
        .get("suggestions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
    {
        if s.get("kind").and_then(|k| k.as_str()) != Some("duplicate") {
            continue;
        }
        let (Some(node_id), Some(message)) = (
            s.get("node_id").and_then(|v| v.as_str()),
            s.get("message").and_then(|v| v.as_str()),
        ) else {
            continue;
        };
        if laruche_essaim::reine_queue::proposer_hygiene(node_id, message) {
            enqueued += 1;
        }
    }
    if let Some(obj) = dream.as_object_mut() {
        obj.insert("proposals_enqueued".into(), serde_json::json!(enqueued));
    }
    Json(dream)
}

/// Virtual node the tree shows above the non-system roots. It exists nowhere in the
/// database: it is a handle for "everything the agent accumulates", so one click can
/// act on episodes, projects, people and the rest without walking them one by one.
pub(crate) const NOEUD_MEMOIRE: &str = "@memory";

/// Root segments that belong to the SYSTEM group, excluded from `@memory`.
const RACINES_SYSTEME: [&str; 3] = ["system", "capacities", "tools"];

/// True when `id` hangs off one of the system roots.
fn est_systeme(id: &str) -> bool {
    let racine = id.split('.').next().unwrap_or(id);
    RACINES_SYSTEME.contains(&racine)
}

/// POST /api/memory/consolidate?node=<id> - ACTUALLY merges items (via the aux model).
/// With `node`: consolidates that node. With `@memory`: every node outside the system
/// group. Without: processes overloaded nodes (>=4 items). Old items are soft-deleted
/// (recoverable). This is what the "Consolidate" button triggers.
pub(crate) async fn api_memory_consolidate(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    // Consolidation is a bulk, low-stakes rewrite: it deserves its own model choice, so a
    // cheap local one can grind through the memory while the chat keeps the good one.
    let mut config = state.essaim_config.read().await.clone();
    apply_channel_model(&state, "consolidation", &mut config).await;
    let _garde = ouvrir_travail(&state, "curateur", "consolidation", &config, None);
    let node = q.get("node").map(|s| s.as_str()).filter(|s| !s.is_empty());
    let res = match node {
        Some(n) => {
            // RECURSIVE: consolidate the node AND every node of its subtree - each
            // node individually (merging children INTO the parent would destroy the
            // structure). Before, sub-node items were silently ignored.
            let virtuel = n == NOEUD_MEMOIRE;
            let prefixe = format!("{n}.");
            let cibles: Vec<String> = state
                .memoire
                .list_nodes()
                .await
                .ok()
                .and_then(|v| v.as_array().cloned())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| {
                            x.get("node_id")
                                .or_else(|| x.get("id"))
                                .and_then(|v| v.as_str())
                                .filter(|id| {
                                    if virtuel {
                                        !est_systeme(id)
                                    } else {
                                        *id == n || id.starts_with(&prefixe)
                                    }
                                })
                                .map(str::to_string)
                        })
                        .collect()
                })
                .filter(|v: &Vec<String>| !v.is_empty())
                // A real node with no listing still deserves an attempt; the virtual one
                // has nothing to fall back on, so an empty map means nothing to do.
                .unwrap_or_else(|| if virtuel { Vec::new() } else { vec![n.to_string()] });
            let mut rapports = Vec::new();
            for cible in &cibles {
                match laruche_essaim::brain::consolider_node(&state.memoire, &config, cible).await
                {
                    Ok(r) => rapports.push(r),
                    Err(e) => rapports
                        .push(serde_json::json!({ "node_id": cible, "error": e.to_string() })),
                }
            }
            Ok(serde_json::json!({ "node_id": n, "nodes_traites": cibles.len(), "rapports": rapports }))
        }
        None => laruche_essaim::brain::consolider_memoire(&state.memoire, &config).await,
    };
    Json(res.unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() })))
}

/// GET /api/memory/grep?q=<texte>&limit=30 - substring search in item content.
pub(crate) async fn api_memory_grep(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let pattern = q.get("q").cloned().unwrap_or_default();
    let limit = q.get("limit").and_then(|s| s.parse::<u8>().ok());
    Json(
        state
            .memoire
            .grep(&pattern, limit)
            .await
            .unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() })),
    )
}

#[cfg(test)]
mod tests {
    use super::est_systeme;

    #[test]
    fn le_groupe_systeme_est_hors_du_noeud_memoire() {
        for id in ["system", "system.soul", "capacities", "capacities.skills.maps", "tools", "tools.web_fetch"] {
            assert!(est_systeme(id), "{id} devrait etre systeme");
        }
    }

    #[test]
    fn tout_ce_que_l_agent_accumule_en_fait_partie() {
        for id in ["episodes", "episodes.2026-07-28.mission", "projects", "people.fabien", "orphans", "decisions"] {
            assert!(!est_systeme(id), "{id} ne devrait pas etre systeme");
        }
    }

    #[test]
    fn un_prefixe_qui_ressemble_a_une_racine_systeme_n_en_est_pas_une() {
        // Segment comparison, not string prefix: `systemes` is a node like any other.
        assert!(!est_systeme("systemes"));
        assert!(!est_systeme("toolsmith.notes"));
    }
}
