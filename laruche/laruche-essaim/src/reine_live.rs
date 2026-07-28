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
// Compiled in: renaming this skill folder breaks the build rather than silently
// shipping a Queen with no charter. Keep the path in step with `skills/`.
const CHARTE_SKILL: &str = include_str!("../../skills/lareine-charter/SKILL.md");

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
/// Says WHY it failed. The automatic path only needs to know there is no verdict; a
/// hand-made call is a user waiting in front of the screen, and the three causes call for
/// completely different fixes: write something first, check the provider, or pick a model
/// able to follow the scorecard format.
#[allow(clippy::too_many_arguments)]
pub async fn juger_avec_raison(
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
    revues_precedentes: &str,
) -> Result<Scorecard, String> {
    let demande = DemandeJugement {
        tier: Tier::Reponse,
        objectif: "",
        requete: prompt,
        brouillon: reponse,
        charte,
        contexte,
        atelier,
        revues_precedentes,
    };
    let invite = construire_prompt(&demande);
    let messages = vec![serde_json::json!({ "role": "user", "content": invite })];

    let mut stream = match provider_chat_stream(
        provider,
        model,
        &messages,
        0.2, // low temperature: judging wants determinism
        // Room for a model that preambles before complying. The scorecard itself is ~80
        // tokens; the rest is headroom so a chatty judge still reaches the end of it.
        2048,
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
            return Err(format!("the judge provider ({provider}/{model}) refused the call: {e}"));
        }
    };

    let mut brut = String::new();
    while let Some(chunk) = stream.next().await {
        brut.push_str(&chunk.text);
    }
    let apercu: String = brut.chars().take(220).collect();
    tracing::info!(target: "reine", model = %model, len = brut.len(), preview = %apercu, "judge: raw output");
    if brut.trim().is_empty() {
        return Err(format!(
            "the judge ({provider}/{model}) answered nothing"
        ));
    }
    parser_scorecard(&brut).map_err(|e| {
        tracing::warn!(target: "reine", error = %e, "judge: output not parseable as scorecard");
        // The preview matters: it is the only way to see that the model wrote prose, or
        // wrapped its JSON in a code fence, rather than the expected scorecard.
        format!("{model} answered but not as a scorecard ({e}). It replied: {apercu}")
    })
}

/// Best-effort twin used by the automatic path, where a failed review must never break
/// a turn: any cause collapses to None.
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
    revues_precedentes: &str,
) -> Option<Scorecard> {
    juger_avec_raison(
        provider, model, api_key, api_base, ollama_url, reponse, prompt, charte, contexte,
        atelier, revues_precedentes,
    )
    .await
    .ok()
}

/// Opening of the rework brief handed to the worker, and the way to RECOGNISE one.
///
/// A rework is not a user mission. Its brief was being written into cognitive memory as
/// an episode title, so `Mission: [Your supervisor LaReine reviewed your previous
/// ANSWER...` came back on later turns as a past mission the agent had supposedly run.
/// Same leak as the `[SYSTEM]` paragraph before it: internal text re-entering as memory.
pub const PREFIXE_REVUE: &str = "[Your supervisor LaReine reviewed";

