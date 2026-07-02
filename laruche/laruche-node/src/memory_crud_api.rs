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
                &serde_json::json!({ "node_id": node_id, "content": content, "propose": propose }),
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

    let agent_id = uuid::Uuid::new_v4();
    let registry = state.essaim_registry.clone();
    let state_clone = state.clone();

    let task = format!(
        "You must enrich the cognitive node '{}'.\nHere is the user's request: '{}'.\nRead the node with 'memory_read_node', perform the necessary research, then use 'memory_write' to add your findings to this node.",
        node_id, prompt
    );
    let context = Some(node_id.to_string());

    tokio::spawn(async move {
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
        Ok(value) => Ok(Json(serde_json::json!({ "status": "ok", "result": value }))),
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
                &serde_json::json!({ "item_id": item_id, "action": action }),
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

/// POST /api/memory/consolidate?node=<id> - ACTUALLY merges items (via the aux model).
/// With `node`: consolidates that node. Without: processes overloaded nodes (>=4 items). Old
/// items are soft-deleted (recoverable). This is what the "Consolidate" button triggers.
pub(crate) async fn api_memory_consolidate(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let config = state.essaim_config.read().await.clone();
    let node = q.get("node").map(|s| s.as_str()).filter(|s| !s.is_empty());
    let res = match node {
        Some(n) => {
            // RECURSIVE: consolidate the node AND every node of its subtree - each
            // node individually (merging children INTO the parent would destroy the
            // structure). Before, sub-node items were silently ignored.
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
                                .filter(|id| *id == n || id.starts_with(&prefixe))
                                .map(str::to_string)
                        })
                        .collect()
                })
                .filter(|v: &Vec<String>| !v.is_empty())
                .unwrap_or_else(|| vec![n.to_string()]);
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
