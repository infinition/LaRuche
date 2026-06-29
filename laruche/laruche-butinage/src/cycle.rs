//! The **cycle**: the foraging loop. Deliberately *dumb*: it orchestrates,
//! while policy lives elsewhere ([`crate::cap`]), as does error classification
//! ([`crate::meteo`]). No string heuristics here.
//!
//! Flow of one pass: assemble the context, call the model (with weather),
//! `analyser` the response into an [`Issue`], `cap` decides, act (post / relaunch /
//! récolte), checkpoint.

use crate::cap::boussole::{cap, Decision};
use crate::cap::vigie::Vigie;
use crate::carnet::Carnet;
use crate::evenement::{Emetteur, Evenement};
use crate::fournisseur::{Fournisseur, ReponseModele};
use crate::issue::{Bilan, FinDeVol, Issue, StopReason, TexteSeul};
use crate::messagerie::Message;
use crate::meteo::{reagir, ClasseErreur, Reaction};
use crate::nectar::Source;
use crate::outils::Outils;
use crate::reglages::Reglages;
use std::time::Duration;

/// Runs a foraging session to completion (mission accomplished, clarification, cap,
/// fatal error or sterile loop). The [`Carnet`] is mutated on each pass and
/// can be persisted (resume after crash).
pub async fn butiner(
    carnet: &mut Carnet,
    reglages: &Reglages,
    fournisseur: &dyn Fournisseur,
    outils: &dyn Outils,
    emet: &dyn Emetteur,
    source: Option<&dyn Source>,
    mut steering: Option<&mut tokio::sync::mpsc::Receiver<String>>,
) -> anyhow::Result<Bilan> {
    if carnet.historique.is_empty() {
        let mission = carnet.mission.clone();
        let pieces = std::mem::take(&mut carnet.pieces);
        carnet.historique.push(Message::utilisateur_multimodal(mission, pieces));
    }
    let mut vigie = Vigie::nouvelle(reglages.profil.seuils_vigie());
    let mut jauge = crate::cap::jauge::Jauge::nouvelle(reglages.context_max_tokens, 0.70, 0.85);
    let schemas = outils.schemas();

    // Tier 3 supervision state (only used when `reglages.supervision` is active).
    let mut sup_faites = carnet.itineraire.nb_faites();
    let mut sup_sans_progres: u32 = 0;
    let mut sup_interventions: u32 = 0;

    loop {
        if carnet.passe >= reglages.plafond_passes {
            let msg = format!("Cap of {} passes reached, may be incomplete.", reglages.plafond_passes);
            emet.emettre(Evenement::Statut(msg.clone()));
            return Ok(Bilan::nouveau(msg, FinDeVol::Plafond, carnet.passe));
        }

        // Steering: messages the user injects DURING the run (non-blocking).
        // Real user messages (visible/persisted), not internal nudges.
        if let Some(rx) = steering.as_deref_mut() {
            while let Ok(msg) = rx.try_recv() {
                let msg = msg.trim();
                if !msg.is_empty() {
                    carnet
                        .historique
                        .push(Message::utilisateur(format!("[Steering during run] {msg}")));
                    emet.emettre(Evenement::Statut("User steering injected.".into()));
                }
            }
        }

        // Escale: compaction (extractive) or, if the gauge is critical and a memory
        // is plugged in, cognitive consolidation (LLM, durable facts, fresh context).
        jauge.estimer(&reglages.systeme, &carnet.historique);
        let consolide =
            matches!(jauge.besoin(), crate::cap::jauge::Besoin::Consolider) && source.is_some();
        if consolide {
            if let Some(ev) =
                crate::escale::consolider(
                    carnet,
                    fournisseur,
                    source.unwrap(),
                    emet,
                    reglages.prompt_extraction.as_deref(),
                )
                .await
            {
                emet.emettre(ev);
                jauge.estimer(&reglages.systeme, &carnet.historique);
            }
        } else if let Some(ev) = crate::escale::peut_etre(carnet, &jauge, reglages.garder_recents) {
            emet.emettre(ev);
            jauge.estimer(&reglages.systeme, &carnet.historique);
        }

        // HARD guardrail (sliding window): escale compaction does not guarantee we
        // fit within the model's REAL window (e.g. llama.cpp n_ctx=32768). We drop the
        // OLDEST messages until we fit in the budget, keeping the recent ones, so
        // the model keeps the conversation thread without exceeding its window.
        tronquer_historique(carnet, &reglages.systeme, reglages.context_max_tokens);

        let messages = assembler(carnet, reglages);
        let reponse = match appeler_modele(fournisseur, &messages, &schemas, reglages, emet).await {
            Ok(r) => r,
            Err(motif) => {
                return Ok(Bilan::nouveau(
                    format!("Fatal provider error: {motif}"),
                    FinDeVol::Erreur(motif),
                    carnet.passe,
                ));
            }
        };

        // Real provider input tokens (if supplied): recalibrate the gauge for precise
        // compaction/consolidation decisions on the next turn.
        if let Some(u) = reponse.usage {
            jauge.maj_usage(u.entree as usize);
        }

        if !reponse.texte.is_empty() {
            emet.emettre(Evenement::Texte(reponse.texte.clone()));
        }
        carnet.historique.push(Message::assistant(reponse.texte.clone()));

        let issue = analyser(&reponse, carnet);
        // Candidate final text (before `cap` consumes the issue).
        let texte_final = match &issue {
            Issue::MissionAccomplie { resume, .. } => resume.clone(),
            Issue::TexteSeul(t) => t.texte.clone(),
            _ => reponse.texte.clone(),
        };

        let ctx = carnet.contexte_cap(reglages.relance_max, reglages.min_web_exploration);
        match cap(&ctx, issue) {
            Decision::Poser(fin) => {
                carnet.itineraire.finaliser();
                emet.emettre(Evenement::Fin(texte_final.clone()));
                return Ok(Bilan::nouveau(texte_final, fin, carnet.passe + 1));
            }
            Decision::Clarifier(q) => {
                emet.emettre(Evenement::Fin(q.clone()));
                return Ok(Bilan::nouveau(q.clone(), FinDeVol::Clarification(q), carnet.passe + 1));
            }
            Decision::Relancer(nudge) => {
                carnet.consommer_auto();
                emet.emettre(Evenement::Statut(format!(
                    "Relaunch ({}/{})",
                    carnet.auto_continue, reglages.relance_max
                )));
                carnet.historique.push(Message::nudge(nudge)); // internal: not persisted/displayed
            }
            Decision::Recolter(appels) => {
                carnet.rearmer_auto();
                if let Some(bilan) =
                    crate::recolte::recolter(&appels, carnet, reglages, outils, &mut vigie, emet).await
                {
                    return Ok(bilan); // clean stop: sterile loop
                }
            }
        }

        carnet.passe += 1;
        if let Some(chemin) = &reglages.chemin_carnet {
            if let Err(e) = carnet.sauver(chemin, chrono::Utc::now()) {
                tracing::warn!(error = %e, "carnet checkpoint failed");
            }
        }

        // Tier 3 supervision: the loop reached here because the task is CONTINUING
        // (relance or a non-sterile harvest). Measure plan progress; if it has stalled,
        // LaReine nudges the worker back on track, then escalates if it stays stuck.
        if let Some(sup) = &reglages.supervision {
            let faites = carnet.itineraire.nb_faites();
            if faites > sup_faites {
                sup_faites = faites;
                sup_sans_progres = 0;
            } else {
                sup_sans_progres += 1;
            }
            let etat = crate::cap::reine::EtatTache {
                etapes_faites: faites,
                etapes_totales: carnet.itineraire.etapes.len() as u32,
                passes_sans_progres: sup_sans_progres,
            };
            match crate::cap::reine::superviser(sup, &etat, sup_interventions) {
                crate::cap::reine::ActionSupervision::Continuer => {}
                crate::cap::reine::ActionSupervision::Intervenir(consigne) => {
                    emet.emettre(Evenement::Statut(
                        "Supervisor LaReine: task stalled, nudging back on track.".into(),
                    ));
                    carnet.historique.push(Message::nudge(consigne)); // internal, not persisted
                    sup_interventions += 1;
                    sup_sans_progres = 0;
                }
                crate::cap::reine::ActionSupervision::Escalader(motif) => {
                    emet.emettre(Evenement::Statut(format!("Supervisor LaReine escalation: {motif}")));
                    return Ok(Bilan::nouveau(motif.clone(), FinDeVol::Plafond, carnet.passe));
                }
            }
        }
    }
}

