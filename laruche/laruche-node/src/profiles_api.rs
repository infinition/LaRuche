//! Provider profiles + codex + active model + capabilities API - split out of main.rs.

use crate::*;
use axum::extract::{Path, State};
use axum::response::Json;
use axum::http::StatusCode;
use std::sync::Arc;


// ======================== Provider Profiles API ========================

/// GET /api/profiles: list all profiles.
pub(crate) async fn api_get_profiles(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Require auth to access profiles (contain API keys)
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let cfg = state.profiles.read().await;
    // Mask API keys: show only last 4 chars
    let mut profiles_map = serde_json::to_value(&cfg.profiles).unwrap_or_default();
    if let Some(obj) = profiles_map.as_object_mut() {
        for (_id, profile) in obj.iter_mut() {
            if let Some(key) = profile.get("api_key").and_then(|k| k.as_str()) {
                if key.len() > 4 {
                    let masked = format!("{}...{}", &key[..4], &key[key.len() - 4..]);
                    profile["api_key"] = serde_json::json!(masked);
                }
            }
        }
    }
    Ok(Json(serde_json::json!({
        "profiles": profiles_map,
        "active_model": cfg.active_model,
    })))
}

/// POST /api/profiles/:id/test: send a minimal request to verify this profile's
/// credentials and endpoint actually work. Returns `{ok, status, message}` so the UI
/// shows "Connected" or the exact provider error (e.g. a 429 "insufficient balance"
/// from z.ai) instead of the user digging through logs.
pub(crate) async fn api_test_profile(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Resolve provider/model/key/base for THIS profile (same mapping as the worker).
    let (provider, model, api_key, api_base, ollama_url) = {
        let cfg = state.profiles.read().await;
        let Some(profile) = cfg.profiles.get(&id) else {
            return Ok(Json(serde_json::json!({"ok": false, "message": "unknown profile"})));
        };
        // Prefer the active model if this profile is active, else its first listed model.
        let model = if cfg.active_model.profile_id == id {
            cfg.active_model.model.clone()
        } else {
            profile.models.first().cloned().unwrap_or_default()
        };
        if model.is_empty() {
            return Ok(Json(serde_json::json!({
                "ok": false,
                "message": "no model configured for this profile"
            })));
        }
        let ollama_url = if profile.provider == "ollama" {
            profile.base_url.clone()
        } else {
            cfg.profiles
                .values()
                .find(|p| p.provider == "ollama")
                .map(|p| p.base_url.clone())
                .unwrap_or_else(|| "http://127.0.0.1:11434".to_string())
        };
        let api_base = if profile.provider != "ollama" {
            Some(profile.base_url.clone())
        } else {
            None
        };
        (
            profile.provider.clone(),
            model,
            profile.api_key.clone(),
            api_base,
            ollama_url,
        )
    };

    // Substitute a `@@secret/${NAME}` vault reference before the call.
    let api_key = laruche_essaim::secrets::substituer(&api_key);

    let messages = vec![serde_json::json!({"role": "user", "content": "ping"})];
    let res = laruche_essaim::providers::provider_chat_stream(
        &provider,
        &model,
        &messages,
        0.0,
        8,
        &api_key,
        api_base.as_deref(),
        &ollama_url,
        None,
    )
    .await;

    match res {
        Ok(mut stream) => {
            use futures_util::StreamExt;
            // Pull one chunk to confirm the stream actually flows.
            let _ = stream.next().await;
            Ok(Json(serde_json::json!({
                "ok": true,
                "model": model,
                "message": "Connected"
            })))
        }
        Err(e) => {
            // Surface the real HTTP status + a body excerpt (e.g. z.ai code 1113).
            if let Some(pe) = e.downcast_ref::<laruche_essaim::providers::ProviderError>() {
                let body: String = pe.body.chars().take(300).collect();
                Ok(Json(serde_json::json!({
                    "ok": false,
                    "status": pe.status,
                    "message": body.trim()
                })))
            } else {
                Ok(Json(serde_json::json!({
                    "ok": false,
                    "message": e.to_string()
                })))
            }
        }
    }
}

