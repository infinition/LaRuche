//! HTTP API for LaReine, the swarm supervisor: the activation and settings the UI
//! binds to (mode, max review rounds, escalation threshold, tier toggles, the
//! memory proposals gate, and the judge provider). File-backed in
//! `laruche-reine.json` so it stays out of the AppState boot path; the config is
//! tiny and read rarely. The live review hook (brain.rs) and the proposals-queue
//! endpoints land in a later step.

use crate::*;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const REINE_CONFIG_FILE: &str = "laruche-reine.json";
/// Hard ceiling on revision rounds, mirrored from `cap::reine::PLAFOND_REVUES`.
const PLAFOND_REVUES: u8 = 10;

/// LaReine settings, persisted to `laruche-reine.json` and bound by the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ReineSettings {
    /// off | auto | hybride | humaine.
    pub mode: String,
    /// Max revision rounds: 0 = off, capped at [`PLAFOND_REVUES`]. She stops as
    /// soon as a draft passes, so the ceiling is only a runaway guard.
    pub max_revues: u8,
    /// Escalation confidence threshold (0..=100), used in Hybride mode.
    pub seuil_confiance: u8,
    /// Tier 1: review chat answers before they reach the user.
    pub tier_reponse: bool,
    /// Tier 2: review self-created artifacts (skills, tools, memory edits).
    pub tier_artefacts: bool,
    /// Tier 3: proactive orchestration (the optional supervisor loop).
    pub tier_supervision: bool,
    /// Gate agent memory writes into the proposals queue (the PR backlog).
    pub queue_gate: bool,
    /// Provider profile id used by the judge (None = same as the worker model).
    pub provider_profile: Option<String>,
}

impl Default for ReineSettings {
    fn default() -> Self {
        Self {
            mode: "off".into(),
            max_revues: 0,
            seuil_confiance: 60,
            tier_reponse: true,
            tier_artefacts: false,
            tier_supervision: false,
            queue_gate: false,
            provider_profile: None,
        }
    }
}

impl ReineSettings {
    /// Clamp values to safe ranges and reject unknown modes.
    fn assainir(&mut self) {
        self.max_revues = self.max_revues.min(PLAFOND_REVUES);
        self.seuil_confiance = self.seuil_confiance.min(100);
        if !matches!(self.mode.as_str(), "off" | "auto" | "hybride" | "humaine") {
            self.mode = "off".into();
        }
    }
}

