//! Configuration / settings HTTP handlers, split out of main.rs:
//! provider config, per-channel models, runtime generation levers, compaction, context stats.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use std::sync::Arc;

/// {profile, model} options to pick from.
pub(crate) async fn api_get_channel_models(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let profiles = state.profiles.read().await;
    let mut options = Vec::new();
    for (pid, p) in &profiles.profiles {
        for m in &p.models {
            options.push(serde_json::json!({
                "profile_id": pid, "provider": p.provider, "name": p.name, "model": m,
            }));
        }
    }
    Json(serde_json::json!({
        "overrides": serde_json::to_value(&profiles.channel_overrides).unwrap_or_default(),
        "active": { "profile_id": profiles.active_model.profile_id, "model": profiles.active_model.model },
        "options": options,
        "channels": ["telegram", "discord", "slack", "web"],
        // Internal usages routed through the same override map. They are not channels a
        // user talks on, but they each run an LLM, and each deserves its own choice: bulk
        // consolidation can go to a cheap local model while the chat keeps the good one.
        "usages": ["consolidation", "memory-enrich", "feed"],
    }))
}

/// POST /api/config/channel-models: set or clear a per-channel model override.
/// Body: {"channel":"telegram","profile_id":"...","model":"..."}. Empty model/profile clears.
pub(crate) async fn api_save_channel_model(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let channel = body["channel"].as_str().unwrap_or("").to_string();
    if channel.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "channel required"}));
    }
    let model = body["model"].as_str().unwrap_or("").to_string();
    let profile_id = body["profile_id"].as_str().unwrap_or("").to_string();
    {
        let mut profiles = state.profiles.write().await;
        if model.is_empty() || profile_id.is_empty() {
            profiles.channel_overrides.remove(&channel);
        } else {
            profiles
                .channel_overrides
                .insert(channel, profiles::ActiveModel { profile_id, model });
        }
        let _ = profiles::save_profiles(&state.profiles_path, &profiles);
    }
    Json(serde_json::json!({"ok": true}))
}

/// A fallback route as the browser is allowed to see it.
///
/// A route may carry a credential. What leaves the node is the vault REFERENCE when
/// the key is one (`@@NAME`, `${NAME}` or `{{NAME}}`), because a reference is not a
/// secret and the form has to be able to show it and send it back. A literal key is
/// replaced by an empty string: a secret that never reaches the browser cannot be
/// read out of it. `api_key_set` says that one exists, and the setter puts it back
/// when the field returns empty.
fn route_publique(route: &laruche_essaim::config::ProfilSecours) -> serde_json::Value {
    let reference = ["@@", "${", "{{"]
        .iter()
        .any(|marque| route.api_key.contains(marque));
    serde_json::json!({
        "provider": route.provider,
        "model": route.model,
        "api_key": if reference { route.api_key.as_str() } else { "" },
        "api_key_set": !route.api_key.is_empty(),
        "api_base": route.api_base,
        "ollama_url": route.ollama_url,
        "context_max_tokens": route.context_max_tokens,
        "text_tools": route.text_tools,
    })
}

/// GET /api/config/provider: get current LLM provider settings.
pub(crate) async fn api_get_provider_config(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let ec = state.essaim_config.read().await;
    Json(serde_json::json!({
        "provider": ec.provider,
        "api_key_set": !ec.api_key.is_empty(),
        "api_base": ec.api_base,
        "model": ec.model,
        "ollama_url": ec.ollama_url,
        "fallback_models": ec.fallback_models.join(", "),
        "fallback_profiles": ec.fallback_profiles.iter().map(route_publique).collect::<Vec<_>>(),
        "mission_budget_tokens": ec.mission_budget_tokens,
        "review_model": ec.review_model,
        "max_tokens": ec.max_tokens,
        "temperature": ec.temperature,
    }))
}