/// POST /api/profiles: create or update a profile (auth required).
pub(crate) async fn api_upsert_profile(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let id = match body["id"].as_str() {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => return Ok(Json(serde_json::json!({"error": "missing id"}))),
    };
    let provider = body["provider"].as_str().unwrap_or("ollama").to_string();
    let name = body["name"].as_str().unwrap_or(&id).to_string();
    let base_url = body["base_url"].as_str().unwrap_or("").to_string();
    let soumise = body["api_key"].as_str().unwrap_or("").to_string();
    let models: Vec<String> = body["models"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let max_context_length = body["max_context_length"]
        .as_u64()
        .map(|v| v as u32)
        .unwrap_or_else(|| match provider.as_str() {
            "anthropic" => 200000,
            "codex" => 128000,
            "openai" => 128000,
            _ => 32768,
        });

    let mut cfg = state.profiles.write().await;
    let ancien = cfg.profiles.get(&id);

    // La cle n'est JAMAIS rendue en clair par l'API: la liste la masque en
    // `sk-1...ab42`. Le formulaire ne peut donc pas la reafficher, et il la
    // renvoie vide quand la personne n'y a pas touche. Prendre cette valeur vide
    // au pied de la lettre effacait la cle a chaque enregistrement: on venait
    // ajouter un modele, et le profil se retrouvait sans identifiants, avec une
    // erreur qui parle de configuration manquante et rien qui dise qu'on vient
    // soi-meme de la supprimer.
    //
    // Donc: vide veut dire "inchangee". Pour retirer une cle, on efface le
    // profil ou on en met une autre. Une valeur masquee renvoyee telle quelle
    // est traitee pareil, sinon on stockerait `sk-1...ab42` comme cle.
    let masque_renvoye = ancien
        .map(|a| a.api_key.len() > 4 && soumise == format!("{}...{}", &a.api_key[..4], &a.api_key[a.api_key.len() - 4..]))
        .unwrap_or(false);
    let api_key = if soumise.is_empty() || masque_renvoye {
        ancien.map(|a| a.api_key.clone()).unwrap_or_default()
    } else {
        soumise
    };

    // Meme raisonnement pour le partage sur le maillage: le formulaire ne
    // l'expose pas, donc il ne doit pas pouvoir le remettre a zero en passant.
    let visibilite = ancien.map(|a| a.visibilite).unwrap_or_default();
    let allowed_peers = ancien.map(|a| a.allowed_peers.clone()).unwrap_or_default();

    let profile = profiles::ProviderProfile {
        provider,
        name: name.clone(),
        base_url,
        api_key,
        models,
        visibilite,
        allowed_peers,
        max_context_length,
    };

    cfg.profiles.insert(id.clone(), profile);

    // Auto-discover Ollama models if provider is ollama
    if cfg.profiles.get(&id).map(|p| p.provider.as_str()) == Some("ollama") {
        let base = cfg.profiles[&id].base_url.clone();
        drop(cfg);
        let models = profiles::discover_ollama_models(&base).await;
        let mut cfg = state.profiles.write().await;
        if !models.is_empty() {
            if let Some(p) = cfg.profiles.get_mut(&id) {
                p.models = models;
            }
        }
        let _ = profiles::save_profiles(&state.profiles_path, &cfg);
        drop(cfg);
    } else {
        let _ = profiles::save_profiles(&state.profiles_path, &cfg);
        drop(cfg);
    }

    // Sync essaim config from active profile
    sync_essaim_from_profiles(&state).await;

    Ok(Json(
        serde_json::json!({"status": "ok", "id": id, "name": name}),
    ))
}

/// DELETE /api/profiles/:id: delete a profile (auth required).
pub(crate) async fn api_delete_profile(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let mut cfg = state.profiles.write().await;
    if cfg.profiles.remove(&id).is_some() {
        // If we deleted the active profile, fall back to first available
        if cfg.active_model.profile_id == id {
            if let Some(first_id) = cfg.profiles.keys().next().cloned() {
                let first_model = cfg.profiles[&first_id]
                    .models
                    .first()
                    .cloned()
                    .unwrap_or_default();
                cfg.active_model = profiles::ActiveModel {
                    profile_id: first_id,
                    model: first_model,
                };
            }
        }
        let _ = profiles::save_profiles(&state.profiles_path, &cfg);
        drop(cfg);
        sync_essaim_from_profiles(&state).await;
        Ok(Json(serde_json::json!({"status": "ok"})))
    } else {
        Ok(Json(serde_json::json!({"error": "profile not found"})))
    }
}

// ─── ChatGPT Codex auth (OAuth subscription) for the web UI ─────────────────
//
// The device code flow is asynchronous: `start` launches the connection in a
// background task and immediately returns the URL + the code to display; the
// frontend then polls `status` until `connected`. On success, a "codex" provider
// profile is auto-created for one-click use.

#[derive(Clone, Serialize, Default)]
struct CodexLoginStatus {
    phase: String, // idle | pending | connected | error
    verification_url: String,
    user_code: String,
    message: String,
    account_id: Option<String>,
}

fn codex_login_cell() -> &'static std::sync::Mutex<CodexLoginStatus> {
    static CELL: std::sync::OnceLock<std::sync::Mutex<CodexLoginStatus>> =
        std::sync::OnceLock::new();
    CELL.get_or_init(|| std::sync::Mutex::new(CodexLoginStatus::default()))
}