/// Sliding window: drops the OLDEST messages from history until
/// (system + history) fits within `budget_tokens`. **Conservative** estimate (`chars/3`:
/// tool schemas / JSON tokenize denser than `chars/4`, which underestimated and
/// let requests slip past `n_ctx`). Always keeps the last message (current
/// turn), so the model retains the recent conversation thread.
fn tronquer_historique(carnet: &mut Carnet, systeme: &str, budget_tokens: usize) {
    if budget_tokens == 0 {
        return;
    }
    let cible_chars = (budget_tokens as f32 * 0.72 * 3.0) as usize; // ~72% of budget, chars/3
    let total_chars = |h: &[Message]| -> usize {
        systeme.len() + h.iter().map(|m| m.contenu.len()).sum::<usize>()
    };
    while carnet.historique.len() > 1 && total_chars(&carnet.historique) > cible_chars {
        carnet.historique.remove(0);
    }
}

/// Assembles the messages sent to the model: system (stable tier) + history.
fn assembler(carnet: &Carnet, reglages: &Reglages) -> Vec<Message> {
    let mut v = Vec::with_capacity(carnet.historique.len() + 1);
    if !reglages.systeme.is_empty() {
        v.push(Message::systeme(reglages.systeme.clone()));
    }
    v.extend(carnet.historique.iter().cloned());
    v
}

