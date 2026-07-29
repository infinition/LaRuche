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
    /// Record each review as a training sample in `evals/reine-dataset.jsonl`.
    /// OFF by default: it persists the full text of requests and answers, which the
    /// scorecard never did, so it has to be a deliberate choice.
    #[serde(default)]
    pub dataset: bool,
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
            // Capturing full request/answer text is never a default.
            dataset: false,
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
    // Apply the gate immediately (skill_create reads this process-global).
    laruche_essaim::reine_queue::definir_gate(cfg.queue_gate);
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
    revue_complete_avec(state, session_id, prompt, reponse, tx, charger_reine_settings()).await
}

/// Same, with the settings supplied by the caller instead of read from disk.
///
/// The summon button needs this: LaReine ships switched off, and `active_for_responses`
/// would refuse the very review the user just asked for by hand. The manual path passes
/// a forced copy rather than reaching around the gate, so one place still decides what
/// "active" means.
pub(crate) async fn revue_complete_avec(
    state: &AppState,
    session_id: uuid::Uuid,
    prompt: &str,
    reponse: &str,
    tx: tokio::sync::broadcast::Sender<laruche_essaim::ChatEvent>,
    rs: ReineSettings,
) {
    let fin = |tx: &tokio::sync::broadcast::Sender<laruche_essaim::ChatEvent>| {
        let _ = tx.send(laruche_essaim::ChatEvent::Status {
            message: "__reine_end__".to_string(),
        });
    };

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

    // The judging AND any rework it triggers show in the status bar: a rework is a full
    // agentic run that can take minutes, and it used to happen with no sign of life.
    let _garde = GardeTravail::nouveau(
        &state.travaux,
        Travail {
            acteur: "lareine".to_string(),
            sujet: "review".to_string(),
            fournisseur: if juge.provider.is_empty() {
                "ollama".to_string()
            } else {
                juge.provider.clone()
            },
            modele: juge.model.clone(),
            canal: Some("web".to_string()),
            depuis: chrono::Utc::now().to_rfc3339(),
        },
    );

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

    // Persist into the real session: the redone answer (replace the last assistant
    // message) AND the verdict itself as a reine thought, so the judgment survives
    // a reload instead of evaporating with the ephemeral status event.
    {
        let mut sessions = state.essaim_sessions.write().await;
        if let Some(s) = sessions.get_mut(&session_id) {
            if rev.revised {
                if let Some(m) = s
                    .messages
                    .iter_mut()
                    .rev()
                    .find(|m| matches!(m, laruche_essaim::session::Message::Assistant(_)))
                {
                    *m = laruche_essaim::session::Message::Assistant(rev.final_answer.clone());
                }
            }
            if !rev.resume.is_empty() {
                let texte = if rev.analyse.is_empty() {
                    rev.resume.clone()
                } else {
                    format!("{}\n{}", rev.resume, rev.analyse)
                };
                s.ajouter_thought("reine", "verdict", &texte);
            }
            let _ = s.sauvegarder();
        }
    }

    // Into the Feed: her verdicts happened entirely off the record, so a rework could
    // rewrite an answer with nothing anywhere saying who decided it or why.
    if !rev.resume.is_empty() {
        laruche_essaim::feed_journal::record(
            "LaReine",
            "reine",
            if rev.revised {
                "reviewed and had the work redone"
            } else {
                "reviewed and approved"
            },
            preview_text(&rev.resume, 160),
            chrono::Utc::now(),
        );
    }

    fin(&tx);
}