fn codex_set_status(f: impl FnOnce(&mut CodexLoginStatus)) {
    if let Ok(mut s) = codex_login_cell().lock() {
        f(&mut s);
    }
}

/// Models supported by Codex with a ChatGPT account (subscription).
/// Keep this independent of the public API catalog: ChatGPT subscription access is
/// entitlement-based and accepts the general GPT-5.6 model variants on this backend.
const CODEX_CHATGPT_MODELS: &[&str] = &["gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna"];

/// Auto-creates (or updates) the "codex" provider profile (ChatGPT subscription).
pub(crate) async fn ensure_codex_profile(state: &Arc<AppState>) {
    let id = "codex-chatgpt";
    let models: Vec<String> = CODEX_CHATGPT_MODELS.iter().map(|s| s.to_string()).collect();
    let mut cfg = state.profiles.write().await;
    match cfg.profiles.get_mut(id) {
        Some(p) => {
            // Existing profile: refresh the model list + base URL
            // (fixes a profile created with old unsupported models).
            p.provider = "codex".to_string();
            p.base_url = laruche_essaim::codex_auth::DEFAULT_CODEX_BASE_URL.to_string();
            p.models = models.clone();
        }
        None => {
            cfg.profiles.insert(
                id.to_string(),
                profiles::ProviderProfile {
                    provider: "codex".to_string(),
                    name: "ChatGPT Codex".to_string(),
                    base_url: laruche_essaim::codex_auth::DEFAULT_CODEX_BASE_URL.to_string(),
                    api_key: String::new(),
                    models,
                    visibilite: Default::default(), allowed_peers: Vec::new(),
                    max_context_length: 128000,
                },
            );
        }
    }
    // A profile refresh used to leave the global active selection on an old model
    // (`gpt-5.4-mini`), even though that model had just disappeared from the picker.
    // Land on the flagship current option when the active Codex model is no longer
    // part of the supported subscription list.
    let codex_is_active = cfg.active_model.profile_id == id;
    if codex_is_active
        && !CODEX_CHATGPT_MODELS.contains(&cfg.active_model.model.as_str())
    {
        cfg.active_model.model = CODEX_CHATGPT_MODELS[0].to_string();
    }
    let _ = profiles::save_profiles(&state.profiles_path, &cfg);
    drop(cfg);
    if codex_is_active {
        // Keep the legacy Essaim runtime view aligned with the profile source of truth.
        sync_essaim_from_profiles(state).await;
    }
}

/// GET /api/auth/codex/status: ChatGPT Codex connection state.
pub(crate) async fn api_codex_status(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let live = codex_login_cell()
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default();
    // An in-progress (pending) or errored login takes priority over the stored state.
    if live.phase == "pending" || live.phase == "error" {
        return Ok(Json(serde_json::to_value(&live).unwrap_or_default()));
    }
    // Otherwise, reflect the persisted tokens.
    match laruche_essaim::codex_auth::read_codex_tokens() {
        Some(t) => {
            // A connected account must always have the matching provider card. This also
            // refreshes model IDs after upgrades without forcing a logout/login cycle.
            ensure_codex_profile(&state).await;
            let acct = laruche_essaim::codex_auth::account_id_from_token(&t.access_token);
            Ok(Json(serde_json::json!({
                "phase": "connected",
                "account_id": acct,
                "expiring": laruche_essaim::codex_auth::access_token_is_expiring(&t.access_token, 60),
            })))
        }
        None => Ok(Json(serde_json::json!({"phase": "idle"}))),
    }
}

