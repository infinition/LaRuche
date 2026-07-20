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
    /// One-line outcome summary (also sent as the `__reine_verdict__` event); the
    /// caller persists it into the real session so the verdict survives a reload.
    pub resume: String,
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
    contexte: &str,
    atelier: &str,
) -> Option<Scorecard> {
    let demande = DemandeJugement {
        tier: Tier::Reponse,
        objectif: "",
        requete: prompt,
        brouillon: reponse,
        charte,
        contexte,
        atelier,
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
/// Format the last `n` conversation turns (User / Assistant) for the judge's context,
/// excluding the trailing draft (last assistant) and the current request (last user)
/// which the prompt already shows separately. Each turn is capped to keep the judge's
/// context bounded. Empty when `n == 0` or there is no earlier history.
fn construire_contexte(session: &Session, n: usize) -> String {
    if n == 0 {
        return String::new();
    }
    use crate::session::Message;
    let mut tours: Vec<(&'static str, String)> = Vec::new();
    for m in &session.messages {
        match m {
            Message::User(t) => tours.push(("User", t.clone())),
            Message::UserMultimodal { text, .. } => tours.push(("User", text.clone())),
            Message::Assistant(t) => tours.push(("LaRuche", t.clone())),
            _ => {}
        }
    }
    // Drop the trailing draft (last assistant) then the current request (last user).
    if tours.last().map(|t| t.0 == "LaRuche").unwrap_or(false) {
        tours.pop();
    }
    if tours.last().map(|t| t.0 == "User").unwrap_or(false) {
        tours.pop();
    }
    let start = tours.len().saturating_sub(n);
    tours[start..]
        .iter()
        .map(|t| {
            let body: String = t.1.trim().chars().take(800).collect();
            format!("{}: {}", t.0, body)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn revue_et_refaire(
    juge: &ProviderCreds,
    charte: &str,
    user_prompt: &str,
    answer_initial: &str,
    mode: &str,
    max_revues: u8,
    seuil: u8,
    contexte_messages: u8,
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
    // Recent conversation context for the judge (last N turns before this one), so
    // she reviews with awareness of what came before. Built once from the history
    // captured before the rework starts mutating the session.
    let contexte = construire_contexte(session, contexte_messages as usize);
    // Extended live introspection, built once per review: the ruche's memory
    // domains (top-level nodes). The Reine is the guardian of memory use, so she
    // judges knowing what LaRuche actually knows about.
    let etat_ruche = construire_etat_ruche(&memoire).await;
    // Détecte une « réponse » qui n'est PAS du travail : une erreur système/provider renvoyée
    // telle quelle (« Provider API error 500… », timeout réseau, builder reqwest…). La juger ou
    // la faire refaire ne sert à RIEN (observé : « redone 5x » contre un provider en panne).
    // Conservateur : texte court uniquement — une vraie réponse qui CITE une erreur ne matche pas.
    fn est_erreur_systeme(texte: &str) -> bool {
        let t = texte.trim();
        if t.is_empty() {
            return true;
        }
        if t.chars().count() > 500 {
            return false;
        }
        let bas = t.to_lowercase();
        [
            "provider api error",
            "error sending request",
            "builder error",
            "connection refused",
            "connect timeout",
            "timed out",
            "erreur fournisseur",
            "fatal provider",
        ]
        .iter()
        .any(|s| bas.contains(s))
    }

    let mut answer = answer_initial.to_string();
    let mut journal: Vec<String> = Vec::new();
    let mut revised = false;
    let mut rounds = 0u8;
    let mut analyse = String::new();
    let mut carte_finale: Option<Scorecard> = None;
    // Best draft seen so far (score, text): when the budget forces shipping, the
    // best-scoring draft goes out, not blindly the last one (anti-regression).
    let mut meilleur: (u8, String) = (0, String::new());
    // Cumulative wall-clock budget across ALL reworks: each rework is a full
    // agentic run, so unlimited rounds on a slow model need a global bound.
    let debut = std::time::Instant::now();
    let budget = std::time::Duration::from_secs(
        std::env::var("LARUCHE_REINE_BUDGET_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(600),
    );

    loop {
        // COURT-CIRCUIT : le brouillon est une erreur système, pas du travail. Inutile de payer
        // un appel juge ou des redos contre un provider en panne — on signale et on sort.
        // (Le plafond de rounds/budget reste intact : l'option « reine infinie » sert au testing.)
        if est_erreur_systeme(&answer) {
            tracing::warn!(extrait = %answer.chars().take(120).collect::<String>(), "reine: draft is a system error, skipping judge/redo");
            journal.push(
                "LaReine: the draft is a SYSTEM/PROVIDER ERROR, not work — no redo will fix it; flagged for you".into(),
            );
            break;
        }
        // Live workshop introspection for THIS draft (recomputed per round: a rework
        // appends its own turn and trace to the working session).
        let mut atelier = construire_atelier(session, registry);
        if !etat_ruche.is_empty() {
            atelier.push('\n');
            atelier.push_str(&etat_ruche);
        }
        let card = match juger_avec(
            &juge.provider,
            &juge.model,
            &juge.api_key,
            juge.api_base.as_deref(),
            &juge.ollama_url,
            &answer,
            user_prompt,
            charte,
            &contexte,
            &atelier,
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
        let score_courant = card.score_global();
        if meilleur.1.is_empty() || score_courant >= meilleur.0 {
            meilleur = (score_courant, answer.clone());
        }

        match reine.juger(&card) {
            Action::Reviser { tour, instruction } => {
                if debut.elapsed() >= budget {
                    if reine.regression(score_courant) {
                        answer = meilleur.1.clone();
                        journal.push(format!(
                            "LaReine anti-regression: kept the best draft (score {} > {score_courant})",
                            meilleur.0
                        ));
                    }
                    journal.push(format!(
                        "LaReine: cumulative time budget reached ({}s), shipping without further rework",
                        budget.as_secs()
                    ));
                    carte_finale = Some(card.clone());
                    break;
                }
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
                    // Un rework qui revient en ERREUR SYSTÈME n'écrase pas le brouillon précédent
                    // (sinon la boucle repartirait juger/refaire une erreur — gaspillage observé).
                    Ok(new_answer) if est_erreur_systeme(&new_answer) => {
                        tracing::warn!("reine rework came back as a system/provider error; keeping previous draft");
                        journal.push(
                            "LaReine: the rework came back as a SYSTEM/PROVIDER ERROR; keeping the previous draft".into(),
                        );
                        break;
                    }
                    Ok(new_answer) if !new_answer.trim().is_empty() => {
                        answer = new_answer;
                        revised = true;
                        rounds = tour;
                    }
                    // Cause VISIBLE (journal + logs) : le générique « could not be completed »
                    // masquait s'il s'agissait d'une réponse vide (fin de vol sans texte) ou
                    // d'une vraie erreur moteur/provider — indiagnosticable a posteriori.
                    Ok(_) => {
                        tracing::warn!("reine rework returned an empty answer");
                        journal.push(
                            "LaReine: the rework returned an EMPTY answer (flight ended without text); keeping the previous draft".into(),
                        );
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "reine rework failed");
                        journal.push(format!("LaReine: the rework FAILED ({e}); keeping the previous draft"));
                        break;
                    }
                }
            }
            // Approved, escalated, or round budget reached.
            action => {
                // Anti-regression: when the ROUND budget forces shipping (the judge
                // still wanted a revision), take the best-scoring draft, not the
                // last one. An explicit approval always ships as-is.
                if matches!(action, Action::Expedier(_))
                    && card.avis == Avis::Reviser
                    && reine.regression(score_courant)
                {
                    answer = meilleur.1.clone();
                    journal.push(format!(
                        "LaReine anti-regression: kept the best draft (score {} > {score_courant})",
                        meilleur.0
                    ));
                }
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
    // Scorecard journal: one JSONL line per completed review. This is the data the
    // future eval dashboard aggregates; without it every verdict evaporated.
    if let Some(c) = &carte_finale {
        journaliser_scorecard(c, mode, rounds, revised);
    }

    Revision {
        final_answer: answer,
        journal,
        revised,
        rounds,
        analyse,
        resume: summary,
    }
}

/// Compact workshop introspection for the judge: which tools the worker HAD, and
/// what it actually called to produce the current draft (since the last user
/// turn). This grounds the METHODOLOGY score in facts instead of the draft's own
/// claims, and gives the Reine live knowledge of the ruche's real capabilities.
fn construire_atelier(session: &Session, registry: &AbeilleRegistry) -> String {
    use crate::session::Message;
    let mut noms = registry.noms();
    noms.sort();
    let total = noms.len();
    let mut liste = noms.join(", ");
    if liste.len() > 600 {
        liste.truncate(600);
        if let Some(p) = liste.rfind(", ") {
            liste.truncate(p);
        }
        liste.push_str(", ...");
    }

    let dernier_user = session
        .messages
        .iter()
        .rposition(|m| matches!(m, Message::User(_) | Message::UserMultimodal { .. }))
        .map(|i| i + 1)
        .unwrap_or(0);
    let mut trace: Vec<String> = Vec::new();
    let mut echecs = 0usize;
    for m in &session.messages[dernier_user..] {
        match m {
            Message::ToolCall { name, .. } => trace.push(name.clone()),
            Message::Observation { tool, result, .. } => {
                let ko = result.trim_start().to_lowercase().starts_with("error");
                if ko {
                    echecs += 1;
                    if let Some(last) = trace.last_mut() {
                        if last == tool {
                            *last = format!("{tool} (FAILED)");
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let trace_txt = if trace.is_empty() {
        "No tool was called to produce this draft.".to_string()
    } else {
        let mut suite = trace.join(", ");
        if suite.len() > 400 {
            suite.truncate(400);
            if let Some(p) = suite.rfind(", ") {
                suite.truncate(p);
            }
            suite.push_str(", ...");
        }
        format!(
            "{} tool call(s): {}{}",
            trace.len(),
            suite,
            if echecs > 0 {
                format!(" ({echecs} failed)")
            } else {
                String::new()
            }
        )
    };
    format!("Tools available ({total}): {liste}\nTrace for this draft: {trace_txt}")
}

/// Top-level memory domains of the ruche, one compact line for the judge. The
/// Reine guards memory use: knowing the real domains lets her spot an answer
/// that ignored (or should have written to) the cognitive map.
async fn construire_etat_ruche(memoire: &Arc<dyn MemoireCognitive>) -> String {
    let Ok(v) = memoire.list_nodes().await else {
        return String::new();
    };
    let noeuds = v
        .as_array()
        .cloned()
        .or_else(|| v.get("nodes").and_then(|n| n.as_array()).cloned())
        .unwrap_or_default();
    let mut racines: Vec<String> = noeuds
        .iter()
        .filter(|n| {
            n.get("parent_id")
                .map(|p| p.is_null())
                .unwrap_or(true)
        })
        .filter_map(|n| n.get("id").and_then(|i| i.as_str()).map(String::from))
        .collect();
    if racines.is_empty() {
        return String::new();
    }
    racines.sort();
    let mut liste = racines.join(", ");
    if liste.len() > 300 {
        liste.truncate(300);
        if let Some(p) = liste.rfind(", ") {
            liste.truncate(p);
        }
        liste.push_str(", ...");
    }
    format!("Memory domains of the ruche: {liste}")
}

/// Append the review outcome to `evals/reine-scorecards.jsonl` (best-effort).
fn journaliser_scorecard(card: &Scorecard, mode: &str, rounds: u8, revised: bool) {
    let ligne = serde_json::json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "mode": mode,
        "rounds": rounds,
        "revised": revised,
        "relevance": card.pertinence,
        "methodology": card.methodologie,
        "objective": card.objectif,
        "brand": card.conformite_marque,
        "confidence": card.confiance,
        "avis": match card.avis {
            Avis::Approuver => "approve",
            Avis::Reviser => "revise",
            Avis::Escalader => "escalate",
        },
    });
    let _ = std::fs::create_dir_all("evals");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("evals/reine-scorecards.jsonl")
    {
        use std::io::Write;
        let _ = writeln!(f, "{ligne}");
    }
}