/// Load settings from disk, falling back to defaults when absent or invalid.
pub(crate) fn charger_reine_settings() -> ReineSettings {
    std::fs::read_to_string(REINE_CONFIG_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn sauver_reine_settings(s: &ReineSettings) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(s).unwrap_or_default();
    std::fs::write(REINE_CONFIG_FILE, json)
}

/// GET /api/config/reine - current LaReine settings.
pub(crate) async fn api_get_reine_config() -> Json<ReineSettings> {
    Json(charger_reine_settings())
}

/// POST /api/config/reine - update LaReine settings (auth, clamped, persisted).
///
/// Merges the provided fields into the current settings, so callers can send a
/// full object (the Settings panel) or a single field (the chat crown toggle
/// sending just `{ "mode": "auto" }`).
pub(crate) async fn api_set_reine_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let mut cfg = charger_reine_settings();
    if let Some(v) = body.get("mode").and_then(|x| x.as_str()) {
        cfg.mode = v.to_string();
    }
    if let Some(v) = body.get("max_revues").and_then(|x| x.as_u64()) {
        cfg.max_revues = v.min(255) as u8;
    }
    if let Some(v) = body.get("seuil_confiance").and_then(|x| x.as_u64()) {
        cfg.seuil_confiance = v.min(255) as u8;
    }
    if let Some(v) = body.get("tier_reponse").and_then(|x| x.as_bool()) {
        cfg.tier_reponse = v;
    }
    if let Some(v) = body.get("tier_artefacts").and_then(|x| x.as_bool()) {
        cfg.tier_artefacts = v;
    }
    if let Some(v) = body.get("tier_supervision").and_then(|x| x.as_bool()) {
        cfg.tier_supervision = v;
    }
    if let Some(v) = body.get("queue_gate").and_then(|x| x.as_bool()) {
        cfg.queue_gate = v;
    }
    if body.get("provider_profile").is_some() {
        cfg.provider_profile = body
            .get("provider_profile")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
    }
    cfg.assainir();
    sauver_reine_settings(&cfg).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

impl ReineSettings {
    /// Is Tier 1 review active for chat responses?
    fn active_for_responses(&self) -> bool {
        self.mode != "off" && self.max_revues > 0 && self.tier_reponse
    }
}

/// Cheap check (reads the settings file) used to show the "reviewing" animation
/// before the blocking judge call.
pub(crate) fn review_active() -> bool {
    charger_reine_settings().active_for_responses()
}

/// Resolve provider credentials for a profile id + model, overlaying the resolved
/// profile onto the worker config (used as a fallback when the profile is unknown).
async fn resoudre_creds(
    state: &AppState,
    pid: &str,
    model: &str,
) -> laruche_essaim::reine_live::ProviderCreds {
    let (mut provider, mut api_key, mut api_base, mut ollama_url, fallback_model) = {
        let ec = state.essaim_config.read().await;
        (
            ec.provider.clone(),
            ec.api_key.clone(),
            ec.api_base.clone(),
            ec.ollama_url.clone(),
            ec.model.clone(),
        )
    };
    {
        let profiles = state.profiles.read().await;
        if let Some(p) = profiles.profiles.get(pid) {
            provider = p.provider.clone();
            api_key = p.api_key.clone();
            if p.provider == "ollama" {
                ollama_url = p.base_url.clone();
                api_base = None;
            } else {
                api_base = Some(p.base_url.clone());
            }
        }
    }
    laruche_essaim::reine_live::ProviderCreds {
        provider,
        model: if model.trim().is_empty() {
            fallback_model
        } else {
            model.to_string()
        },
        api_key,
        api_base,
        ollama_url,
    }
}

/// Full Tier 1 review for a finished answer: judge it, and if LaReine asks for a
/// revision (within the round budget) have the worker rewrite it, then re-judge.
/// Returns a verdict summary line and, when the answer was rewritten, the final
/// revised text. None when the Reine is inactive.
pub(crate) async fn revue_complete(
    state: &AppState,
    prompt: &str,
    reponse: &str,
) -> Option<(String, Option<String>)> {
    let rs = charger_reine_settings();
    if !rs.active_for_responses() || reponse.trim().is_empty() {
        return None;
    }

    // Judge profile: LaReine's own pick (`profile_id|||model`), else the active model.
    let (j_pid, j_model) = match rs.provider_profile.as_deref().filter(|s| !s.is_empty()) {
        Some(pp) => {
            let mut p = pp.split("|||");
            (
                p.next().unwrap_or("").to_string(),
                p.next().unwrap_or("").to_string(),
            )
        }
        None => {
            let pr = state.profiles.read().await;
            (
                pr.active_model.profile_id.clone(),
                pr.active_model.model.clone(),
            )
        }
    };
    // Worker (for the rewrite): the active model.
    let (w_pid, w_model) = {
        let pr = state.profiles.read().await;
        (
            pr.active_model.profile_id.clone(),
            pr.active_model.model.clone(),
        )
    };

    let juge = resoudre_creds(state, &j_pid, &j_model).await;
    let worker = resoudre_creds(state, &w_pid, &w_model).await;

    // Editable rubric (`system.prompt_reine`), else the charter default.
    let charte = laruche_essaim::brain::charger_doc_systeme(&state.memoire, "system.prompt_reine")
        .await
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| laruche_essaim::reine_live::prompt_reine_defaut().to_string());

    tracing::info!(
        target: "reine",
        judge = %juge.model,
        worker = %worker.model,
        max_revues = rs.max_revues,
        "review-revise running"
    );

    let rev = laruche_essaim::reine_live::revue_et_revise(
        &juge,
        &worker,
        &charte,
        prompt,
        reponse,
        &rs.mode,
        rs.max_revues,
        rs.seuil_confiance,
    )
    .await;

    let summary = if rev.revised {
        format!("LaReine revised the answer ({} round(s))", rev.rounds)
    } else {
        rev.journal
            .last()
            .cloned()
            .unwrap_or_else(|| "LaReine reviewed the answer".to_string())
    };
    let revised = rev.revised.then_some(rev.final_answer);
    Some((summary, revised))
}