/// POST /api/auth/codex/start: starts the device code flow, returns URL + code.
pub(crate) async fn api_codex_start(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    codex_set_status(|s| {
        *s = CodexLoginStatus {
            phase: "pending".into(),
            message: "Initializing...".into(),
            ..Default::default()
        };
    });

    let (tx, rx) = tokio::sync::oneshot::channel::<(String, String)>();
    let state_bg = state.clone();
    tokio::spawn(async move {
        let res = laruche_essaim::codex_auth::device_code_login(move |url, code| {
            codex_set_status(|s| {
                s.phase = "pending".into();
                s.verification_url = url.to_string();
                s.user_code = code.to_string();
                s.message = "Waiting for sign-in in the browser...".into();
            });
            let _ = tx.send((url.to_string(), code.to_string()));
        })
        .await;
        match res {
            Ok(tokens) => {
                let _ = laruche_essaim::codex_auth::save_codex_tokens(&tokens);
                let acct = laruche_essaim::codex_auth::account_id_from_token(&tokens.access_token);
                ensure_codex_profile(&state_bg).await;
                codex_set_status(|s| {
                    s.phase = "connected".into();
                    s.account_id = acct;
                    s.message = "Connected!".into();
                });
            }
            Err(e) => {
                codex_set_status(|s| {
                    s.phase = "error".into();
                    s.message = format!("{e}");
                });
            }
        }
    });

    // Briefly wait for the 1st request to return the code to display.
    match tokio::time::timeout(std::time::Duration::from_secs(25), rx).await {
        Ok(Ok((url, code))) => Ok(Json(serde_json::json!({
            "phase": "pending",
            "verification_url": url,
            "user_code": code,
        }))),
        _ => {
            let live = codex_login_cell()
                .lock()
                .map(|s| s.clone())
                .unwrap_or_default();
            let msg = if live.message.is_empty() {
                "Could not obtain the code, please retry.".to_string()
            } else {
                live.message
            };
            Ok(Json(serde_json::json!({
                "phase": if live.phase == "error" { "error" } else { "pending" },
                "message": msg,
            })))
        }
    }
}

/// POST /api/auth/codex/logout: deletes the stored Codex tokens.
pub(crate) async fn api_codex_logout(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let _ = laruche_essaim::codex_auth::clear_codex_tokens();
    codex_set_status(|s| *s = CodexLoginStatus::default());
    Ok(Json(serde_json::json!({"phase": "idle"})))
}

/// GET /api/profiles/models: unified model list across all profiles.
pub(crate) async fn api_get_unified_models(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // Refresh Ollama models before returning
    let mut cfg = state.profiles.write().await;
    profiles::refresh_ollama_profiles(&mut cfg).await;
    let _ = profiles::save_profiles(&state.profiles_path, &cfg);
    let models = profiles::build_unified_models(&cfg);
    let active = cfg.active_model.clone();
    // Probe n_ctx → engine: aligns context_max_tokens to the REAL n_ctx of the active model.
    // Without this, the default (128000) stays for a local 32768 model → the compact path
    // (index ~4K + dynamic selection, active if ≤ 40000) never triggers → "request exceeds
    // context size" overflow. Here the probed value propagates automatically.
    let (.., mcl) = profiles::active_to_essaim_fields(&cfg);
    drop(cfg);
    {
        let mut ec = state.essaim_config.write().await;
        if ec.context_max_tokens != mcl {
            ec.context_max_tokens = mcl;
        }
    }
    save_persistent_state(&state).await;

    // NOTE: voice services (tts/stt) are deliberately NOT added here. This endpoint feeds
    // the LLM model selector; voice nodes are surfaced separately via /swarm/models (the
    // dashboard mesh panel + the dedicated TTS selector in the status bar).
    Json(serde_json::json!({
        "models": models,
        "active": active,
    }))
}

/// Probe a local voice service `/health`, returning its backend name (e.g. "kokoro")
/// when it responds. Short timeout so the models endpoint stays responsive.
pub(crate) async fn probe_voice_backend(port: u16) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(700))
        .build()
        .ok()?;
    let resp = client
        .get(format!("http://127.0.0.1:{port}/health"))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: serde_json::Value = resp.json().await.ok()?;
    Some(
        v.get("backend")
            .and_then(|x| x.as_str())
            .unwrap_or("local")
            .to_string(),
    )
}

