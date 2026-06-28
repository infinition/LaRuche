//! LaReine's live review (Tier 1, advisory): after the worker produces an answer,
//! the Reine judges it with a single LLM call and surfaces a verdict. Advisory in
//! this first cut: the answer still ships; the verdict (scores plus any corrective
//! instruction) is emitted as a status event. The auto-revise loop and the
//! proposals-queue redirect build on top of this.
//!
//! The review is best-effort: any provider or parse failure is swallowed so it can
//! never block or break a normal turn. It is a strict no-op unless the user has
//! enabled the Reine for responses (off by default).

use crate::brain::{ChatEvent, EssaimConfig};
use crate::providers::provider_chat_stream;
use crate::reine_juge::{construire_prompt, parser_scorecard, DemandeJugement};
use futures_util::StreamExt;
use laruche_butinage::cap::reine::{Action, Reine, Scorecard, Tier};

/// Condensed LaReine charter used as the judging rubric. Mirrors the full charter
/// skill (`skills/lareine-charte/SKILL.md`) so the engine stays self-contained.
const CHARTE_CONDENSEE: &str = "You are LaReine, the supervisor of LaRuche. Judge the draft from the \
outside on four axes: relevance (answers the real request, right scope, no padding), methodology \
(sound reasoning, grounded claims not invention), objective (serves the user's real goal), and brand \
compliance (English code and comments, French brand terms kept, no em dash, professional non-LLM tone, \
user-facing strings translated). Approve readily when the draft is good; a revision that does not \
measurably improve it is worse than shipping the original. When you revise, the instruction must be \
specific and executable, naming what is wrong and what to do.";

/// Default LaReine rubric, re-exported for the memory UI (editable node
/// `system.prompt_reine`, hot-reloaded). Empty override falls back to this.
pub fn prompt_reine_defaut() -> &'static str {
    CHARTE_CONDENSEE
}

/// Pick the judge model: the Reine's chosen model when set (profile id encoded as
/// `profile_id|||model`), else the review model, else the worker model. The
/// provider and credentials stay the worker's in this first cut.
fn modele_juge(config: &EssaimConfig) -> String {
    config
        .reine
        .provider_profile
        .as_deref()
        .and_then(|p| p.split("|||").nth(1))
        .filter(|m| !m.is_empty())
        .map(|m| m.to_string())
        .or_else(|| config.review_model.clone())
        .unwrap_or_else(|| config.model.clone())
}

/// Run one judge call over `reponse`. Returns the parsed scorecard, or None on any
/// provider or parse error (best-effort, never blocks the turn).
pub async fn juger_reponse(
    reponse: &str,
    prompt: &str,
    config: &EssaimConfig,
    charte: &str,
) -> Option<Scorecard> {
    let demande = DemandeJugement {
        tier: Tier::Reponse,
        objectif: "",
        requete: prompt,
        brouillon: reponse,
        charte,
    };
    let invite = construire_prompt(&demande);
    let messages = vec![serde_json::json!({ "role": "user", "content": invite })];

    let mut stream = provider_chat_stream(
        &config.provider,
        &modele_juge(config),
        &messages,
        0.2, // low temperature: judging wants determinism
        1024,
        &config.api_key,
        config.api_base.as_deref(),
        &config.ollama_url,
        None,
    )
    .await
    .ok()?;

    let mut brut = String::new();
    while let Some(chunk) = stream.next().await {
        brut.push_str(&chunk.text);
    }
    parser_scorecard(&brut).ok()
}

/// Format a one-line verdict for the status channel.
fn ligne_verdict(card: &Scorecard, action: &Action) -> String {
    let scores = format!(
        "relevance {} / method {} / objective {} / brand {}",
        card.pertinence, card.methodologie, card.objectif, card.conformite_marque
    );
    match action {
        Action::Expedier(_) => format!("LaReine approved ({scores})"),
        Action::Escalader(raison) => format!("LaReine escalated: {raison} ({scores})"),
        Action::Reviser { instruction, .. } => {
            format!("LaReine suggests a revision: {instruction} ({scores})")
        }
    }
}

/// Advisory Tier 1 review: judge the answer and emit a status verdict. Best-effort;
/// the answer itself is never modified. No-op when the Reine is inactive for responses.
pub async fn revue_reponse_advisory(
    reponse: &str,
    prompt: &str,
    config: &EssaimConfig,
    charte: Option<String>,
    tx: &tokio::sync::broadcast::Sender<ChatEvent>,
) {
    if !config.reine.actif_reponse() || reponse.trim().is_empty() {
        return;
    }
    // Editable rubric from memory (`system.prompt_reine`); fall back to the default.
    let charte = charte
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(CHARTE_CONDENSEE);
    let _ = tx.send(ChatEvent::Status {
        message: "LaReine is reviewing the answer...".to_string(),
    });
    let Some(card) = juger_reponse(reponse, prompt, config, charte).await else {
        return;
    };
    let mut reine = Reine::nouvelle(config.reine.to_core());
    let action = reine.juger(&card);
    let _ = tx.send(ChatEvent::Status {
        message: ligne_verdict(&card, &action),
    });
}
