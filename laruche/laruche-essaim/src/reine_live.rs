//! LaReine's live review (Tier 1, advisory): judge an answer with a single LLM
//! call and return a verdict line. The node drives this on `Done` (it can resolve
//! LaReine's own provider profile and emit the verdict before the turn closes), so
//! this module stays parameter-explicit and free of `EssaimConfig` / channel types.
//!
//! Best-effort: any provider or parse failure returns None so a review can never
//! block or break a turn.

use crate::brain::{boucle_react_memoire_multimodal, ChatEvent, EssaimConfig};
use crate::providers::provider_chat_stream;
use crate::reine_juge::{construire_prompt, parser_scorecard, DemandeJugement};
use crate::session::Session;
use crate::AbeilleRegistry;
use futures_util::StreamExt;
use laruche_butinage::cap::reine::{
    Action, Avis, ConfigReine, ModeReine, Reine, Scorecard, Tier,
};
use laruche_memoire::MemoireCognitive;
use std::sync::Arc;

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
    /// The judge's reasoning from the final assessment (shown on demand).
    pub analyse: String,
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

/// The full Tier 1 loop with REAL rework: judge the answer, and while LaReine asks
/// for a revision and the round budget allows, send the worker back to **redo the
/// work** (a fresh agentic run with its tools, not a text reformulation), then judge
/// again. Stops when she approves, escalates, or the budget is reached. Bounded by
/// the round budget so it cannot run away.
///
/// The rework streams to a throwaway channel, so its internal steps stay silent;
/// only the final answer surfaces. `session` is a working copy (the node persists
/// the final answer to the real session afterwards).
#[allow(clippy::too_many_arguments)]
pub async fn revue_et_refaire(
    juge: &ProviderCreds,
    charte: &str,
    user_prompt: &str,
    answer_initial: &str,
    mode: &str,
    max_revues: u8,
    seuil: u8,
    session: &mut Session,
    registry: &AbeilleRegistry,
    config: &EssaimConfig,
    memoire: Arc<dyn MemoireCognitive>,
    tx: &tokio::sync::broadcast::Sender<ChatEvent>,
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
    let mut analyse = String::new();
    let mut carte_finale: Option<Scorecard> = None;

    loop {
        let card = match juger_avec(
            &juge.provider,
            &juge.model,
            &juge.api_key,
            juge.api_base.as_deref(),
            &juge.ollama_url,
            &answer,
            user_prompt,
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
        analyse = card.analyse.clone();

        match reine.juger(&card) {
            Action::Reviser { tour, instruction } => {
                journal.push(format!("LaReine round {tour}: {}", instruction.trim()));
                // Tell the UI she is sending it back, then stream the rework live.
                let _ = tx.send(ChatEvent::Status {
                    message: format!("__reine_rework_start__|{}", instruction.trim()),
                });
                let consigne = format!(
                    "[Your supervisor LaReine reviewed your previous answer and sends you back to redo \
                     the work properly. Apply this in good faith; if part is clearly wrong, keep what \
                     was already right. Use your tools if needed, and reply in the user's language.]\n\n{}",
                    instruction.trim()
                );
                match boucle_react_memoire_multimodal(
                    &consigne,
                    session,
                    registry,
                    config,
                    tx,
                    memoire.clone(),
                    Vec::new(),
                    None,
                    None,
                )
                .await
                {
                    Ok(new_answer) if !new_answer.trim().is_empty() => {
                        answer = new_answer;
                        revised = true;
                        rounds = tour;
                    }
                    _ => {
                        journal.push("LaReine: the rework could not be completed".into());
                        break;
                    }
                }
            }
            // Approved or budget reached or escalated: keep the current answer.
            _ => {
                journal.push(ligne_verdict(&card));
                carte_finale = Some(card.clone());
                break;
            }
        }
    }

    // Emit the verdict (final judgment + her reasoning) once the rework is done. When
    // she sent it back, the summary states the rework count AND the final outcome.
    let summary = match (&carte_finale, revised) {
        (Some(c), true) => {
            let etat = match c.avis {
                Avis::Approuver => "approved",
                Avis::Escalader => "flagged for you",
                Avis::Reviser => "shipped best (budget reached)",
            };
            format!(
                "LaReine: redone {rounds}x, {etat} (relevance {} / method {} / objective {} / brand {})",
                c.pertinence, c.methodologie, c.objectif, c.conformite_marque
            )
        }
        (Some(c), false) => ligne_verdict(c),
        (None, _) => journal
            .last()
            .cloned()
            .unwrap_or_else(|| "LaReine reviewed the answer".to_string()),
    };
    let _ = tx.send(ChatEvent::Status {
        message: format!("__reine_verdict__|{summary}\u{1f}{analyse}"),
    });

    Revision {
        final_answer: answer,
        journal,
        revised,
        rounds,
        analyse,
    }
}