/// POST /api/profiles/active: set the active model.
pub(crate) async fn api_set_active_model(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if !auth_user::require_admin(&state, &headers).await {
        return Json(serde_json::json!({"error": "unauthorized (admin required)"}));
    }
    let profile_id = match body["profile_id"].as_str() {
        Some(id) => id.to_string(),
        None => return Json(serde_json::json!({"error": "missing profile_id"})),
    };
    let model = match body["model"].as_str() {
        Some(m) => m.to_string(),
        None => return Json(serde_json::json!({"error": "missing model"})),
    };

    let mut cfg = state.profiles.write().await;
    if !cfg.profiles.contains_key(&profile_id) {
        return Json(serde_json::json!({"error": "profile not found"}));
    }
    cfg.active_model = profiles::ActiveModel {
        profile_id: profile_id.clone(),
        model: model.clone(),
    };
    let _ = profiles::save_profiles(&state.profiles_path, &cfg);
    drop(cfg);

    // Sync to essaim config
    sync_essaim_from_profiles(&state).await;

    Json(serde_json::json!({"status": "ok", "profile_id": profile_id, "model": model}))
}

/// POST /api/profiles/:id/visibility: toggles the mesh visibility of a provider.
pub(crate) async fn api_set_visibility(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if !auth_user::require_admin(&state, &headers).await {
        return Json(serde_json::json!({"error": "unauthorized (admin required)"}));
    }
    let vis = match body["visibility"].as_str() {
        Some("public_proxy") => profiles::Visibilite::PublicProxy,
        Some("restricted") => profiles::Visibilite::Restricted,
        Some("prive") => profiles::Visibilite::Prive,
        _ => {
            return Json(serde_json::json!(
                {"error": "visibility must be 'prive' | 'public_proxy' | 'restricted'"}
            ))
        }
    };
    let allowed: Vec<String> = body["allowed_peers"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let mut cfg = state.profiles.write().await;
    match cfg.profiles.get_mut(&id) {
        Some(p) => {
            p.visibilite = vis;
            if vis == profiles::Visibilite::Restricted {
                p.allowed_peers = allowed;
            }
        }
        None => return Json(serde_json::json!({"error": "profile not found"})),
    }
    let _ = profiles::save_profiles(&state.profiles_path, &cfg);
    Json(serde_json::json!({"status": "ok", "id": id, "visibility": body["visibility"]}))
}

/// POST /api/models/use: 2-click selection of a model (local or mesh) for its capability.
pub(crate) async fn api_models_use(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if !auth_user::require_admin(&state, &headers).await {
        return Json(serde_json::json!({"error": "unauthorized (admin required)"}));
    }
    let name = body["name"].as_str().unwrap_or_default().to_string();
    if name.is_empty() {
        return Json(serde_json::json!({"error": "missing model name"}));
    }
    let host = body["host"].as_str().unwrap_or_default().to_string();
    let capability = body["capability"].as_str().unwrap_or("llm").to_lowercase();
    let node_id = body["node_id"].as_str().filter(|s| !s.is_empty());
    let base_url_in = body["base_url"].as_str().map(|s| s.to_string());

    let (provider, base_url, profile_id, disp) = if let Some(nid) = node_id {
        let burl = base_url_in.clone().unwrap_or_else(|| host.clone());
        (
            "miel".to_string(),
            burl,
            format!("miel-{nid}"),
            format!("{host} (mesh)"),
        )
    } else if host == "ollama" {
        (
            "ollama".to_string(),
            state.config.ollama_url.clone(),
            "ollama-local".to_string(),
            "Ollama Local".to_string(),
        )
    } else {
        let burl = base_url_in.clone().unwrap_or_else(|| {
            local_inference::backends_openai_compat_par_defaut()
                .into_iter()
                .find(|b| b.label == host)
                .map(|b| b.base_url)
                .unwrap_or_default()
        });
        (
            "openai".to_string(),
            burl,
            format!("local-{host}"),
            format!("{host} (local)"),
        )
    };

    // Dedup: if an existing profile already serves this model, REUSE it (avoids the
    // "local-llama.cpp" vs "llamacpp-8001" duplicates, or a bogus "local-codex").
    let existing_id = {
        let cfg = state.profiles.read().await;
        cfg.profiles
            .iter()
            .find(|(_, p)| p.models.iter().any(|m| m == &name))
            .map(|(id, _)| id.clone())
    };
    let profile_id = existing_id.unwrap_or(profile_id);

    {
        let mut cfg = state.profiles.write().await;
        // Create the profile ONLY if it doesn't exist (otherwise we overwrite neither its
        // provider, nor its base_url, nor its key: we just add the model).
        let prof =
            cfg.profiles
                .entry(profile_id.clone())
                .or_insert_with(|| profiles::ProviderProfile {
                    provider: provider.clone(),
                    name: disp.clone(),
                    base_url: base_url.clone(),
                    api_key: String::new(),
                    models: vec![],
                    visibilite: profiles::Visibilite::Prive, allowed_peers: Vec::new(),
                    max_context_length: 128000,
                });
        if !prof.models.contains(&name) {
            prof.models.push(name.clone());
        }
        // Only change the active chat LLM for "llm"/"agent".
        if capability == "llm" || capability == "agent" {
            cfg.active_model = profiles::ActiveModel {
                profile_id: profile_id.clone(),
                model: name.clone(),
            };
        }
        let _ = profiles::save_profiles(&state.profiles_path, &cfg);
    }
    state
        .default_models
        .write()
        .await
        .insert(capability.clone(), name.clone());
    state.capability_selection.write().await.insert(
        capability.clone(),
        CapabilitySelection {
            capability: capability.clone(),
            model: name.clone(),
            backend: host.clone(),
            node_id: node_id.map(|s| s.to_string()),
            is_local: node_id.is_none(),
            profile_id: profile_id.clone(),
        },
    );

    sync_essaim_from_profiles(&state).await;
    save_persistent_state(&state).await;
    Json(
        serde_json::json!({"status": "ok", "profile_id": profile_id, "model": name, "capability": capability}),
    )
}

/// GET /api/capabilities/selection: current service selection per capability.
pub(crate) async fn api_capabilities_selection(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let sel = state.capability_selection.read().await;
    Json(serde_json::json!({ "selection": serde_json::to_value(&*sel).unwrap_or_default() }))
}

/// `(profile_id, model)` chosen for a capability (e.g. "code"), if any.
pub(crate) async fn capability_profile(state: &Arc<AppState>, capability: &str) -> Option<(String, String)> {
    let sel = state.capability_selection.read().await;
    sel.get(capability)
        .map(|s| (s.profile_id.clone(), s.model.clone()))
}

/// Applies a **profile**'s provider + key + base_url + model onto `config`.
/// Single resolution reused (capability chat, cron, watcher, kanban).
pub(crate) async fn appliquer_profil(
    state: &Arc<AppState>,
    config: &mut EssaimConfig,
    profile_id: &str,
    model: Option<&str>,
) {
    let profiles = state.profiles.read().await;
    if let Some(p) = profiles.profiles.get(profile_id) {
        config.provider = p.provider.clone();
        config.api_key = p.api_key.clone();
        if p.provider == "ollama" {
            config.ollama_url = p.base_url.clone();
            config.api_base = None;
        } else {
            config.api_base = Some(p.base_url.clone());
        }
        if let Some(m) = model {
            config.model = m.to_string();
        } else if let Some(first) = p.models.first() {
            config.model = first.clone();
        }
    }
}

/// Applies the profile serving `capability` (if there is a selection).
pub(crate) async fn appliquer_capacite(state: &Arc<AppState>, config: &mut EssaimConfig, capability: &str) {
    if let Some((pid, model)) = capability_profile(state, capability).await {
        appliquer_profil(state, config, &pid, Some(&model)).await;
    }
}

/// Sync the active profile into EssaimConfig so brain.rs picks it up.
pub(crate) async fn sync_essaim_from_profiles(state: &Arc<AppState>) {
    let cfg = state.profiles.read().await;
    let (provider, model, api_key, api_base, ollama_url, max_context_length) = profiles::active_to_essaim_fields(&cfg);
    drop(cfg);

    let mut ec = state.essaim_config.write().await;
    ec.provider = provider;
    ec.model = model;
    ec.api_key = api_key;
    ec.api_base = api_base;
    ec.ollama_url = ollama_url;
    ec.context_max_tokens = max_context_length;
}