/// Model call with weather policy (backoff/abandon). Key rotation and model
/// rerouting are handled internally by the `Fournisseur` adapter; the core
/// applies the wait and the abandon. Returns `Err(motif)` on permanent stop.
async fn appeler_modele(
    fournisseur: &dyn Fournisseur,
    messages: &[Message],
    schemas: &[serde_json::Value],
    reglages: &Reglages,
    emet: &dyn Emetteur,
) -> Result<ReponseModele, String> {
    let mut tentative = 0usize;
    loop {
        match fournisseur.repondre(messages, schemas).await {
            Ok(r) => return Ok(r),
            Err(e) => {
                tentative += 1;
                let classe = ClasseErreur::classer(e.status, e.retry_after.as_deref(), &e.corps);
                let now = chrono::Utc::now().timestamp();
                match reagir(
                    &classe,
                    tentative,
                    reglages.max_rate_limit,
                    reglages.max_transitoire,
                    false, // key rotation: already attempted by the adapter
                    false, // rerouting: already attempted by the adapter
                    now,
                ) {
                    Reaction::Patienter(s) => {
                        emet.emettre(Evenement::Statut(format!(
                            "Provider error ({}), retrying in {s}s (attempt {tentative}).",
                            e.status
                        )));
                        tokio::time::sleep(Duration::from_secs(s)).await;
                    }
                    Reaction::RotationCle | Reaction::Deroutement => {
                        emet.emettre(Evenement::Statut("Resuming after provider error.".into()));
                    }
                    Reaction::Stopper(motif) => {
                        // Diagnostic error: surface the REAL HTTP code + an excerpt of the
                        // provider body, otherwise "fatal error" is opaque (impossible to
                        // tell if it's a missing model, an absent key, a payload...).
                        let corps: String = e.corps.chars().take(300).collect();
                        let detail = if corps.trim().is_empty() {
                            format!("{motif} [HTTP {}]", e.status)
                        } else {
                            format!("{motif} [HTTP {}] {}", e.status, corps.trim())
                        };
                        return Err(detail);
                    }
                }
            }
        }
    }
}

