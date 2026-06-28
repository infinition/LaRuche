//! Skill endpoints (list, get, upsert, toggle, delete agent skills) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use std::sync::Arc;

pub(crate) async fn api_list_skills(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let disabled = state.essaim_config.read().await.disabled_skills.clone();
    let mut out: Vec<serde_json::Value> = Vec::new();
    if let Ok(root) = state.memoire.read_node("capacities.skills").await {
        if let Some(children) = root["children"].as_array() {
            for child in children {
                let id = child["id"].as_str().or_else(|| child["node_id"].as_str());
                let Some(id) = id else { continue };
                let name = id
                    .strip_prefix("capacities.skills.")
                    .unwrap_or(id)
                    .to_string();
                // Load the content to extract the description.
                let mut description = child["label"].as_str().unwrap_or("").to_string();
                if let Ok(node) = state.memoire.read_node(id).await {
                    if let Some(items) = node["items"].as_array() {
                        if let Some(body) = items.iter().rev().find_map(|it| {
                            it["content"].as_str().filter(|c| c.contains("type: skill"))
                        }) {
                            if let Ok(sk) = laruche_skills::Skill::parse(body) {
                                description = sk.meta.description.clone();
                            }
                        }
                    }
                }
                out.push(serde_json::json!({
                    "name": name,
                    "description": description,
                    "enabled": !disabled.iter().any(|d| d == &name),
                }));
            }
        }
    }
    out.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });
    Json(serde_json::json!(out))
}

/// GET /api/skills/:name - returns the full SKILL.md (OKF).
pub(crate) async fn api_get_skill(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let node_id = laruche_skills::skill_node_id(&name);
    if let Ok(node) = state.memoire.read_node(&node_id).await {
        if let Some(items) = node["items"].as_array() {
            if let Some(body) = items
                .iter()
                .rev()
                .find_map(|it| it["content"].as_str().filter(|c| c.contains("type: skill")))
            {
                return Json(serde_json::json!({"name": name, "content": body}));
            }
        }
    }
    Json(serde_json::json!({"error": "not found"}))
}

/// POST /api/skills - creates/updates a skill (body: {content} OKF, or {name, content}).
pub(crate) async fn api_upsert_skill(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if auth_user::extract_user_from_headers(&headers, &state.cookie_secret).is_none() {
        return Json(serde_json::json!({"error": "unauthorized"}));
    }
    let content = body["content"].as_str().unwrap_or("");
    let sk = match laruche_skills::Skill::parse(content) {
        Ok(s) if !s.meta.name.trim().is_empty() => s,
        _ => {
            return Json(
                serde_json::json!({"error": "invalid frontmatter (name/description required, type: skill)"}),
            )
        }
    };
    let node_id = laruche_skills::skill_node_id(&sk.meta.name);
    match state
        .memoire
        .write(laruche_memoire::MemoryItem::new(node_id, content).with_source("skills-ui"))
        .await
    {
        Ok(_) => Json(serde_json::json!({"status": "ok", "name": sk.meta.name})),
        Err(e) => Json(serde_json::json!({"error": format!("{e}")})),
    }
}

/// POST /api/skills/:name/toggle - enables/disables a skill (persisted).
pub(crate) async fn api_toggle_skill(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    if auth_user::extract_user_from_headers(&headers, &state.cookie_secret).is_none() {
        return Json(serde_json::json!({"error": "unauthorized"}));
    }
    let enabled = {
        let mut cfg = state.essaim_config.write().await;
        if let Some(pos) = cfg.disabled_skills.iter().position(|d| d == &name) {
            cfg.disabled_skills.remove(pos);
            true
        } else {
            cfg.disabled_skills.push(name.clone());
            false
        }
    };
    save_persistent_state(&state).await;
    Json(serde_json::json!({"status": "ok", "name": name, "enabled": enabled}))
}

/// DELETE /api/skills/:name - deletes the skill (node items) + cleans up the state.
pub(crate) async fn api_delete_skill(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    if auth_user::extract_user_from_headers(&headers, &state.cookie_secret).is_none() {
        return Json(serde_json::json!({"error": "unauthorized"}));
    }
    let node_id = laruche_skills::skill_node_id(&name);
    let _ = state.memoire.delete_node(&node_id).await;
    {
        let mut cfg = state.essaim_config.write().await;
        cfg.disabled_skills.retain(|d| d != &name);
    }
    save_persistent_state(&state).await;
    Json(serde_json::json!({"status": "ok"}))
}
