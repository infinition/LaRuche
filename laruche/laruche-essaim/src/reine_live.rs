//! LaReine's live review (Tier 1, advisory): judge an answer with a single LLM
//! call and return a verdict line. The node drives this on `Done` (it can resolve
//! LaReine's own provider profile and emit the verdict before the turn closes), so
//! this module stays parameter-explicit and free of `EssaimConfig` / channel types.
//!
//! Best-effort: any provider or parse failure returns None so a review can never
//! block or break a turn.

use crate::providers::provider_chat_stream;
use crate::reine_juge::{construire_prompt, parser_scorecard, DemandeJugement};
use futures_util::StreamExt;
use laruche_butinage::cap::reine::{Avis, Scorecard, Tier};

/// Condensed LaReine rubric. Mirrors the full charter skill
/// (`skills/lareine-charte/SKILL.md`) and is the fallback when the editable
/// `system.prompt_reine` node is empty.
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

/// Run one judge call over `reponse`. Returns the parsed scorecard, or None on any
/// provider or parse error. Provider/model/credentials are explicit so the caller
/// (the node) can point the judge at LaReine's own provider.
#[allow(clippy::too_many_arguments)]
pub async fn juger_avec(
    provider: &str,
    model: &str,
    api_key: &str,
    api_base: Option<&str>,
    ollama_url: &str,
    reponse: &str,
    prompt: &str,
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
        provider,
        model,
        &messages,
        0.2, // low temperature: judging wants determinism
        1024,
        api_key,
        api_base,
        ollama_url,
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

/// Format a one-line verdict from a scorecard (advisory: the judge's own avis).
fn ligne_verdict(card: &Scorecard) -> String {
    let scores = format!(
        "relevance {} / method {} / objective {} / brand {}",
        card.pertinence, card.methodologie, card.objectif, card.conformite_marque
    );
    match card.avis {
        Avis::Approuver => format!("LaReine approved ({scores})"),
        Avis::Escalader => format!(
            "LaReine flagged this for you: {} ({scores})",
            card.raison.trim()
        ),
        Avis::Reviser => format!(
            "LaReine suggests a revision: {} ({scores})",
            card.instruction.trim()
        ),
    }
}

/// Judge the answer and return a ready-to-display verdict line, or None if the
/// review could not run. Convenience over [`juger_avec`] for the node.
#[allow(clippy::too_many_arguments)]
pub async fn juger_et_formater(
    provider: &str,
    model: &str,
    api_key: &str,
    api_base: Option<&str>,
    ollama_url: &str,
    reponse: &str,
    prompt: &str,
    charte: &str,
) -> Option<String> {
    let card = juger_avec(
        provider, model, api_key, api_base, ollama_url, reponse, prompt, charte,
    )
    .await?;
    Some(ligne_verdict(&card))
}