pub(crate) async fn api_get_context_stats(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let ec = state.essaim_config.read().await;
    let max_messages = ec.context_max_messages;
    let max_tokens = ec.context_max_tokens;

    let session_id = params.get("session_id");
    // Le run est-il TOUJOURS en cours sur cette session ?
    //
    // C'est la seule source de verite. Le client, lui, oublie tout des qu'il
    // recharge, et changer de langue fait justement un location.reload(): on
    // revenait sur une conversation dont le travail tournait encore et le bouton
    // affichait "Envoyer", sans plus aucun moyen d'arreter l'agent. Meme chose en
    // changeant simplement de conversation et en revenant.
    let mut running = false;
    let (messages, used_tokens) = if let Some(sid_str) = session_id {
        if let Ok(sid) = uuid::Uuid::parse_str(sid_str) {
            let session_stats = state
                .essaim_sessions
                .read()
                .await
                .get(&sid)
                .map(|s| (s.messages.len() as u32, s.estimated_tokens() as u32))
                .unwrap_or((0, 0));
            let active_stats = state.active_context_stats.read().await.get(&sid).cloned();
            if let Some(active) = active_stats {
                running = active.running;
                if active.running {
                    (
                        active.messages.max(session_stats.0),
                        active.used_tokens().max(session_stats.1),
                    )
                } else {
                    session_stats
                }
            } else {
                session_stats
            }
        } else {
            (0, 0)
        }
    } else {
        (0, 0)
    };

    let ratio = if max_tokens > 0 {
        used_tokens as f32 / max_tokens as f32
    } else {
        0.0
    };

    Json(serde_json::json!({
        "used": messages,
        "max_messages": max_messages,
        "used_tokens": used_tokens,
        "max_tokens": max_tokens,
        "ratio": ratio,
        "messages": messages,
        "running": running
    }))
}

pub(crate) async fn api_get_compaction_config(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let ec = state.essaim_config.read().await;
    Json(serde_json::json!({
        "context_max_messages": ec.context_max_messages,
        "compaction_threshold": ec.compaction_threshold
    }))
}

pub(crate) async fn api_set_compaction_config(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    {
        let mut ec = state.essaim_config.write().await;
        if let Some(max) = body["context_max_messages"].as_u64() {
            ec.context_max_messages = max as usize;
        }
        if let Some(threshold) = body["compaction_threshold"].as_f64() {
            ec.compaction_threshold = threshold as f32;
        }
    }

    save_persistent_state(&state).await;

    Ok(Json(serde_json::json!({
        "status": "ok"
    })))
}

/// GET /api/config/runtime: HOT-adjustable generation levers (no restart).
pub(crate) async fn api_get_runtime_config(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let ec = state.essaim_config.read().await;
    Json(serde_json::json!({
        "max_iterations": ec.max_iterations,
        "temperature": ec.temperature,
        "max_tokens": ec.max_tokens,
        "tool_selection_limit": ec.tool_selection_limit,
        "dynamic_tool_selection": ec.dynamic_tool_selection,
        "dynamic_context_threshold": ec.dynamic_context_threshold,
        "reactions_agent": ec.reactions_agent,
    }))
}

