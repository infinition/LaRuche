//! HTTP API for LaReine, the swarm supervisor: the activation and settings the UI
//! binds to (mode, max review rounds, escalation threshold, tier toggles, the
//! memory proposals gate, and the judge provider). File-backed in
//! `laruche-reine.json` so it stays out of the AppState boot path; the config is
//! tiny and read rarely. The live review hook (brain.rs) and the proposals-queue
//! endpoints land in a later step.

use crate::*;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const REINE_CONFIG_FILE: &str = "laruche-reine.json";
/// Hard ceiling on revision rounds, mirrored from `cap::reine::PLAFOND_REVUES`.
const PLAFOND_REVUES: u8 = 10;
/// Hard ceiling on how many recent turns the judge may be fed as context.
const PLAFOND_CONTEXTE: u8 = 20;

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
    /// How many recent conversation turns the judge sees for context (0 = none,
    /// capped at [`PLAFOND_CONTEXTE`]). Gives her awareness of prior questions.
    #[serde(default = "defaut_contexte_messages")]
    pub contexte_messages: u8,
}

fn defaut_contexte_messages() -> u8 {
    4
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
            contexte_messages: defaut_contexte_messages(),
        }
    }
}

impl ReineSettings {
    /// Clamp values to safe ranges and reject unknown modes.
    fn assainir(&mut self) {
        // 255 is the unlimited sentinel (mirrors cap::reine::REVUES_ILLIMITEES);
        // every finite value is clamped to the runaway ceiling.
        if self.max_revues != u8::MAX {
            self.max_revues = self.max_revues.min(PLAFOND_REVUES);
        }
        self.seuil_confiance = self.seuil_confiance.min(100);
        self.contexte_messages = self.contexte_messages.min(PLAFOND_CONTEXTE);
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
    if let Some(v) = body.get("contexte_messages").and_then(|x| x.as_u64()) {
        cfg.contexte_messages = v.min(255) as u8;
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

/// Full Tier 1 review for a finished answer, streamed live to `tx`: judge it, and
/// if LaReine asks for a revision (within the round budget) send the worker back to
/// **redo the work** (a fresh agentic run, visible in the chat), then re-judge.
/// Emits a `__reine_end__` sentinel when finished. No-op (just the sentinel) when
/// the Reine is inactive.
pub(crate) async fn revue_complete(
    state: &AppState,
    session_id: uuid::Uuid,
    prompt: &str,
    reponse: &str,
    tx: tokio::sync::broadcast::Sender<laruche_essaim::ChatEvent>,
) {
    let fin = |tx: &tokio::sync::broadcast::Sender<laruche_essaim::ChatEvent>| {
        let _ = tx.send(laruche_essaim::ChatEvent::Status {
            message: "__reine_end__".to_string(),
        });
    };

    let rs = charger_reine_settings();
    if !rs.active_for_responses() || reponse.trim().is_empty() {
        fin(&tx);
        return;
    }

    // Animated "reviewing" marker while she judges (the judge call is silent).
    let _ = tx.send(laruche_essaim::ChatEvent::Status {
        message: "__reine_thinking__".to_string(),
    });

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
    let juge = resoudre_creds(state, &j_pid, &j_model).await;

    // Editable rubric (`system.prompt_reine`), else the charter default.
    let charte = laruche_essaim::brain::charger_doc_systeme(&state.memoire, "system.prompt_reine")
        .await
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| laruche_essaim::reine_live::prompt_reine_defaut().to_string());

    // Working copy of the session + the worker config (active profile) for the rework.
    let mut session = match state.essaim_sessions.read().await.get(&session_id).cloned() {
        Some(s) => s,
        None => {
            fin(&tx);
            return;
        }
    };
    let config = state.essaim_config.read().await.clone();

    tracing::info!(target: "reine", judge = %juge.model, max_revues = rs.max_revues, "review + rework");

    let rev = laruche_essaim::reine_live::revue_et_refaire(
        &juge,
        &charte,
        prompt,
        reponse,
        &rs.mode,
        rs.max_revues,
        rs.seuil_confiance,
        rs.contexte_messages,
        &mut session,
        &state.essaim_registry,
        &config,
        state.memoire.clone(),
        &tx,
    )
    .await;

    // Persist the redone answer into the real session (replace the last assistant
    // message), so the conversation continues from LaReine's approved version.
    if rev.revised {
        let mut sessions = state.essaim_sessions.write().await;
        if let Some(s) = sessions.get_mut(&session_id) {
            if let Some(m) = s
                .messages
                .iter_mut()
                .rev()
                .find(|m| matches!(m, laruche_essaim::session::Message::Assistant(_)))
            {
                *m = laruche_essaim::session::Message::Assistant(rev.final_answer.clone());
            }
            let _ = s.sauvegarder();
        }
    }

    fin(&tx);
}

// ======================== Proposals queue (Tier 2, PR-style backlog) ========================

/// GET /api/reine/proposals - the proposals backlog (pending + recent decisions).
pub(crate) async fn api_list_proposals() -> Json<serde_json::Value> {
    // Age out pending proposals older than 14 days (anti-rot).
    let _ = laruche_essaim::reine_queue::purger_perimes(14 * 86_400);
    let props = laruche_essaim::reine_queue::charger();
    let items: Vec<serde_json::Value> = props
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "type": format!("{:?}", p.type_),
                "target": p.cible,
                "preview": p.raison,
                "provenance": p.provenance,
                "status": format!("{:?}", p.statut),
                "risk": format!("{:?}", p.risque()),
                "created_at": p.cree_a,
            })
        })
        .collect();
    let pending = props
        .iter()
        .filter(|p| p.statut == laruche_essaim::reine_file::Statut::EnAttente)
        .count();
    Json(serde_json::json!({ "proposals": items, "pending": pending }))
}

/// POST /api/reine/proposals/:id/approve - apply a proposal to memory (auth).
pub(crate) async fn api_approve_proposal(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let ok = laruche_essaim::reine_queue::approuver(&state.memoire, &id).await;
    Ok(Json(serde_json::json!({ "status": if ok { "ok" } else { "failed" } })))
}

/// POST /api/reine/proposals/:id/reject - discard a proposal, kept for audit (auth).
pub(crate) async fn api_reject_proposal(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let ok = laruche_essaim::reine_queue::rejeter(&id);
    Ok(Json(serde_json::json!({ "status": if ok { "ok" } else { "failed" } })))
}

/// POST /api/reine/proposals/apply-safe - approve all pending safe proposals (auth).
pub(crate) async fn api_approve_safe(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let n = laruche_essaim::reine_queue::approuver_surs(&state.memoire).await;
    Ok(Json(serde_json::json!({ "status": "ok", "applied": n })))
}