/// LaReine Tier 1 review for a background MISSION iteration. Unlike `revue_complete` (which
/// looks the session up in `essaim_sessions`), this operates on the mission's own ephemeral
/// session and the mission's config (same provider/model/disabled-tools), and RETURNS the
/// final text (reworked if she revised it, otherwise the original) so the caller can deliver
/// the approved version. Runs only when Tier 1 is active; otherwise returns the original.
pub(crate) async fn revue_mission(
    state: &AppState,
    session: &mut laruche_essaim::session::Session,
    config: &laruche_essaim::EssaimConfig,
    prompt: &str,
    reponse: &str,
    tx: &tokio::sync::broadcast::Sender<laruche_essaim::ChatEvent>,
) -> String {
    let rs = charger_reine_settings();
    if !rs.active_for_responses() || reponse.trim().is_empty() {
        return reponse.to_string();
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
    let juge = resoudre_creds(state, &j_pid, &j_model).await;
    let charte = laruche_essaim::brain::charger_doc_systeme(&state.memoire, "system.prompt_reine")
        .await
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| laruche_essaim::reine_live::prompt_reine_defaut().to_string());

    // LaReine judges on her own model, which is often not the one that produced the answer:
    // the indicator must name hers while she is the one working.
    // Registered from the judge's own credentials rather than the run config: they carry
    // provider and model directly, and are the pair actually being billed here.
    let _garde = GardeTravail::nouveau(
        &state.travaux,
        Travail {
            acteur: "lareine".to_string(),
            sujet: "review".to_string(),
            fournisseur: if juge.provider.is_empty() {
                "ollama".to_string()
            } else {
                juge.provider.clone()
            },
            modele: juge.model.clone(),
            canal: config.origin_channel.clone(),
            depuis: chrono::Utc::now().to_rfc3339(),
        },
    );
    let rev = laruche_essaim::reine_live::revue_et_refaire(
        &juge,
        &charte,
        prompt,
        reponse,
        &rs.mode,
        rs.max_revues,
        rs.seuil_confiance,
        rs.contexte_messages,
        session,
        &state.essaim_registry,
        config,
        state.memoire.clone(),
        tx,
    )
    .await;

    if rev.revised && !rev.final_answer.trim().is_empty() {
        rev.final_answer
    } else {
        reponse.to_string()
    }
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
            // Full proposed content, display-ready: memory writes store a
            // serialized MemoryItem (extract its text), skills store the raw
            // OKF body, hygiene proposals carry their message in `raison`.
            let full = match serde_json::from_str::<serde_json::Value>(&p.contenu) {
                Ok(v) => v
                    .get("content")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| p.contenu.clone()),
                Err(_) => p.contenu.clone(),
            };
            serde_json::json!({
                "id": p.id,
                "type": format!("{:?}", p.type_),
                "target": p.cible,
                "preview": p.raison,
                "full": if full.trim().is_empty() { p.raison.clone() } else { full },
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

/// GET /api/reine/dataset?format=sft|dpo|judge — convert the captured reviews into a
/// ready-to-train JSONL, downloaded as a file.
///
/// One capture, three shapes, because they train different things:
///   * `sft`   — `{messages:[user, assistant]}`, the accepted answer only.
///   * `dpo`   — `{prompt, chosen, rejected}`, ONLY from reviews that actually produced a
///     revision. A record where nothing was sent back has no rejected side, and inventing
///     one (pairing an answer against itself) would teach the model noise.
///   * `judge` — `{messages:[user(request+draft), assistant(verdict+critique)]}`, to
///     distil LaReine's judgement into a smaller reviewer.
///
/// Values are already masked at capture time; nothing is unmasked here.
pub(crate) async fn api_reine_dataset(
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Result<axum::response::Response, StatusCode> {
    // The file holds the full text of every reviewed exchange: admin only.
    let users = state.users.read().await;
    let (_, is_admin) = auth_user::check_admin(&headers, &state.cookie_secret, &users);
    drop(users);
    if !is_admin {
        return Err(StatusCode::FORBIDDEN);
    }
    let format = q.get("format").map(String::as_str).unwrap_or("dpo");
    let brut = std::fs::read_to_string("evals/reine-dataset.jsonl").unwrap_or_default();
    let mut out = String::new();
    let mut n = 0usize;
    for v in brut.lines().filter_map(|l| {
        serde_json::from_str::<serde_json::Value>(l).ok()
    }) {
        let prompt = v["prompt"].as_str().unwrap_or("");
        let chosen = v["chosen"].as_str().unwrap_or("");
        let rejected = v["rejected"].as_str().unwrap_or("");
        if prompt.is_empty() || chosen.is_empty() {
            continue;
        }
        let ligne = match format {
            "sft" => serde_json::json!({
                "messages": [
                    {"role": "user", "content": prompt},
                    {"role": "assistant", "content": chosen},
                ]
            }),
            "judge" => {
                let critique = v["critique"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|c| c.as_str())
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                let verdict = v["avis"].as_str().unwrap_or("");
                if critique.trim().is_empty() && v["reasoning"].as_str().unwrap_or("").is_empty() {
                    continue;
                }
                serde_json::json!({
                    "messages": [
                        {"role": "user", "content": format!("Request:\n{prompt}\n\nDraft answer:\n{}", if rejected.is_empty() { chosen } else { rejected })},
                        {"role": "assistant", "content": format!("verdict: {verdict}\nscores: {}\n{}\n{critique}", v["scores"], v["reasoning"].as_str().unwrap_or(""))},
                    ]
                })
            }
            // Default: preference pairs. Skip anything that is not a genuine pair.
            _ => {
                if !v["paire_preference"].as_bool().unwrap_or(false) {
                    continue;
                }
                serde_json::json!({ "prompt": prompt, "chosen": chosen, "rejected": rejected })
            }
        };
        out.push_str(&ligne.to_string());
        out.push('\n');
        n += 1;
    }
    let nom = format!("laruche-{format}-{n}.jsonl");
    axum::response::Response::builder()
        .header("Content-Type", "application/x-ndjson")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{nom}\""),
        )
        .body(axum::body::Body::from(out))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// GET /api/reine/scorecards - aggregate view of the review journal
/// (`evals/reine-scorecards.jsonl`): totals per verdict, average scores, and the
/// most recent entries. This is the Scorecard dashboard's data.
pub(crate) async fn api_reine_scorecards() -> Json<serde_json::Value> {
    let brut = std::fs::read_to_string("evals/reine-scorecards.jsonl").unwrap_or_default();
    let lignes: Vec<serde_json::Value> = brut
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let total = lignes.len();
    let compte = |avis: &str| lignes.iter().filter(|v| v["avis"] == avis).count();
    let moyenne = |champ: &str| -> f64 {
        if lignes.is_empty() {
            return 0.0;
        }
        let somme: f64 = lignes
            .iter()
            .filter_map(|v| v[champ].as_f64())
            .sum();
        (somme / lignes.len() as f64 * 10.0).round() / 10.0
    };
    let revises = lignes.iter().filter(|v| v["revised"] == true).count();
    let recentes: Vec<serde_json::Value> = lignes.iter().rev().take(15).cloned().collect();
    Json(serde_json::json!({
        "total": total,
        "approve": compte("approve"),
        "revise": compte("revise"),
        "escalate": compte("escalate"),
        "revised": revises,
        "avg": {
            "relevance": moyenne("relevance"),
            "methodology": moyenne("methodology"),
            "objective": moyenne("objective"),
            "brand": moyenne("brand"),
            "confidence": moyenne("confidence"),
            "rounds": moyenne("rounds"),
        },
        "recent": recentes,
    }))
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

/// Widened window for a hand-made call. The automatic review runs on every turn and
/// four turns is enough to grade the last answer; someone pressing the button wants to
/// know where a LONG conversation went wrong, and that needs more history. This is a
/// deliberate one-off, so the extra tokens cost nothing that matters.
const FENETRE_APPEL_MANUEL: usize = 12;

/// POST /api/reine/appel - summon LaReine on the conversation as it stands.
///
/// Body: `{"session_id": "<uuid>"}`. Returns her verdict, scores and reasoning.
///
/// JUDGE ONLY: it never rewrites the message the user is looking at. It works with
/// LaReine switched off, which is the point: the mode ships off, so without this the
/// most distinctive part of the project is something most users never see once.
pub(crate) async fn api_reine_appel(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    use axum::http::StatusCode;
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let session_id = body
        .get("session_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let session = {
        let sessions = state.essaim_sessions.read().await;
        let s = sessions.get(&session_id).ok_or(StatusCode::NOT_FOUND)?;
        if s.user_id.is_some() && s.user_id != caller {
            return Err(StatusCode::FORBIDDEN);
        }
        s.clone()
    };

    let rs = charger_reine_settings();
    // Her own judge profile when one is set, otherwise the active model. Same
    // resolution as the automatic path: a manual call must not silently judge with a
    // different brain than the one configured.
    let (pid, model) = match rs.provider_profile.as_deref().filter(|s| !s.is_empty()) {
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
    let juge = resoudre_creds(&state, &pid, &model).await;

    let charte = laruche_essaim::brain::charger_doc_systeme(&state.memoire, "system.prompt_reine")
        .await
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| laruche_essaim::reine_live::prompt_reine_defaut().to_string());

    tracing::info!(target: "reine", judge = %juge.model, "summoned by hand");

    match laruche_essaim::reine_live::juger_a_la_demande(
        &juge,
        &charte,
        &session,
        &state.essaim_registry,
        state.memoire.clone(),
        FENETRE_APPEL_MANUEL,
    )
    .await
    {
        Ok(card) => {
            let mut out = laruche_essaim::reine_live::verdict_json(&card);
            if let Some(o) = out.as_object_mut() {
                o.insert("ok".into(), serde_json::json!(true));
            }
            Ok(Json(out))
        }
        // The three causes need different fixes, so the message names the one that hit
        // instead of listing them all and leaving the user to guess.
        Err(raison) => {
            tracing::warn!(target: "reine", judge = %juge.model, reason = %raison, "manual call: no verdict");
            Ok(Json(serde_json::json!({
                "ok": false,
                "error": format!("LaReine could not deliver a verdict: {raison}")
            })))
        }
    }
}

/// POST /api/reine/renvoyer - summon LaReine AND let her send the work back.
///
/// Body: `{"session_id": "<uuid>"}`. Returns as soon as the review starts; the verdict
/// and, if she asks for one, the fresh agentic run both stream into the chat over the
/// session's existing channel, exactly as they would with her on duty.
///
/// `/api/reine/appel` stays the read-only sibling: a verdict and nothing else. This one
/// is the "back to work" button, and it is separate precisely because redoing the work
/// spends tokens and replaces the answer on screen: that has to be an explicit click,
/// never a side effect of asking for an opinion.
pub(crate) async fn api_reine_renvoyer(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let session_id = body
        .get("session_id")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or(StatusCode::BAD_REQUEST)?;

    // The session's own event channel: without it the rework would run invisibly and
    // land as a message nobody saw being produced.
    let (prompt, reponse, tx) = {
        let sessions = state.essaim_sessions.read().await;
        let s = sessions.get(&session_id).ok_or(StatusCode::NOT_FOUND)?;
        if s.user_id.is_some() && s.user_id != caller {
            return Err(StatusCode::FORBIDDEN);
        }
        let reponse = s
            .messages
            .iter()
            .rev()
            .find_map(|m| match m {
                laruche_essaim::Message::Assistant(t) if !t.trim().is_empty() => Some(t.clone()),
                _ => None,
            })
            .ok_or(StatusCode::BAD_REQUEST)?;
        let prompt = s
            .messages
            .iter()
            .rev()
            .find_map(|m| match m {
                laruche_essaim::Message::User(t) => Some(t.clone()),
                laruche_essaim::Message::UserMultimodal { text, .. } => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let tx = s.event_tx.clone().ok_or(StatusCode::CONFLICT)?;
        (prompt, reponse, tx)
    };

    // Forced settings. She ships off, and `active_for_responses` would otherwise refuse
    // the review the user just asked for. Tier 1 on, at least one round so a revision
    // can actually happen, and the wider window because a hand-made call is about a
    // whole conversation rather than the last turn. Everything else, including her
    // judge profile, stays as configured: a manual call must not judge with a different
    // brain than the automatic one.
    let mut rs = charger_reine_settings();
    if rs.mode == "off" {
        rs.mode = "auto".into();
    }
    rs.tier_reponse = true;
    rs.max_revues = rs.max_revues.max(1);
    rs.contexte_messages = rs.contexte_messages.max(FENETRE_APPEL_MANUEL as u8);
    rs.assainir();

    tracing::info!(target: "reine", max_revues = rs.max_revues, "summoned by hand, rework allowed");

    // Detached: the rework is a full agentic run and can take minutes. Holding the HTTP
    // request open for it would time out in the browser while the work still streams.
    let state2 = state.clone();
    tokio::spawn(async move {
        revue_complete_avec(&state2, session_id, &prompt, &reponse, tx, rs).await;
    });

    Ok(Json(serde_json::json!({ "ok": true, "started": true })))
}
