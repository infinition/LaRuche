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
use laruche_butinage::cap::reine::{
    Action, Avis, ConfigReine, ModeReine, Reine, Scorecard, Tier,
};

/// Resolved provider credentials for one LLM call (judge or worker).
#[derive(Debug, Clone)]
pub struct ProviderCreds {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub api_base: Option<String>,
    pub ollama_url: String,
}

/// Outcome of the review-revise loop.
#[derive(Debug, Clone)]
pub struct Revision {
    /// The final answer (possibly rewritten one or more times).
    pub final_answer: String,
    /// One verdict line per round, for the chat trace.
    pub journal: Vec<String>,
    /// Did the answer actually change?
    pub revised: bool,
    /// Number of revision rounds applied.
    pub rounds: u8,
}

/// The full LaReine charter, embedded at compile time so the engine is
/// self-contained. This is the judging rubric.
const CHARTE_SKILL: &str = include_str!("../../skills/lareine-charte/SKILL.md");

/// Default LaReine rubric (the charter body, frontmatter stripped), re-exported
/// for the memory UI (editable node `system.prompt_reine`, hot-reloaded). The
/// editable node, when non-empty, overrides this.
pub fn prompt_reine_defaut() -> &'static str {
    static BODY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    BODY.get_or_init(|| {
        // Strip the leading YAML frontmatter (--- ... ---) if present.
        if let Some(rest) = CHARTE_SKILL.strip_prefix("---") {
            if let Some(idx) = rest.find("\n---") {
                return rest[idx + 4..].trim_start().to_string();
            }
        }
        CHARTE_SKILL.to_string()
    })
    .as_str()
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

    let mut stream = match provider_chat_stream(
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
    {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(target: "reine", provider = %provider, model = %model, error = %e, "judge: provider call failed");
            return None;
        }
    };

    let mut brut = String::new();
    while let Some(chunk) = stream.next().await {
        brut.push_str(&chunk.text);
    }
    let apercu: String = brut.chars().take(220).collect();
    tracing::info!(target: "reine", model = %model, len = brut.len(), preview = %apercu, "judge: raw output");
    match parser_scorecard(&brut) {
        Ok(c) => Some(c),
        Err(e) => {
            tracing::warn!(target: "reine", error = %e, "judge: output not parseable as scorecard");
            None
        }
    }
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
    match juger_avec(
        provider, model, api_key, api_base, ollama_url, reponse, prompt, charte,
    )
    .await
    {
        Some(card) => Some(ligne_verdict(&card)),
        // Visible fallback (instead of vanishing) so it is clear the review ran but
        // the judge did not return a usable verdict. See logs (target "reine").
        None => Some("LaReine could not produce a verdict (judge output unusable)".to_string()),
    }
}

/// Ask the worker model to rewrite its answer per LaReine's instruction. One call,
/// no tools: a reformulation, not a fresh agentic run. Returns the revised text.
async fn regenerer(
    worker: &ProviderCreds,
    prompt: &str,
    answer: &str,
    instruction: &str,
) -> Option<String> {
    let invite = format!(
        "You wrote the answer below for the user. Your supervisor LaReine asks you to revise it.\n\n\
         User request:\n{prompt}\n\nYour answer:\n{answer}\n\nRevision instruction:\n{instruction}\n\n\
         Apply the instruction in good faith. If part of it is clearly wrong or would make the answer \
         worse, keep what was already correct rather than degrading it. Reply with ONLY the revised \
         answer, in the user's language, with no preamble and no mention of this revision."
    );
    let messages = vec![serde_json::json!({ "role": "user", "content": invite })];
    let mut stream = provider_chat_stream(
        &worker.provider,
        &worker.model,
        &messages,
        0.7,
        2048,
        &worker.api_key,
        worker.api_base.as_deref(),
        &worker.ollama_url,
        None,
    )
    .await
    .ok()?;
    let mut out = String::new();
    while let Some(chunk) = stream.next().await {
        out.push_str(&chunk.text);
    }
    (!out.trim().is_empty()).then_some(out)
}

/// The full Tier 1 review-revise loop: judge the answer, and while LaReine asks
/// for a revision and the round budget allows, have the worker rewrite it, then
/// judge again. Stops when she approves, escalates, or the budget is reached. The
/// loop is bounded by [`ConfigReine::revues_effectives`], so it cannot run away.
#[allow(clippy::too_many_arguments)]
pub async fn revue_et_revise(
    juge: &ProviderCreds,
    worker: &ProviderCreds,
    charte: &str,
    prompt: &str,
    answer_initial: &str,
    mode: &str,
    max_revues: u8,
    seuil: u8,
) -> Revision {
    let cfg = ConfigReine {
        mode: ModeReine::depuis_str(mode),
        max_revues,
        seuil_confiance: if seuil == 0 { 60 } else { seuil },
        tier_reponse: true,
        tier_artefacts: false,
        tier_supervision: false,
    };
    let mut reine = Reine::nouvelle(cfg);
    let mut answer = answer_initial.to_string();
    let mut journal: Vec<String> = Vec::new();
    let mut revised = false;
    let mut rounds = 0u8;

    loop {
        let card = match juger_avec(
            &juge.provider,
            &juge.model,
            &juge.api_key,
            juge.api_base.as_deref(),
            &juge.ollama_url,
            &answer,
            prompt,
            charte,
        )
        .await
        {
            Some(c) => c,
            None => {
                journal.push("LaReine could not produce a verdict (judge output unusable)".into());
                break;
            }
        };

        match reine.juger(&card) {
            Action::Reviser { tour, instruction } => {
                journal.push(format!("LaReine round {tour}: {}", instruction.trim()));
                match regenerer(worker, prompt, &answer, &instruction).await {
                    Some(new_answer) => {
                        answer = new_answer;
                        revised = true;
                        rounds = tour;
                    }
                    None => {
                        journal.push("LaReine: the revision could not be generated".into());
                        break;
                    }
                }
            }
            // Approved or budget reached or escalated: keep the current answer.
            _ => {
                journal.push(ligne_verdict(&card));
                break;
            }
        }
    }

    Revision {
        final_answer: answer,
        journal,
        revised,
        rounds,
    }
}
