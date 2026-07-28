//! Blueprint endpoints (list/create/delete parameterized cron automation templates, instantiate) - split out of main.rs.

use crate::*;
use axum::extract::{Path, State};
use axum::response::Json;
use axum::http::StatusCode;
use std::sync::Arc;

// -- Blueprints: parameterized cron automation templates ------------------------
// Built-in catalogue (laruche_essaim::blueprints::catalogue) + blueprints CREATED by
// the user, persisted in `blueprints.json`.

fn load_user_blueprints() -> Vec<laruche_essaim::blueprints::Blueprint> {
    std::fs::read_to_string("blueprints.json")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_user_blueprints(bps: &[laruche_essaim::blueprints::Blueprint]) -> std::io::Result<()> {
    std::fs::write(
        "blueprints.json",
        serde_json::to_string_pretty(bps).unwrap_or_else(|_| "[]".into()),
    )
}

/// GET /api/blueprints - built-in catalogue + user blueprints.
pub(crate) async fn get_blueprints() -> Json<Vec<laruche_essaim::blueprints::Blueprint>> {
    let mut all = laruche_essaim::blueprints::catalogue();
    all.extend(load_user_blueprints());
    Json(all)
}

/// POST /api/blueprints - creates (or updates) a user blueprint. Body = Blueprint
/// {id, title, schedule_template, prompt_template, slots:[{name,label,default}]}.
pub(crate) async fn api_create_blueprint(
    Json(mut bp): Json<laruche_essaim::blueprints::Blueprint>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if bp.id.trim().is_empty() {
        // derive an id from the title
        let slug: String = bp
            .title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let slug = slug.trim_matches('-').to_string();
        bp.id = if slug.is_empty() {
            format!("bp-{}", Uuid::new_v4())
        } else {
            slug
        };
    }
    // Forbid overwriting a built-in blueprint.
    if laruche_essaim::blueprints::catalogue()
        .iter()
        .any(|b| b.id == bp.id)
    {
        return Err(StatusCode::CONFLICT);
    }
    let mut users = load_user_blueprints();
    users.retain(|b| b.id != bp.id); // upsert
    users.push(bp.clone());
    save_user_blueprints(&users).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "status": "ok", "id": bp.id })))
}

/// DELETE /api/blueprints/:id - deletes a user blueprint (built-ins are immutable).
pub(crate) async fn api_delete_blueprint(Path(id): Path<String>) -> Json<serde_json::Value> {
    let mut users = load_user_blueprints();
    let before = users.len();
    users.retain(|b| b.id != id);
    let removed = before - users.len();
    let _ = save_user_blueprints(&users);
    Json(serde_json::json!({ "status": "ok", "removed": removed }))
}

/// POST /api/blueprints/:id/instancier - instantiates a blueprint into a REAL cron.
/// Body = slot values: `{ "<slot>": "<value>", ... }` (or `{slots:{...}}`).
pub(crate) async fn instancier_blueprint(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut all = laruche_essaim::blueprints::catalogue();
    all.extend(load_user_blueprints());
    let Some(bp) = all.into_iter().find(|b| b.id == id) else {
        return Err(StatusCode::NOT_FOUND);
    };
    // Accepts {slots:{...}} or a flat object of values.
    let src = body.get("slots").filter(|v| v.is_object()).unwrap_or(&body);
    let mut valeurs: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Some(obj) = src.as_object() {
        for (k, v) in obj {
            if let Some(s) = v.as_str() {
                valeurs.insert(k.clone(), s.to_string());
            }
        }
    }
    let (name, cron_expr, prompt) = laruche_essaim::blueprints::instancier(&bp, &valeurs);
    let extras = laruche_essaim::blueprints::instancier_extras(&bp, &valeurs);
    // Routing and model come from the body, never from the slots: a blueprint templates
    // WHAT runs and WHEN, not where the answer goes nor which model answers. Hardcoded to
    // None, an instantiated task always landed on the activity log with the default
    // model, and three of the six fields of the manual form were unreachable this way.
    let champ = |cle: &str| -> Option<String> {
        body.get(cle)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    // The blueprint says WHICH kind of thing it is; the same slots then feed a scheduled
    // task, a watcher or a piece of research. Before, everything became a cron task,
    // which is why watching a page or opening an investigation had no starting point.
    use laruche_essaim::blueprints::Cible;
    match bp.cible {
        Cible::Cron => {
            let task = ScheduledTask {
                id: Uuid::new_v4(),
                name,
                prompt,
                cron_expr: Some(cron_expr),
                fire_at: None,
                channel: champ("channel"),
                provider: champ("provider"),
                model: champ("model"),
                profile_id: champ("profile_id"),
                skills: vec![],
                enabled: true,
                created_at: chrono::Utc::now(),
                last_run: None,
                run_count: 0,
            };
            let cron_id = {
                let mut cron = state.essaim_cron.write().await;
                cron.add(task)
            };
            Ok(Json(
                serde_json::json!({ "status": "ok", "cible": "cron", "cron_id": cron_id }),
            ))
        }
        Cible::Watcher => {
            let lire = |cle: &str| extras.get(cle).cloned().unwrap_or_default();
            let watcher_type = match lire("watcher_type").as_str() {
                "url" => laruche_watchers::WatcherType::Url,
                "log" => laruche_watchers::WatcherType::Log,
                "command" | "commande" => laruche_watchers::WatcherType::Commande,
                _ => laruche_watchers::WatcherType::File,
            };
            let watcher = laruche_watchers::Watcher {
                id: Uuid::new_v4(),
                name,
                watcher_type,
                target: lire("target"),
                condition: lire("condition"),
                prompt,
                channel: champ("channel"),
                model: champ("model"),
                profile_id: champ("profile_id"),
                active: true,
                created_at: chrono::Utc::now(),
                last_run: None,
                run_count: 0,
                last_state: None,
                lignes_vues: None,
                action: laruche_watchers::Action::default(),
                echecs_consecutifs: 0,
                dernier_verdict: None,
                verdict_depuis: None,
                // Left at their defaults: the watcher machinery fills them on the first
                // poll, and a blueprint has no business fixing a polling interval.
                interval_secs: None,
                cooldown_secs: None,
                sustained: false,
                regles: None,
            };
            let id = watcher.id;
            state.watchers.write().await.add(watcher);
            Ok(Json(
                serde_json::json!({ "status": "ok", "cible": "watcher", "watcher_id": id }),
            ))
        }
        Cible::Recherche => {
            // The prompt IS the objective; the schedule, empty by default, becomes the
            // optional cadence, so the same blueprint serves a one-off and a recurring one.
            let cadence = (!cron_expr.trim().is_empty()).then_some(cron_expr);
            let corps = serde_json::json!({
                "objective": prompt,
                "slug": name,
                "cadence": cadence,
                "channel": champ("channel"),
                "provider": champ("provider"),
                "model": champ("model"),
                "profile_id": champ("profile_id"),
            });
            // Creating a mission demands admin rights: the caller's headers travel with
            // the call rather than being forged here.
            let reponse = crate::missions_api::api_create_mission(
                State(state.clone()),
                headers.clone(),
                axum::extract::Json(corps),
            )
            .await;
            Ok(Json(serde_json::json!({
                "status": "ok", "cible": "recherche", "mission": reponse.0
            })))
        }
    }
}