/// Is this prompt a rework brief rather than something the user asked?
pub fn est_consigne_revue(prompt: &str) -> bool {
    prompt.trim_start().starts_with(PREFIXE_REVUE)
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
    let rendre = |t: &(&'static str, String)| {
        let body: String = t.1.trim().chars().take(800).collect();
        format!("{}: {}", t.0, body)
    };
    let mut lignes: Vec<String> = Vec::new();
    // The OPENING request, when the window no longer reaches it. A conversation that
    // ran past `n` turns loses the objective it started from, and the judge then
    // reviews an answer against the last follow-up instead of against what was
    // actually asked. One line, only when it would otherwise be missing.
    if start > 0 {
        if let Some(premier) = tours.iter().find(|t| t.0 == "User") {
            lignes.push(format!("[opening request] {}", rendre(premier)));
            if start > 1 {
                lignes.push(format!("[... {} earlier turn(s) omitted ...]", start - 1));
            }
        }
    }
    lignes.extend(tours[start..].iter().map(rendre));
    lignes.join("\n")
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
    // Conservateur : texte court uniquement - une vraie réponse qui CITE une erreur ne matche pas.
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

    // Consecutive rounds that failed to improve the score. Observed live: a worker
    // stuck calling the wrong tool was sent back SIX times, each round a full agentic
    // run, because the only stop conditions were the time budget and the round cap.
    // A rework that does not move the score twice in a row is not going to move it a
    // third time; the judge is talking to a worker that cannot hear her.
    // Corrections already handed to the worker, so each judge call stops starting
    // blind: a fresh LLM call has no memory of what she asked one round earlier.
    let mut corrections_donnees: Vec<String> = Vec::new();
    let mut sans_progres: u8 = 0;
    let mut derniers_scores: Option<u8> = None;
    const STAGNATION_MAX: u8 = 2;

    loop {
        // COURT-CIRCUIT : le brouillon est une erreur système, pas du travail. Inutile de payer
        // un appel juge ou des redos contre un provider en panne - on signale et on sort.
        // (Le plafond de rounds/budget reste intact : l'option « reine infinie » sert au testing.)
        if est_erreur_systeme(&answer) {
            tracing::warn!(extrait = %answer.chars().take(120).collect::<String>(), "reine: draft is a system error, skipping judge/redo");
            journal.push(
                "LaReine: the draft is a SYSTEM/PROVIDER ERROR, not work - no redo will fix it; flagged for you".into(),
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
            &corrections_donnees.join("
"),
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
        // Strictly better, or we are going nowhere.
        if reine.regression(score_courant) || Some(score_courant) == derniers_scores {
            sans_progres = sans_progres.saturating_add(1);
        } else {
            sans_progres = 0;
        }
        derniers_scores = Some(score_courant);

        match reine.juger(&card) {
            Action::Reviser { tour, instruction } => {
                if sans_progres >= STAGNATION_MAX {
                    answer = meilleur.1.clone();
                    journal.push(format!(
                        "LaReine: {STAGNATION_MAX} rounds without progress (score stuck at                          {score_courant}); keeping the best draft and stopping rather than                          sending the same work back again"
                    ));
                    carte_finale = Some(card.clone());
                    break;
                }
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
                corrections_donnees.push(format!("round {tour}: {}", instruction.trim()));
                // Tell the UI she is sending it back, then stream the rework live.
                let _ = tx.send(ChatEvent::Status {
                    message: format!("__reine_rework_start__|{}", instruction.trim()),
                });
                // The scope has to be spelled out. With "redo the work properly, use your
                // tools if needed" and nothing else, an instruction like "replace all em
                // dashes per brand standards" was read as an order to go and EDIT THE
                // SOURCE FILES: the agent started patching cycle.rs during a question that
                // only asked it to explain the architecture. She judges an ANSWER, so her
                // instruction can only ever be about the answer.
                let consigne = format!(
                    "[Your supervisor LaReine reviewed your previous ANSWER and is sending it back.\n\
                     \n\
                     SCOPE: her instruction below is about the TEXT YOU ARE ABOUT TO WRITE for the \
                     user. It is NEVER an order to change anything on disk. Removing em dashes, \
                     sourcing a claim, changing the structure: all of that concerns your answer. Do \
                     NOT create, edit, patch or delete any file, and do not run any mutating command \
                     to satisfy it. If it seems to ask you to modify the project, you have \
                     misread it: apply it to your answer instead.\n\
                     \n\
                     KEEP what was already right, do not start from nothing. Read-only tools are \
                     encouraged: when she asks you to ground a claim, go and READ the file or the \
                     page rather than asserting again. Answer the user's original request, in their \
                     language.]\n\n\
                     Her instruction: {}",
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
                    // (sinon la boucle repartirait juger/refaire une erreur - gaspillage observé).
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
                    // d'une vraie erreur moteur/provider - indiagnosticable a posteriori.
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
        // Opt-in, and a no-op when off. Kept next to the scorecard because this is the
        // only moment where the request, the refused draft, the accepted one and her
        // reasoning all still exist together.
        journaliser_dataset(
            c,
            mode,
            rounds,
            revised,
            user_prompt,
            answer_initial,
            &answer,
            &corrections_donnees,
            &analyse,
        );
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
/// Pull the most identifying argument out of a tool call: the query, the URL, the
/// path, the sub-agent's task. It is what turns "web_deep_search was called" into
/// "web_deep_search looked for X", which is the difference between a name and evidence.
fn cle_appel(args: &serde_json::Value) -> Option<String> {
    for champ in ["query", "url", "q", "path", "task", "prompt", "command", "motif"] {
        if let Some(v) = args.get(champ).and_then(|v| v.as_str()) {
            let v = v.trim();
            if !v.is_empty() {
                return Some(couper(v, 110));
            }
        }
    }
    None
}

/// Truncate on a word boundary, so an excerpt never ends mid-word.
fn couper(texte: &str, max: usize) -> String {
    if texte.chars().count() <= max {
        return texte.to_string();
    }
    let t: String = texte.chars().take(max).collect();
    match t.rfind(char::is_whitespace) {
        Some(i) if i >= max * 3 / 4 => format!("{}...", t[..i].trim_end()),
        _ => format!("{}...", t.trim_end()),
    }
}

/// Distinct http(s) URLs appearing in a tool result, in order of appearance.
///
/// The single most decision-relevant fact for the judge is how many real sources the
/// draft actually rests on. Counting them beats any adjective about "thorough research".
fn urls_de(texte: &str, deja: &mut std::collections::BTreeSet<String>, out: &mut Vec<String>) {
    const BORNES: [char; 7] = ['"', '\'', '<', '>', ')', ']', '`'];
    let mut reste = texte;
    while let Some(pos) = reste.find("http") {
        let depart = &reste[pos..];
        if !depart.starts_with("http://") && !depart.starts_with("https://") {
            reste = &reste[pos + 4..];
            continue;
        }
        let fin = depart
            .find(|c: char| c.is_whitespace() || BORNES.contains(&c))
            .unwrap_or(depart.len());
        let url = depart[..fin].trim_end_matches(['.', ',', ';', ':']).to_string();
        if url.len() > 12 && deja.insert(url.clone()) {
            out.push(url);
        }
        reste = &depart[fin.max(8)..];
    }
}

/// What the agent actually DID to produce this draft, with the evidence it collected.
///
/// This used to be a list of tool NAMES and nothing else: `12 tool call(s): delegate,
/// delegate, web_deep_search, ...`. The charter asks the judge to require that every
/// claim rest on a real search result, and she could not see a single one, so she sent
/// perfectly grounded work back to be redone, turn after turn. The loop was structural,
/// not a whim of the judging model.
///
/// She now gets the three things a reviewer actually needs: WHAT was searched (queries
/// and URLs), WHAT came back (short extracts, and the scouts' own reports, which are
/// syntheses and therefore the densest evidence available), and HOW MANY distinct
/// sources the draft rests on. Everything is budgeted: this rides in the judge's prompt
/// on every single review.
fn construire_atelier(session: &Session, registry: &AbeilleRegistry) -> String {
    use crate::session::Message;
    const BUDGET_PREUVES: usize = 2600;
    const EXTRAIT_SCOUT: usize = 420;
    const EXTRAIT_AUTRE: usize = 180;

    let mut noms = registry.noms();
    noms.sort();
    let total = noms.len();
    let mut liste = noms.join(", ");
    if liste.len() > 400 {
        liste.truncate(400);
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

    let mut compte: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    // Each call remembers its identifying argument, so the observation that follows can
    // be attributed to the right one.
    let mut appels: Vec<(String, Option<String>)> = Vec::new();
    let mut preuves: Vec<String> = Vec::new();
    let mut urls: Vec<String> = Vec::new();
    let mut vues = std::collections::BTreeSet::new();
    let mut echecs = 0usize;
    let mut total_appels = 0usize;
    let mut budget = BUDGET_PREUVES;

    for m in &session.messages[dernier_user..] {
        match m {
            Message::ToolCall { name, args } => {
                total_appels += 1;
                *compte.entry(name.clone()).or_insert(0) += 1;
                appels.push((name.clone(), cle_appel(args)));
            }
            Message::Observation { tool, result, .. } => {
                let rate = result.trim_start().to_lowercase().starts_with("error");
                if rate {
                    echecs += 1;
                }
                urls_de(result, &mut vues, &mut urls);
                if budget == 0 {
                    continue;
                }
                let cle = appels
                    .iter()
                    .rposition(|(n, _)| n == tool)
                    .and_then(|i| appels[i].1.clone());
                // A scout returns a synthesis rather than raw data: densest evidence
                // per character, so it gets the larger share.
                let taille = if tool == "delegate" || tool == "spawn_specialist" {
                    EXTRAIT_SCOUT
                } else {
                    EXTRAIT_AUTRE
                };
                let corps = couper(
                    &result.split_whitespace().collect::<Vec<_>>().join(" "),
                    taille,
                );
                let marque = if rate { " FAILED" } else { "" };
                let ligne = match cle {
                    Some(k) => format!("- {tool}{marque} [{k}] -> {corps}"),
                    None => format!("- {tool}{marque} -> {corps}"),
                };
                budget = budget.saturating_sub(ligne.len().min(budget));
                preuves.push(ligne);
            }
            _ => {}
        }
    }

    if total_appels == 0 {
        return format!(
            "Tools available ({total}): {liste}\nTrace for this draft: NO tool was called. \
             Every factual claim in it is unverified by construction."
        );
    }

    let resume: Vec<String> = compte
        .iter()
        .map(|(n, c)| {
            if *c > 1 {
                format!("{n} x{c}")
            } else {
                n.clone()
            }
        })
        .collect();

    let mut sortie = format!(
        "Tools available ({total}): {liste}\nTrace for this draft: {total_appels} call(s): \
         {}{}\nDistinct sources actually fetched or returned: {}",
        resume.join(", "),
        if echecs > 0 {
            format!(" ({echecs} failed)")
        } else {
            String::new()
        },
        urls.len()
    );
    if !urls.is_empty() {
        let apercu: Vec<&str> = urls.iter().take(12).map(String::as_str).collect();
        let reste = urls.len() - apercu.len();
        sortie.push_str(&format!(
            "\nSources: {}{}",
            apercu.join(" | "),
            if reste > 0 {
                format!(" (+{reste} more)")
            } else {
                String::new()
            }
        ));
    }
    if !preuves.is_empty() {
        sortie.push_str(
            "\nWhat the tools actually returned. Judge grounding against THIS evidence, not \
             against your own knowledge, and do not ask for a search that already appears \
             here:\n",
        );
        sortie.push_str(&preuves.join("\n"));
    }
    sortie
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

/// Is training-data capture switched on? OFF unless explicitly enabled.
///
/// Read from disk at write time rather than plumbed through the call chain: this is
/// best-effort journalling like the scorecard beside it, and the setting is owned by the
/// node, which writes the same file.
fn dataset_actif() -> bool {
    std::fs::read_to_string("laruche-reine.json")
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("dataset").and_then(|b| b.as_bool()))
        .unwrap_or(false)
}

/// Append ONE rich record per completed review to `evals/reine-dataset.jsonl`.
///
/// The scorecard beside this keeps only numbers, which measures LaReine but trains
/// nothing. A review that sent work back has already produced, for free, the three
/// things a training set is built from:
///
///   * `prompt` + `chosen`             -> supervised fine-tuning
///   * `prompt` + `rejected` + `chosen` -> a preference pair (DPO/ORPO), the real prize:
///     same request, the draft she refused and the one she accepted, plus why
///   * `prompt` + `rejected` -> `critique` -> distilling her judgement into a smaller
///     judge model, so reviewing stops costing a large model every turn
///
/// EVERY text field goes through `secrets::masquer` first. Without that, one `env` dump
/// or verbose curl in a draft puts an API key in a file whose whole purpose is to be
/// copied around and fed to a trainer. A leak here is a leak everywhere, forever.
#[allow(clippy::too_many_arguments)]
fn journaliser_dataset(
    card: &Scorecard,
    mode: &str,
    rounds: u8,
    revised: bool,
    requete: &str,
    rejete: &str,
    retenu: &str,
    consignes: &[String],
    raisonnement: &str,
) {
    if !dataset_actif() {
        return;
    }
    let m = |s: &str| crate::secrets::masquer(s);
    let ligne = serde_json::json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "mode": mode,
        "rounds": rounds,
        // A record without a rejected draft is SFT-only: no preference pair to build.
        "paire_preference": revised && !rejete.trim().is_empty() && rejete.trim() != retenu.trim(),
        "prompt": m(requete),
        "rejected": m(rejete),
        "chosen": m(retenu),
        "critique": consignes.iter().map(|c| m(c)).collect::<Vec<_>>(),
        "reasoning": m(raisonnement),
        "scores": {
            "relevance": card.pertinence,
            "methodology": card.methodologie,
            "objective": card.objectif,
            "brand": card.conformite_marque,
            "confidence": card.confiance,
        },
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
        .open("evals/reine-dataset.jsonl")
    {
        use std::io::Write;
        let _ = writeln!(f, "{ligne}");
    }
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


/// A one-shot judgement the USER asked for, from the chat, outside any automatic mode.
///
/// Deliberately judge-only: it never rewrites the answer. The automatic path
/// ([`revue_et_refaire`]) sends the worker back to redo the work, which is right when
/// LaReine is on duty and watching every turn. Asked for by hand on a conversation
/// already in front of the user, silently replacing the message they are reading is the
/// wrong move: they want a verdict and a direction, and they decide what happens next.
///
/// The window is the caller's choice and is meant to be WIDER than the automatic one.
/// Four turns is enough to grade the last answer; it is not enough to say where a long
/// conversation went wrong, which is exactly why someone reaches for this button.
pub async fn juger_a_la_demande(
    juge: &ProviderCreds,
    charte: &str,
    session: &Session,
    registry: &AbeilleRegistry,
    memoire: Arc<dyn MemoireCognitive>,
    fenetre: usize,
) -> Result<Scorecard, String> {
    use crate::session::Message;

    // The answer under review and the request that produced it, read from the session
    // itself: unlike the automatic path there is no draft in flight to be handed in.
    // Three different failures used to collapse into one unactionable sentence. Each one
    // now says what it is, because the fix differs completely: write something first,
    // check the provider, or use a model that can follow the scorecard format.
    let reponse = session
        .messages
        .iter()
        .rev()
        .find_map(|m| match m {
            Message::Assistant(t) if !t.trim().is_empty() => Some(t.clone()),
            _ => None,
        })
        .ok_or("no answer to judge in this conversation yet")?;
    let prompt = session
        .messages
        .iter()
        .rev()
        .find_map(|m| match m {
            Message::User(t) => Some(t.clone()),
            Message::UserMultimodal { text, .. } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let contexte = construire_contexte(session, fenetre);
    let mut atelier = construire_atelier(session, registry);
    let etat = construire_etat_ruche(&memoire).await;
    if !etat.is_empty() {
        atelier.push('\n');
        atelier.push_str(&etat);
    }

    juger_avec_raison(
        &juge.provider,
        &juge.model,
        &juge.api_key,
        juge.api_base.as_deref(),
        &juge.ollama_url,
        &reponse,
        &prompt,
        charte,
        &contexte,
        &atelier,
        // A hand-made call is always a first look: there is no earlier round of hers.
        "",
    )
    .await
}

/// Render a scorecard for the UI: the verdict line, the scores, and the reasoning.
pub fn verdict_json(card: &Scorecard) -> serde_json::Value {
    serde_json::json!({
        "verdict": ligne_verdict(card),
        "avis": match card.avis {
            Avis::Approuver => "approve",
            Avis::Reviser => "revise",
            Avis::Escalader => "escalate",
        },
        "scores": {
            "pertinence": card.pertinence,
            "methodologie": card.methodologie,
            "objectif": card.objectif,
            "conformite_marque": card.conformite_marque,
            "confiance": card.confiance,
        },
        "instruction": card.instruction.trim(),
        "raison": card.raison.trim(),
        "analyse": card.analyse.trim(),
    })
}

/// Test-only handles on the two builders that decide what the judge can see.
#[cfg(test)]
pub(crate) fn construire_atelier_pour_test(
    session: &Session,
    registry: &AbeilleRegistry,
) -> String {
    construire_atelier(session, registry)
}

#[cfg(test)]
pub(crate) fn construire_contexte_pour_test(session: &Session, n: usize) -> String {
    construire_contexte(session, n)
}