/// POST /api/config/runtime: updates the provided levers (partial). Hot-reload + persistence.
pub(crate) async fn api_set_runtime_config(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    {
        let mut ec = state.essaim_config.write().await;
        if let Some(v) = body["max_iterations"].as_u64() {
            ec.max_iterations = (v as usize).clamp(1, 200);
        }
        if let Some(v) = body["temperature"].as_f64() {
            ec.temperature = (v as f32).clamp(0.0, 2.0);
        }
        if let Some(v) = body["max_tokens"].as_u64() {
            ec.max_tokens = (v as u32).clamp(256, 32768);
        }
        if let Some(v) = body["tool_selection_limit"].as_u64() {
            ec.tool_selection_limit = (v as usize).clamp(4, 128);
        }
        if let Some(v) = body["dynamic_tool_selection"].as_bool() {
            ec.dynamic_tool_selection = v;
        }
        // Off by default and meant to stay a deliberate choice: it spends prompt
        // budget every turn, and a marker the model emits is one the model can
        // misplace.
        if let Some(v) = body["reactions_agent"].as_bool() {
            ec.reactions_agent = v;
        }
        if let Some(v) = body["dynamic_context_threshold"].as_u64() {
            ec.dynamic_context_threshold = (v as u32).clamp(4_000, 1_000_000);
        }
    }
    save_persistent_state(&state).await;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

/// POST /api/config/provider: update LLM provider settings at runtime.
pub(crate) async fn api_save_provider_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let mut cg = state.essaim_config.write().await;
    // Validated BEFORE anything is written. This setter applies field by field, so a
    // refusal in the middle used to leave the provider changed and the rest dropped.
    // Malformed routes now cost nothing: the request is refused whole.
    let routes_validees = match body.get("fallback_profiles") {
        None => None,
        Some(brut) => {
            let Ok(mut routes) =
                serde_json::from_value::<Vec<laruche_essaim::config::ProfilSecours>>(brut.clone())
            else {
                return Json(serde_json::json!({"error":"invalid fallback_profiles"}));
            };
            if routes.len() > 8
                || !routes.iter().all(|r| {
                    !r.model.trim().is_empty()
                        && !r.provider.trim().is_empty()
                        && r.context_max_tokens >= 256
                })
            {
                return Json(serde_json::json!({"error":"invalid fallback_profiles"}));
            }
            // A literal key never leaves the node, so the form sends it back empty. An
            // empty field therefore means "unchanged", not "cleared": the key stored
            // for the same provider and model is carried over. Clearing one is done by
            // removing the route. Without this, opening Settings and pressing Apply
            // would silently strip every fallback credential.
            for route in &mut routes {
                if route.api_key.trim().is_empty() {
                    if let Some(ancienne) = cg
                        .fallback_profiles
                        .iter()
                        .find(|r| r.provider == route.provider && r.model == route.model)
                    {
                        route.api_key = ancienne.api_key.clone();
                    }
                }
            }
            Some(routes)
        }
    };
    if let Some(routes) = routes_validees {
        cg.fallback_profiles = routes;
    }
    if let Some(provider) = body["provider"].as_str() {
        let p = provider.to_lowercase();
        if matches!(p.as_str(), "ollama" | "openai" | "anthropic") {
            cg.provider = p;
        }
    }
    if let Some(key) = body["api_key"].as_str() {
        cg.api_key = key.to_string();
    }
    if body.get("api_base").is_some() {
        cg.api_base = body["api_base"]
            .as_str()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());
    }
    if let Some(model) = body["model"].as_str() {
        if !model.is_empty() {
            cg.model = model.to_string();
        }
    }
    if let Some(url) = body["ollama_url"].as_str() {
        if !url.is_empty() {
            cg.ollama_url = url.to_string();
        }
    }
    if let Some(budget) = body["mission_budget_tokens"].as_u64() {
        cg.mission_budget_tokens = budget;
    }
    if let Some(fm) = body["fallback_models"].as_str() {
        cg.fallback_models = fm
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    if let Some(mt) = body["max_tokens"].as_u64() {
        cg.max_tokens = mt as u32;
    }
    if let Some(t) = body["temperature"].as_f64() {
        cg.temperature = t as f32;
    }
    if body.get("review_model").is_some() {
        cg.review_model = body["review_model"]
            .as_str()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());
    }
    let result = serde_json::json!({
        "status": "ok",
        "provider": cg.provider,
        "model": cg.model,
    });
    drop(cg);
    save_persistent_state(&state).await;
    Json(result)
}

#[cfg(test)]
mod tests {
    use super::route_publique;
    use laruche_essaim::config::ProfilSecours;

    fn route(api_key: &str) -> ProfilSecours {
        ProfilSecours {
            provider: "openai".into(),
            model: "gpt-4o-mini".into(),
            api_key: api_key.into(),
            api_base: None,
            ollama_url: None,
            context_max_tokens: 128_000,
            text_tools: false,
        }
    }

    /// A literal key does not reach the browser, and the form still knows one exists.
    #[test]
    fn une_cle_litterale_ne_sort_jamais_du_noeud() {
        let vue = route_publique(&route("sk-a-real-secret-value"));
        assert_eq!(vue["api_key"], "");
        assert_eq!(vue["api_key_set"], true);
        assert!(!vue.to_string().contains("sk-a-real-secret-value"));
    }

    /// A vault reference is not a secret: it is shown, so it can be re-saved.
    #[test]
    fn une_reference_de_coffre_reste_visible() {
        for reference in ["@@OPENAI", "${OPENAI}", "{{OPENAI}}"] {
            let vue = route_publique(&route(reference));
            assert_eq!(vue["api_key"], reference);
            assert_eq!(vue["api_key_set"], true);
        }
    }

    /// A route without a credential says so, rather than claiming a hidden one.
    #[test]
    fn une_route_sans_cle_ne_pretend_pas_en_avoir_une() {
        let vue = route_publique(&route(""));
        assert_eq!(vue["api_key"], "");
        assert_eq!(vue["api_key_set"], false);
    }

    /// The extra field the browser receives does not break the round trip.
    #[test]
    fn la_vue_publique_se_relit_comme_une_route() {
        let vue = route_publique(&route("@@OPENAI"));
        let relue: ProfilSecours = serde_json::from_value(vue).unwrap();
        assert_eq!(relue.provider, "openai");
        assert_eq!(relue.api_key, "@@OPENAI");
        assert_eq!(relue.context_max_tokens, 128_000);
    }
}