/// Converts a model response into an [`Issue`] usable by the compass. Applies
/// the "plan" side effects in passing (updating the itinerary).
fn analyser(reponse: &ReponseModele, carnet: &mut Carnet) -> Issue {
    let mut appels = reponse.appels.clone();

    // `plan`: side effect (sets/updates the itinerary), removed from the call list.
    // Two accepted formats: `items:[{task,status}]` (statuses preserved, the model
    // re-emits its updated plan each turn) or `steps:[titres]` (all to do).
    let plan_trouve = appels.iter().any(|a| a.nom == "plan");
    if let Some(pos) = appels.iter().position(|a| a.nom == "plan") {
        let a = appels.remove(pos);
        if let Some(items) = a.args.get("items").and_then(|v| v.as_array()) {
            let etapes: Vec<crate::itineraire::Etape> = items
                .iter()
                .filter_map(|it| {
                    let titre = it
                        .get("task")
                        .or_else(|| it.get("titre"))
                        .and_then(|v| v.as_str())?;
                    let statut = it
                        .get("status")
                        .or_else(|| it.get("statut"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("pending");
                    Some(crate::itineraire::Etape {
                        titre: titre.to_string(),
                        statut: crate::itineraire::StatutEtape::depuis(statut),
                    })
                })
                .collect();
            if !etapes.is_empty() {
                carnet.itineraire.etapes = etapes;
            }
        } else if let Some(steps) = a.args.get("steps").and_then(|v| v.as_array()) {
            let titres: Vec<String> = steps
                .iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect();
            if !titres.is_empty() {
                carnet.itineraire.definir(titres);
            }
        }
    }

    // Explicit completion tools: `mission_accomplie` (native foraging) or `task_complete`
    // (already registered in LaRuche). We recognize both.
    if let Some(a) = appels
        .iter()
        .find(|a| a.nom == "mission_accomplie" || a.nom == "task_complete")
    {
        let resume = a
            .args
            .get("resume")
            .or_else(|| a.args.get("summary"))
            .and_then(|v| v.as_str())
            .unwrap_or("Mission accomplished.")
            .to_string();
        let confiance = a
            .args
            .get("confiance")
            .or_else(|| a.args.get("confidence"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        return Issue::MissionAccomplie { resume, confiance };
    }

    if let Some(a) = appels.iter().find(|a| a.nom == "clarify") {
        let q = a
            .args
            .get("question")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        return Issue::Clarification(q);
    }

    if !appels.is_empty() {
        return Issue::Outils(appels);
    }

    // Plan posted alone (no other tool): productive act, we continue (bounded), not an end.
    if plan_trouve {
        return Issue::PlanEnregistre;
    }

    let tronquee = matches!(reponse.stop, StopReason::Longueur) || bloc_outil_non_ferme(&reponse.texte);
    Issue::TexteSeul(TexteSeul {
        texte: reponse.texte.clone(),
        fin_native: Some(reponse.stop),
        plan_inacheve: carnet.itineraire.a_des_ouvertes(),
        malforme: ressemble_a_un_outil(&reponse.texte),
        tronquee,
    })
}

/// Does the text look like a tool_call (but wasn't parsed)? Rail for weak models.
fn ressemble_a_un_outil(t: &str) -> bool {
    t.contains("<tool_call>") || (t.contains("\"name\"") && t.contains("\"arguments\""))
}

/// Tool block opened but never closed: output likely truncated.
fn bloc_outil_non_ferme(t: &str) -> bool {
    let ouverts = t.matches("<tool_call>").count();
    let fermes = t.matches("</tool_call>").count();
    ouverts > fermes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carnet::ModeMission;
    use crate::evenement::Silencieux;
    use crate::fournisseur::ErreurFournisseur;
    use crate::issue::Appel;
    use crate::outils::ResultatOutil;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Mutex;

    fn t0() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn tronque_garde_les_recents_et_tient_dans_le_budget() {
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        carnet.historique.clear();
        for i in 0..50 {
            carnet.historique.push(Message::utilisateur("x".repeat(300) + &i.to_string()));
        }
        let avant = carnet.historique.len();
        // tight budget: must drop old ones, keep at least the last
        tronquer_historique(&mut carnet, "systeme", 1000);
        assert!(carnet.historique.len() < avant, "old ones dropped");
        assert!(!carnet.historique.is_empty(), "keeps at least the current turn");
        // the LAST message (the most recent) is preserved
        assert!(carnet.historique.last().unwrap().contenu.ends_with("49"));
    }

    /// Mock provider: serves pre-recorded responses, or a fixed error.
    struct FournisseurScript {
        reponses: Mutex<std::collections::VecDeque<ReponseModele>>,
        erreur: Option<ErreurFournisseur>,
    }
    impl FournisseurScript {
        fn scenario(reponses: Vec<ReponseModele>) -> Self {
            Self { reponses: Mutex::new(reponses.into()), erreur: None }
        }
        fn en_erreur(status: u16) -> Self {
            Self {
                reponses: Mutex::new(Default::default()),
                erreur: Some(ErreurFournisseur { status, retry_after: None, corps: "boom".into() }),
            }
        }
    }
    #[async_trait]
    impl Fournisseur for FournisseurScript {
        async fn repondre(
            &self,
            _m: &[Message],
            _s: &[serde_json::Value],
        ) -> Result<ReponseModele, ErreurFournisseur> {
            if let Some(e) = &self.erreur {
                return Err(e.clone());
            }
            Ok(self.reponses.lock().unwrap().pop_front().unwrap_or(ReponseModele {
                texte: "(end of script)".into(),
                stop: StopReason::FinTour,
                appels: vec![],
                usage: None,
            }))
        }
    }

    struct OutilsMock;
    #[async_trait]
    impl Outils for OutilsMock {
        async fn executer(&self, appel: &Appel) -> ResultatOutil {
            ResultatOutil::ok(format!("result of {}", appel.nom))
        }
        fn idempotent(&self, nom: &str) -> bool {
            nom.starts_with("web_")
        }
    }

    fn rep_texte(t: &str) -> ReponseModele {
        ReponseModele { texte: t.into(), stop: StopReason::FinTour, appels: vec![], usage: None }
    }
    fn rep_appel(nom: &str, args: serde_json::Value) -> ReponseModele {
        ReponseModele { texte: String::new(), stop: StopReason::Outils, appels: vec![Appel::nouveau(nom, args)], usage: None }
    }

    #[tokio::test]
    async fn termine_sur_mission_accomplie() {
        let four = FournisseurScript::scenario(vec![rep_appel(
            "mission_accomplie",
            json!({"resume": "tout est fait", "confiance": 0.9}),
        )]);
        let mut carnet = Carnet::ouvrir("fais X", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Accomplie);
        assert_eq!(bilan.texte, "tout est fait");
        assert_eq!(bilan.passes, 1);
    }

    #[tokio::test]
    async fn execute_un_outil_puis_termine() {
        let four = FournisseurScript::scenario(vec![
            rep_appel("web_search", json!({"q": "rust"})),
            rep_appel("mission_accomplie", json!({"resume": "trouvé"})),
        ]);
        let mut carnet = Carnet::ouvrir("cherche", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Accomplie);
        assert_eq!(carnet.recolte_web, 1, "the web call must be counted");
        // a tool observation was re-injected into the history
        assert!(carnet
            .historique
            .iter()
            .any(|m| m.outil.as_deref() == Some("web_search")));
        assert_eq!(bilan.passes, 2);
    }

    #[tokio::test]
    async fn texte_seul_standard_termine_tout_de_suite() {
        let four = FournisseurScript::scenario(vec![rep_texte("voici la réponse directe")]);
        let mut carnet = Carnet::ouvrir("salut", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Accomplie);
        assert_eq!(bilan.texte, "voici la réponse directe");
    }

    #[tokio::test]
    async fn plan_force_l_auto_continuation() {
        // 1) post a plan (1 step); 2) text only (step still open), must relaunch;
        // 3) mission_accomplie, end. Pass 2 must NOT conclude.
        let four = FournisseurScript::scenario(vec![
            rep_appel("plan", json!({"steps": ["chercher la source"]})),
            rep_texte("je réfléchis à voix haute mais je n'agis pas"),
            rep_appel("mission_accomplie", json!({"resume": "ok"})),
        ]);
        let mut carnet = Carnet::ouvrir("mission", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Accomplie);
        assert!(carnet.auto_continue >= 1 || carnet.passe >= 2, "auto-continuation must have triggered");
    }

    #[tokio::test]
    async fn erreur_fatale_arrete_proprement() {
        let four = FournisseurScript::en_erreur(400); // invalid request: fatal, no fallback
        let mut carnet = Carnet::ouvrir("x", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None)
            .await
            .unwrap();
        assert!(matches!(bilan.fin, FinDeVol::Erreur(_)));
    }
}
