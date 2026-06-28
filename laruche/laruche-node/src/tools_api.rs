//! Tool registry endpoints (list tools, get/save tool enablement config) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use axum::http::StatusCode;
use std::sync::Arc;

pub(crate) async fn api_list_tools(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let disabled = state.essaim_config.read().await.disabled_tools.clone();
    let tools = match state.essaim_registry.schema_complet() {
        serde_json::Value::Array(items) => items
            .into_iter()
            .map(|mut tool| {
                if let Some(name) = tool
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
                {
                    let enabled = !disabled.iter().any(|t| t == &name);
                    if let Some(obj) = tool.as_object_mut() {
                        obj.insert("enabled".to_string(), serde_json::json!(enabled));
                        if let Some(abeille) = state.essaim_registry.get(&name) {
                            obj.insert(
                                "danger".to_string(),
                                serde_json::to_value(abeille.niveau_danger())
                                    .unwrap_or_else(|_| serde_json::json!("safe")),
                            );
                            obj.insert(
                                "origin".to_string(),
                                serde_json::to_value(abeille.origin())
                                    .unwrap_or_else(|_| serde_json::json!("builtin")),
                            );
                        }
                    }
                }
                tool
            })
            .collect(),
        _ => Vec::new(),
    };
    Json(serde_json::Value::Array(tools))
}

/// GET/POST /api/tools/config - enable/disable Abeilles for prompt injection/execution.
pub(crate) async fn api_get_tools_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let disabled = state.essaim_config.read().await.disabled_tools.clone();
    Json(serde_json::json!({ "disabled_tools": disabled }))
}

pub(crate) async fn api_save_tools_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let disabled = body["disabled_tools"]
        .as_array()
        .ok_or(StatusCode::BAD_REQUEST)?
        .iter()
        .filter_map(|v| v.as_str().map(str::trim))
        .filter(|name| !name.is_empty() && state.essaim_registry.get(name).is_some())
        .map(str::to_string)
        .collect::<Vec<_>>();

    {
        let mut cfg = state.essaim_config.write().await;
        cfg.disabled_tools = disabled.clone();
    }
    save_persistent_state(&state).await;
    Ok(Json(
        serde_json::json!({ "status": "ok", "disabled_tools": disabled }),
    ))
}
