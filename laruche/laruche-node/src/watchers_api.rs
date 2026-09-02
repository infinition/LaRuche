//! Watcher endpoints (list, create, update, delete file/event watchers) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use axum::http::StatusCode;
use std::sync::Arc;

/// GET /api/watchers - list watchers.
pub(crate) async fn api_list_watchers(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let registry = state.watchers.read().await;
    let watchers: Vec<serde_json::Value> = registry
        .list()
        .iter()
        .map(|w| {
            serde_json::json!({
                "id": w.id,
                "name": w.name,
                "watcher_type": w.watcher_type,
                "target": w.target,
                "condition": w.condition,
                "prompt": w.prompt,
                "active": w.active,
                "run_count": w.run_count,
                "profile_id": w.profile_id,
                "model": w.model,
                // Ces quatre-la manquaient, et leur absence ne se voyait pas: le
                // formulaire d'edition les affichait vides, donc « valeur par
                // defaut », et l'enregistrement renvoyait zero. Or zero veut dire
                // « efface, reviens au defaut » cote serveur. Modifier le nom d'une
                // vigie efface son intervalle et son delai de garde, en silence.
                "interval_secs": w.interval_secs,
                "cooldown_secs": w.cooldown_secs,
                "sustained": w.sustained,
                "channel": w.channel,
                // The rule tree carries the correlation edges, so the UI can draw which
                // watcher feeds which without a second endpoint. Without it the graph
                // would have nodes and no links.
                "regles": w.regles,
                "action": w.action,
                // Published verdict, so a node can be shown as currently true or not.
                "verdict": w.dernier_verdict,
                "verdict_depuis": w.verdict_depuis,
            })
        })
        .collect();
    Json(serde_json::json!(watchers))
}

/// POST /api/watchers - create a watcher.
pub(crate) async fn api_create_watcher(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    let name = body["name"]
        .as_str()
        .unwrap_or("Unnamed Watcher")
        .to_string();
    let prompt = body["prompt"]
        .as_str()
        .unwrap_or("Analyze this change")
        .to_string();
    let target = body["target"].as_str().unwrap_or("").to_string();
    let condition = body["condition"].as_str().unwrap_or("").to_string();
    let w_type_str = body["watcher_type"].as_str().unwrap_or("file");

    let watcher_type = match w_type_str {
        "url" => laruche_watchers::WatcherType::Url,
        "log" => laruche_watchers::WatcherType::Log,
        "command" | "commande" => laruche_watchers::WatcherType::Commande,
        _ => laruche_watchers::WatcherType::File,
    };

    let watcher = laruche_watchers::Watcher {
        echecs_consecutifs: 0,
        dernier_verdict: None,
        verdict_depuis: None,
        action: laruche_watchers::Action::default(),
        lignes_vues: None,
        id: Uuid::new_v4(),
        name,
        watcher_type,
        target,
        condition,
        prompt,
        channel: body["channel"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string()),
        active: true,
        created_at: chrono::Utc::now(),
        last_run: None,
        run_count: 0,
        last_state: None,
        model: body["model"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        profile_id: body["profile_id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        interval_secs: body["interval_secs"].as_u64().filter(|n| *n > 0),
        cooldown_secs: body["cooldown_secs"].as_u64().filter(|n| *n > 0),
        sustained: body["sustained"].as_bool().unwrap_or(false),
        regles: match body.get("regles") {
            None | Some(serde_json::Value::Null) => None,
            Some(v) => match serde_json::from_value::<laruche_watchers::Regle>(v.clone()) {
                Ok(r) => Some(r),
                Err(_) => return StatusCode::BAD_REQUEST,
            },
        },
    };

    let log_name = watcher.name.clone();
    let mut registry = state.watchers.write().await;
    registry.add(watcher);
    drop(registry);
    laruche_essaim::feed_journal::record(
        "User",
        "watcher",
        "created the watcher",
        log_name,
        chrono::Utc::now(),
    );
    StatusCode::CREATED
}

/// PATCH /api/watchers/:id - updates a watcher's editable fields. Absent key =
/// field unchanged; model/profile_id set to "" = cleared.
pub(crate) async fn api_update_watcher(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    let watcher_type = body.get("watcher_type").and_then(|v| v.as_str()).map(|s| match s {
        "url" => laruche_watchers::WatcherType::Url,
        "log" => laruche_watchers::WatcherType::Log,
        "command" | "commande" => laruche_watchers::WatcherType::Commande,
        _ => laruche_watchers::WatcherType::File,
    });
    let s = |k: &str| body.get(k).and_then(|v| v.as_str()).map(|v| v.to_string());
    // Key present -> update (empty value = clear for model/profile_id).
    let opt = |k: &str| {
        body.get(k)
            .map(|v| v.as_str().filter(|x| !x.is_empty()).map(|x| x.to_string()))
    };
    let mut registry = state.watchers.write().await;
    // Key present -> update; 0 clears (back to the type default).
    let opt_u64 = |k: &str| {
        body.get(k)
            .map(|v| v.as_u64().filter(|n| *n > 0))
    };
    // regles: key absent = unchanged; null = cleared; object = replaced (400 on bad shape).
    let regles: Option<Option<laruche_watchers::Regle>> = match body.get("regles") {
        None => None,
        Some(serde_json::Value::Null) => Some(None),
        Some(v) => match serde_json::from_value::<laruche_watchers::Regle>(v.clone()) {
            Ok(r) => Some(Some(r)),
            Err(_) => return StatusCode::BAD_REQUEST,
        },
    };
    let ok = registry.update(
        &uuid,
        s("name"),
        watcher_type,
        s("target"),
        s("condition"),
        s("prompt"),
        body.get("active").and_then(|v| v.as_bool()),
        opt("model"),
        opt("profile_id"),
        opt("channel"),
        opt_u64("interval_secs"),
        opt_u64("cooldown_secs"),
        body.get("sustained").and_then(|v| v.as_bool()),
        regles,
    );
    if ok {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

/// DELETE /api/watchers/:id - remove a watcher.
pub(crate) async fn api_delete_watcher(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    if let Ok(uuid) = Uuid::parse_str(&id) {
        let mut registry = state.watchers.write().await;
        if registry.remove(&uuid) {
            return StatusCode::OK;
        }
    }
    StatusCode::NOT_FOUND
}
