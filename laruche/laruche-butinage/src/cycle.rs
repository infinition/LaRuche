//! The **cycle**: the foraging loop. Deliberately *dumb*: it orchestrates,
//! while policy lives elsewhere ([`crate::cap`]), as does error classification
//! ([`crate::meteo`]). No string heuristics here.
//!
//! Flow of one pass: assemble the context, call the model (with weather),
//! `analyser` the response into an [`Issue`], `cap` decides, act (post / relaunch /
//! récolte), checkpoint.

use crate::cap::boussole::{cap, Decision, PROTOCOLE_EXPLORATION};
use crate::cap::vigie::Vigie;
use crate::carnet::{Carnet, ModeMission};
use crate::evenement::{Emetteur, Evenement};
use crate::fournisseur::{Fournisseur, ReponseModele};
use crate::issue::{Appel, Bilan, FinDeVol, Issue, StopReason, TexteSeul};
use crate::messagerie::{Message, Role};
use crate::meteo::{reagir, ClasseErreur, Reaction};
use crate::nectar::Source;
use crate::outils::Outils;
use crate::reglages::{ProfilModele, Reglages};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Consecutive passes where EVERY tool call was blocked by the vigie before the
/// loop lands (a stuck model re-emitting the same blocked call would otherwise
/// burn one model call per pass until the pass ceiling).
const MAX_PASSES_BLOQUEES: u32 = 3;

/// Cooldown (passes) after a failed cognitive consolidation before retrying it
/// (each attempt costs an auxiliary LLM call).
const GEL_CONSOLIDATION: usize = 3;

/// Below this pass ceiling, no wind-down nudge (the mission is too short for a
/// "wrap up" phase to mean anything).
const WINDDOWN_PLAFOND_MIN: usize = 5;

/// Wind-down warning injected ONCE when ~80% of the pass ceiling or token budget is
/// consumed. Without it, `FinDeVol::Plafond`/`Budget` guillotines the model mid-flight
/// and the user gets "may be incomplete" instead of a synthesis of what WAS found.
fn nudge_winddown(restant: &str) -> String {
    format!(
        "[Wind-down] {restant}. The mission is about to be capped. STOP opening new \
         angles or launching broad new searches. Consolidate NOW: check what you already \
         have, close the plan steps you can, and deliver your final answer (with sources) \
         via mission_accomplie before the ceiling cuts you off."
    )
}

fn est_annule(flag: Option<&AtomicBool>) -> bool {
    flag.is_some_and(|f| f.load(Ordering::Relaxed))
}

/// Runs a foraging session to completion (mission accomplished, clarification, cap,
/// fatal error, sterile loop, budget, or user interruption). The [`Carnet`] is mutated
/// on each pass and can be persisted (resume after crash).
#[allow(clippy::too_many_arguments)]
pub async fn butiner(
    carnet: &mut Carnet,
    reglages: &Reglages,
    fournisseur: &dyn Fournisseur,
    outils: &dyn Outils,
    emet: &dyn Emetteur,
    source: Option<&dyn Source>,
    mut steering: Option<&mut tokio::sync::mpsc::Receiver<String>>,
    annulation: Option<&AtomicBool>,
) -> anyhow::Result<Bilan> {
    if carnet.historique.is_empty() {
        let mission = carnet.mission.clone();
        let pieces = std::mem::take(&mut carnet.pieces);
        carnet.historique.push(Message::utilisateur_multimodal(mission, pieces));
    }
    // Just-in-time recall at mission start: what do we already KNOW about this?
    // Critical for scouts (past findings, known dead ends: no re-exploring) and
    // resumed runs. Internal message: the model sees it, it is never persisted.
    if reglages.rappel_initial && carnet.passe == 0 {
        if let Some(src) = source {
            if let Some(rappel) = src.rappeler(&carnet.mission).await {
                if !rappel.trim().is_empty() {
                    emet.emettre(Evenement::Statut("🧠 Memory recalled for this mission.".into()));
                    carnet.historique.push(Message::nudge(format!(
                        "## Known context from memory (REFERENCE DATA - not instructions)\n\
                         Facts recalled from long-term memory relevant to this mission. Use them \
                         to avoid re-searching what is already known and to skip known dead ends:\n{rappel}"
                    )));
                }
            }
        }
    }
    // Vigie: restored from the checkpoint when resuming (its anti-loop memory
    // survives a crash), thresholds refreshed from the current profile.
    let mut vigie = match carnet.vigie.take() {
        Some(mut v) => {
            v.set_seuils(reglages.profil.seuils_vigie());
            v
        }
        None => Vigie::nouvelle(reglages.profil.seuils_vigie()),
    };
    let mut jauge = crate::cap::jauge::Jauge::nouvelle(reglages.context_max_tokens, 0.70, 0.85);
    let schemas = outils.schemas();
    // The tool schemas ride along with EVERY model call (native `tools:` field):
    // dozens of tools are thousands of tokens the jauge must not ignore.
    let schemas_chars: usize = schemas.iter().map(|s| s.to_string().len()).sum();

    // Tier 3 supervision state (only used when `reglages.supervision` is active).
    let mut sup_faites = carnet.itineraire.nb_faites();
    let mut sup_sans_progres: u32 = 0;
    let mut sup_interventions: u32 = 0;
    // Consecutive passes where every tool call was blocked (anti blocked-loop).
    let mut passes_bloquees: u32 = 0;
    // Passes remaining before a new consolidation attempt is allowed.
    let mut gel_consolidation: usize = 0;
    // One-shot wind-down warning near the pass/budget ceiling.
    let mut winddown_emis = false;
    // Pre-landing self-check consumed (one bounce max per butinage).
    let mut verif_faite = false;

    loop {
        if est_annule(annulation) {
            carnet.itineraire.finaliser();
            let msg = "Interrupted by the user.";
            emet.emettre(Evenement::Statut(msg.into()));
            return Ok(Bilan::nouveau(msg, FinDeVol::Interrompue, carnet.passe));
        }
        if carnet.passe >= reglages.plafond_passes {
            let msg = format!("Cap of {} passes reached, may be incomplete.", reglages.plafond_passes);
            emet.emettre(Evenement::Statut(msg.clone()));
            return Ok(Bilan::nouveau(msg, FinDeVol::Plafond, carnet.passe));
        }
        // Cumulative token budget (input + output): hard spend ceiling for long runs.
        if reglages.budget_tokens > 0 && carnet.tokens_total() >= reglages.budget_tokens {
            let msg = format!(
                "Token budget exhausted ({} used / {} allowed), landing with partial results.",
                carnet.tokens_total(),
                reglages.budget_tokens
            );
            emet.emettre(Evenement::Statut(msg.clone()));
            return Ok(Bilan::nouveau(msg, FinDeVol::Budget, carnet.passe));
        }

        // Wind-down: one-shot warning at ~80% of the pass ceiling or token budget,
        // so the model lands with a synthesis instead of being guillotined.
        if !winddown_emis {
            let passes_proches = reglages.plafond_passes >= WINDDOWN_PLAFOND_MIN
                && carnet.passe >= reglages.plafond_passes * 4 / 5;
            let budget_proche = reglages.budget_tokens > 0
                && carnet.tokens_total() >= reglages.budget_tokens * 4 / 5;
            if passes_proches || budget_proche {
                winddown_emis = true;
                let restant = if passes_proches {
                    format!(
                        "You have used {} of your {} passes",
                        carnet.passe, reglages.plafond_passes
                    )
                } else {
                    format!(
                        "You have used {} of your {} token budget",
                        carnet.tokens_total(),
                        reglages.budget_tokens
                    )
                };
                emet.emettre(Evenement::Statut("⏳ Wind-down: nearing the cap, asking for a final synthesis.".into()));
                carnet.historique.push(Message::nudge(nudge_winddown(&restant)));
            }
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

        // Escale: compaction (LLM summary by default, extractive fallback) or, if the
        // gauge is critical and a memory is plugged in, cognitive consolidation
        // (durable facts, fresh context). A failed consolidation falls back to
        // compaction and is frozen for a few passes (each attempt costs an LLM call).
        jauge.estimer(&reglages.systeme, &carnet.historique, schemas_chars);
        let veut_consolider = matches!(jauge.besoin(), crate::cap::jauge::Besoin::Consolider)
            && source.is_some()
            && gel_consolidation == 0;
        if veut_consolider {
            match crate::escale::consolider(
                carnet,
                fournisseur,
                source.unwrap(),
                emet,
                reglages.prompt_extraction.as_deref(),
                budget_rendu(reglages, &jauge),
            )
            .await
            {
                Some(ev) => {
                    emet.emettre(ev);
                    jauge.estimer(&reglages.systeme, &carnet.historique, schemas_chars);
                }
                None => {
                    // +1: the cooldown is decremented at the end of THIS very pass; without
                    // it the effective freeze would be GEL_CONSOLIDATION - 1 passes.
                    gel_consolidation = GEL_CONSOLIDATION + 1;
                    if let Some(ev) = compacter(carnet, fournisseur, &jauge, reglages, emet).await {
                        emet.emettre(ev);
                        jauge.estimer(&reglages.systeme, &carnet.historique, schemas_chars);
                    }
                }
            }
        } else if let Some(ev) = compacter(carnet, fournisseur, &jauge, reglages, emet).await {
            emet.emettre(ev);
            jauge.estimer(&reglages.systeme, &carnet.historique, schemas_chars);
        }
        gel_consolidation = gel_consolidation.saturating_sub(1);

        // HARD guardrail (sliding window): escale compaction does not guarantee we
        // fit within the model's REAL window (e.g. llama.cpp n_ctx=32768). Cuts down
        // to a LOW WATERMARK in one go (stable prefix across many passes: prompt-cache
        // friendly), pinning the mission anchor, using the jauge's calibrated ratio.
        tronquer_historique(
            carnet,
            &reglages.systeme,
            reglages.context_max_tokens,
            jauge.chars_par_token(),
        );

        let messages = assembler(carnet, reglages, outils.nouvelles_capacites().as_deref());
        let reponse =
            match appeler_modele(fournisseur, &messages, &schemas, reglages, emet, annulation).await
            {
                Ok(r) => r,
                Err(EchecAppel::Interrompu) => {
                    carnet.itineraire.finaliser();
                    return Ok(Bilan::nouveau(
                        "Interrupted by the user.",
                        FinDeVol::Interrompue,
                        carnet.passe,
                    ));
                }
                Err(EchecAppel::Fatal(motif)) => {
                    return Ok(Bilan::nouveau(
                        format!("Fatal provider error: {motif}"),
                        FinDeVol::Erreur(motif),
                        carnet.passe,
                    ));
                }
            };

        // Real provider tokens (if supplied): recalibrate the gauge for precise
        // compaction/consolidation decisions, and feed the cumulative budget.
        if let Some(u) = reponse.usage {
            jauge.maj_usage(u.entree as usize);
            carnet.tokens_entree_total += u.entree as u64;
            carnet.tokens_sortie_total += u.sortie as u64;
        } else if reglages.budget_tokens > 0 {
            // Provider silent on usage: feed the budget with the jauge ESTIMATE so the
            // spend ceiling stays a guarantee instead of silently never triggering.
            let cpt = jauge.chars_par_token().max(1.0);
            carnet.tokens_entree_total += jauge.utilise as u64;
            let sortie_chars = reponse.texte.len()
                + reponse.appels.iter().map(|a| a.args.to_string().len() + a.nom.len()).sum::<usize>();
            carnet.tokens_sortie_total += (sortie_chars as f32 / cpt) as u64;
        }

        if !reponse.texte.is_empty() {
            emet.emettre(Evenement::Texte(reponse.texte.clone()));
        }
        // The assistant message carries its tool calls: the transcript stays coherent
        // (the model must see WHICH calls produced the observations that follow).
        if !reponse.texte.is_empty() || !reponse.appels.is_empty() {
            carnet
                .historique
                .push({
                    let mut m = Message::assistant_avec_appels(
                        reponse.texte.clone(),
                        reponse.appels.clone(),
                    );
                    // Le raisonnement voyage avec le modele qui l'a produit: il
                    // ne sera rejoue qu'a lui, jamais a un autre fournisseur.
                    m.reasoning = reponse.reasoning.clone();
                    m.reasoning_model = reponse.reasoning_model.clone();
                    m
                });
        }

        let mode_avant = carnet.mode;
        // Empreinte de l'itinéraire AVANT analyse : si l'appel `plan` (outil natif) le modifie,
        // on notifie l'UI - sans ça la barre de plan ne bouge jamais avec les modèles qui
        // émettent leur plan en tool call natif plutôt qu'en bloc <plan> texte.
        let itineraire_avant: Vec<(String, String)> = carnet
            .itineraire
            .etapes
            .iter()
            .map(|e| (e.titre.clone(), e.statut.cle().to_string()))
            .collect();
        let issue = analyser(&reponse, carnet, reglages.profil);
        let itineraire_apres: Vec<(String, String)> = carnet
            .itineraire
            .etapes
            .iter()
            .map(|e| (e.titre.clone(), e.statut.cle().to_string()))
            .collect();
        if itineraire_apres != itineraire_avant && !itineraire_apres.is_empty() {
            emet.emettre(Evenement::Itineraire { etapes: itineraire_apres });
        }
        // Candidate final text (before `cap` consumes the issue).
        let texte_final = match &issue {
            Issue::MissionAccomplie { resume, .. } => resume.clone(),
            Issue::TexteSeul(t) => t.texte.clone(),
            _ => reponse.texte.clone(),
        };

        // Self-check armed once per butinage; sub-agents skip it (their parent
        // cross-checks via gardienne, and each bounce costs a model call per scout).
        let ctx = carnet.contexte_cap(
            reglages.relance_max,
            reglages.min_web_exploration,
            reglages.delegation_disponible,
            !verif_faite && reglages.delegation_disponible,
        );
        let etait_fin = matches!(issue, Issue::MissionAccomplie { .. });
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
                if etait_fin {
                    // The relaunch of a mission_accomplie IS the self-check bounce:
                    // disarm it so the next completion lands (no verification loop).
                    verif_faite = true;
                    emet.emettre(Evenement::Statut("🔎 Self-check before landing…".into()));
                }
                carnet.consommer_auto();
                emet.emettre(Evenement::Statut(format!(
                    "Relaunch ({}/{})",
                    carnet.auto_continue, reglages.relance_max
                )));
                carnet.historique.push(Message::nudge(nudge)); // internal: not persisted/displayed
            }
            Decision::Recolter(appels) => {
                let moisson = crate::recolte::recolter(
                    &appels, carnet, reglages, outils, &mut vigie, emet, annulation,
                )
                .await;
                if let Some(bilan) = moisson.arret {
                    return Ok(bilan); // clean stop: sterile loop / interruption
                }
                if moisson.executes > 0 {
                    // Real progress: rearm the sterile-relaunch budget.
                    carnet.rearmer_auto();
                    passes_bloquees = 0;
                } else {
                    // EVERY call was blocked: no execution happened. A stuck model
                    // re-emitting the same blocked call must not burn the pass ceiling.
                    passes_bloquees += 1;
                    if passes_bloquees >= MAX_PASSES_BLOQUEES {
                        carnet.itineraire.finaliser();
                        return Ok(Bilan::nouveau(
                            "Stopped: every tool call was blocked for several consecutive passes.",
                            FinDeVol::BoucleSterile(format!(
                                "{passes_bloquees} consecutive passes with all calls blocked"
                            )),
                            carnet.passe + 1,
                        ));
                    }
                }
            }
        }

        // Mid-run escalation to exploration (the model called `research_mode`, or set
        // a `mode` on its plan): inject the deep-research protocol ONCE, positioned
        // after this pass's observations so the next turn starts with it fresh.
        if mode_avant != carnet.mode {
            emet.emettre(Evenement::Statut(
                "🔎 Deep-research mode activated (exploration protocol injected).".into(),
            ));
            carnet.historique.push(Message::nudge(PROTOCOLE_EXPLORATION));
        }

        carnet.passe += 1;
        if let Some(chemin) = &reglages.chemin_carnet {
            carnet.vigie = Some(vigie.clone()); // anti-loop memory survives a crash
            if let Err(e) = carnet.sauver(chemin, chrono::Utc::now()) {
                tracing::warn!(error = %e, "carnet checkpoint failed");
            }
            carnet.vigie = None;
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
                    let mut nudge = consigne;
                    // Name the concrete next step: a generic "advance the plan" nudge is
                    // far weaker than pointing at the exact step to attack.
                    let etape = carnet
                        .itineraire
                        .prochaine_ouverte()
                        .and_then(|i| carnet.itineraire.etapes.get(i))
                        .map(|e| e.titre.clone())
                        .unwrap_or_else(|| carnet.mission.clone());
                    nudge.push_str(&format!("\nThe next unfinished step is: \"{etape}\"."));
                    // In-loop recall on stagnation: has memory seen this problem before?
                    if let Some(src) = source {
                        if let Some(rappel) = src.rappeler(&etape).await {
                            if !rappel.trim().is_empty() {
                                nudge.push_str(&format!(
                                    "\n\nPossibly relevant memory (reference):\n{rappel}"
                                ));
                            }
                        }
                    }
                    carnet.historique.push(Message::nudge(nudge)); // internal, not persisted
                    sup_interventions += 1;
                    sup_sans_progres = 0;
                }
                crate::cap::reine::ActionSupervision::Escalader(motif) => {
                    emet.emettre(Evenement::Statut(format!("Supervisor LaReine escalation: {motif}")));
                    return Ok(Bilan::nouveau(motif.clone(), FinDeVol::Escalade(motif), carnet.passe));
                }
            }
        }
    }
}

/// Character budget for a rendered transcript sent to an AUXILIARY call (compaction
/// summary, consolidation extraction): half the model window, in calibrated chars.
/// Without this cap the auxiliary call - fired precisely when the context is nearly
/// FULL - would itself overflow the window and structurally always fail on small n_ctx.
fn budget_rendu(reglages: &Reglages, jauge: &crate::cap::jauge::Jauge) -> usize {
    (reglages.context_max_tokens as f32 * 0.5 * jauge.chars_par_token()) as usize
}

/// Compaction dispatch: LLM summary (default) or extractive, per settings.
async fn compacter(
    carnet: &mut Carnet,
    fournisseur: &dyn Fournisseur,
    jauge: &crate::cap::jauge::Jauge,
    reglages: &Reglages,
    emet: &dyn Emetteur,
) -> Option<Evenement> {
    if reglages.compaction_llm {
        crate::escale::compacter_intelligent(
            carnet,
            fournisseur,
            jauge,
            reglages.garder_recents,
            emet,
            budget_rendu(reglages, jauge),
        )
        .await
    } else {
        crate::escale::peut_etre(carnet, jauge, reglages.garder_recents)
    }
}

/// Sliding-window guardrail. Above the trigger (90% of budget), drops the OLDEST
/// messages down to a LOW WATERMARK (60% of budget) **in one go**: the prefix then
/// stays stable for many passes (prompt-cache friendly), instead of shifting by one
/// message per pass. Pins the mission anchor (and its companion resume/consolidation
/// note) so the model NEVER loses sight of its mission, and always keeps the last
/// message (current turn). Uses the jauge's calibrated chars/token ratio.
fn tronquer_historique(
    carnet: &mut Carnet,
    systeme: &str,
    budget_tokens: usize,
    chars_par_token: f32,
) {
    if budget_tokens == 0 || carnet.historique.len() < 2 {
        return;
    }
    let declencheur = (budget_tokens as f32 * 0.90 * chars_par_token) as usize;
    let cible = (budget_tokens as f32 * 0.60 * chars_par_token) as usize;
    // Full payload cost (text + serialized tool calls + multimodal pieces): a message
    // whose weight is in its `pieces`/`appels` must not look free to the guardrail.
    let total: usize =
        systeme.len() + carnet.historique.iter().map(|m| m.cout_chars()).sum::<usize>();
    if total <= declencheur {
        return;
    }
    // Pinned prefix: the mission anchor, plus its companion when the head is the
    // [user, system-resume] pair (post-compaction) or [system-note, user] (post-
    // consolidation).
    let h = &carnet.historique;
    let mut proteges = 1usize;
    if h.len() >= 2 {
        let (a, b) = (h[0].role, h[1].role);
        if (a == Role::Utilisateur && b == Role::Systeme)
            || (a == Role::Systeme && b == Role::Utilisateur)
        {
            proteges = 2;
        }
    }
    // Single drain down to the watermark, always keeping the last message.
    let mut restant = total;
    let mut k = proteges;
    while k + 1 < carnet.historique.len() && restant > cible {
        restant -= carnet.historique[k].cout_chars();
        k += 1;
    }
    // Turn-boundary alignment: never let the kept window START with observations
    // whose emitting assistant (carrying the tool calls) was just dropped - orphan
    // tool_results force the adapter's text fallback and destabilize the prefix.
    while k + 1 < carnet.historique.len() && carnet.historique[k].role == Role::Observation {
        k += 1;
    }
    if k > proteges {
        carnet.historique.drain(proteges..k);
    }
}

/// Assembles the messages sent to the model: system (stable tier) + history + the
/// findings ledger at the TAIL (outbound only, never persisted in the history -
/// and tail position keeps the cached prefix stable while staying maximally fresh
/// in the model's attention).
fn assembler(carnet: &Carnet, reglages: &Reglages, nouveautes: Option<&str>) -> Vec<Message> {
    let mut v = Vec::with_capacity(carnet.historique.len() + 2);
    if !reglages.systeme.is_empty() {
        v.push(Message::systeme(reglages.systeme.clone()));
    }
    v.extend(carnet.historique.iter().cloned());

    // Everything below belongs at the TAIL, where the model reads it last. HOW it
    // travels depends on the backend: a chat template served by llama.cpp or Ollama
    // commonly asserts that the system message comes FIRST, and a trailing one makes
    // the server refuse the whole request ("System message must be at the beginning").
    // So a strict backend receives the same text merged into the last user turn:
    // same position, same recency, no role the template can object to.
    let mut queue: Vec<String> = Vec::new();
    if !carnet.decouvertes.is_empty() {
        let lignes: Vec<String> =
            carnet.decouvertes.iter().map(|d| format!("- {d}")).collect();
        queue.push(format!(
            "## Findings ledger ({} recorded)\nDecisive facts recorded this mission via the \
             `finding` tool. This ledger SURVIVES context compaction - your final synthesis \
             must build on it:\n{}",
            carnet.decouvertes.len(),
            lignes.join("\n")
        ));
    }
    if let Some(vol) = reglages.contexte_volatil.as_deref().filter(|s| !s.trim().is_empty()) {
        queue.push(vol.to_string());
    }
    if let Some(neuf) = nouveautes.filter(|s| !s.trim().is_empty()) {
        queue.push(format!(
            "## Capabilities you created this mission\nThese exist and are CALLABLE right \
             now. They are absent from the tool list and the skill catalog above, which are \
             frozen at mission start: that absence is expected and is NOT a failure. Do not \
             create them a second time. Reach a tool with `tool_call`, a skill with \
             `skill_view`.\n{neuf}"
        ));
    }
    if queue.is_empty() {
        return v;
    }
    let bloc = queue.join("\n\n");

    if reglages.systeme_en_queue_permis {
        v.push(Message::systeme(bloc));
    } else if let Some(dernier) = v.iter_mut().rev().find(|m| m.role != Role::Systeme) {
        // The LAST non-system message, whatever its role. Targeting the last USER
        // turn instead looked natural and was wrong: an agentic context ends on
        // observations, so the block landed mid-conversation and lost exactly the
        // recency it exists for. Appending here introduces no new role for a strict
        // template to reject, and keeps the tail position.
        dernier.contenu = format!("{}\n\n{bloc}", dernier.contenu);
    } else if let Some(premier) = v.first_mut() {
        // Nothing but system messages: better inside the prompt than lost.
        premier.contenu = format!("{}\n\n{bloc}", premier.contenu);
    }
    v
}

/// Why a model call gave up.
enum EchecAppel {
    /// Permanent provider failure (diagnostic message).
    Fatal(String),
    /// The cancellation flag was raised while waiting.
    Interrompu,
}

/// Model call with weather policy (backoff/abandon). Key rotation and model
/// rerouting are handled internally by the `Fournisseur` adapter; the core
/// applies the wait and the abandon. Waits are sliced so a user cancellation
/// (up to 300 s rate-limit sleeps) is honored within a second.
///
/// The call itself is BOUNDED and CANCELLABLE (mirror of the per-tool timeout):
/// a hung stream (stalled backend, half-dead connection with no RST) must never
/// freeze the butinage, and the user's stop must be honored within ~1 s even
/// mid-call. A timeout is classed as a transient error (backoff, then give up).
async fn appeler_modele(
    fournisseur: &dyn Fournisseur,
    messages: &[Message],
    schemas: &[serde_json::Value],
    reglages: &Reglages,
    emet: &dyn Emetteur,
    annulation: Option<&AtomicBool>,
) -> Result<ReponseModele, EchecAppel> {
    let mut tentative = 0usize;
    loop {
        let resultat = {
            let fut = fournisseur.repondre(messages, schemas);
            tokio::pin!(fut);
            let debut = tokio::time::Instant::now();
            loop {
                tokio::select! {
                    r = &mut fut => break r,
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        if est_annule(annulation) {
                            return Err(EchecAppel::Interrompu);
                        }
                        if reglages.timeout_modele_secs > 0
                            && debut.elapsed().as_secs() >= reglages.timeout_modele_secs
                        {
                            break Err(crate::fournisseur::ErreurFournisseur {
                                status: 0, // transport-class: retried with backoff
                                retry_after: None,
                                corps: format!(
                                    "model call timed out after {}s (no response from provider)",
                                    reglages.timeout_modele_secs
                                ),
                            });
                        }
                    }
                }
            }
        };
        match resultat {
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
                        for _ in 0..s {
                            if est_annule(annulation) {
                                return Err(EchecAppel::Interrompu);
                            }
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
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
                        return Err(EchecAppel::Fatal(detail));
                    }
                }
            }
        }
    }
}

/// Converts a model response into an [`Issue`] usable by the compass. Applies
/// the "plan" side effects in passing (updating the itinerary).
fn analyser(reponse: &ReponseModele, carnet: &mut Carnet, profil: ProfilModele) -> Issue {
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
            // Merge (never wholesale replace): a re-emitted plan with statuses reset
            // to pending must not wipe the `done` marks already earned.
            carnet.itineraire.fusionner(etapes);
        } else if let Some(steps) = a.args.get("steps").and_then(|v| v.as_array()) {
            let etapes: Vec<crate::itineraire::Etape> = steps
                .iter()
                .filter_map(|s| s.as_str().map(crate::itineraire::Etape::nouvelle))
                .collect();
            carnet.itineraire.fusionner(etapes);
        }
        // A plan may also declare the mission mode (`"mode": "deep_research"`).
        if let Some(m) = a.args.get("mode").and_then(|v| v.as_str()) {
            escalader_mode(carnet, m);
        }
    }

    // `research_mode`: the model SELF-DECLARES a long-running research once it has read
    // the mission (more reliable than keyword gates on the user prompt). Intercepted -
    // never executed as a tool. One-way escalation; declaring it is a productive act.
    let mut mode_declare = false;
    if let Some(pos) = appels.iter().position(|a| a.nom == "research_mode") {
        let a = appels.remove(pos);
        let mode = a.args.get("mode").and_then(|v| v.as_str()).unwrap_or("deep");
        mode_declare = escalader_mode(carnet, mode);
    }

    // `finding`: records a decisive fact (with source) into the mission LEDGER -
    // intercepted like `plan`, never executed as a tool. The ledger survives
    // compaction/truncation and is re-rendered at the tail of every context.
    let mut finding_trouve = false;
    while let Some(pos) = appels.iter().position(|a| a.nom == "finding") {
        let a = appels.remove(pos);
        let fait = a
            .args
            .get("fact")
            .or_else(|| a.args.get("fait"))
            .or_else(|| a.args.get("finding"))
            .and_then(|v| v.as_str());
        let src = a.args.get("source").and_then(|v| v.as_str());
        if let Some(f) = fait {
            carnet.ajouter_decouverte(f, src);
            finding_trouve = true;
        }
    }

    // Explicit completion tools: `mission_accomplie` (native foraging) or `task_complete`
    // (already registered in LaRuche). Honored ONLY when they are the sole call: when
    // the model emits real work + completion in the same turn (e.g. [file_write,
    // mission_accomplie]), the WORK executes first and the completion is dropped -
    // the model will re-conclude next turn once the work is actually done.
    let est_fin = |a: &Appel| a.nom == "mission_accomplie" || a.nom == "task_complete";
    if appels.len() == 1 && est_fin(&appels[0]) {
        let a = &appels[0];
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
    if appels.len() > 1 {
        appels.retain(|a| !est_fin(a));
    }

    // Same rule for `clarify`: alone it yields control; alongside real work it is
    // dropped (the work answers the question more often than not).
    if appels.len() == 1 && appels[0].nom == "clarify" {
        let q = appels[0]
            .args
            .get("question")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        return Issue::Clarification(q);
    }
    if appels.len() > 1 {
        appels.retain(|a| a.nom != "clarify");
    }

    if !appels.is_empty() {
        return Issue::Outils(appels);
    }

    // Plan, mode or finding posted alone (no other tool): productive act, continue (bounded).
    if plan_trouve || mode_declare || finding_trouve {
        return Issue::PlanEnregistre;
    }

    // Text-format heuristics (broken <tool_call> detection) are rails for weak models:
    // on native-tools profiles they only produce false positives (text that merely
    // DISCUSSES json) - the native stop_reason is authoritative there.
    let heuristiques = profil != ProfilModele::NatifOutils;
    // stop=Outils with zero parsed calls: the provider says the model wanted tools but
    // nothing was extractable - a strong malformed signal, whatever the profile.
    let stop_outils_vide = matches!(reponse.stop, StopReason::Outils);
    let tronquee = matches!(reponse.stop, StopReason::Longueur)
        || (heuristiques && bloc_outil_non_ferme(&reponse.texte));
    Issue::TexteSeul(TexteSeul {
        texte: reponse.texte.clone(),
        fin_native: Some(reponse.stop),
        plan_inacheve: carnet.itineraire.a_des_ouvertes(),
        malforme: stop_outils_vide || (heuristiques && ressemble_a_un_outil(&reponse.texte)),
        tronquee,
        vide: reponse.texte.trim().is_empty(),
    })
}

/// One-way escalation Standard -> Exploration (a model must never downgrade mid-run
/// to escape the exploration rails). Returns `true` if the mode flipped.
fn escalader_mode(carnet: &mut Carnet, mode: &str) -> bool {
    let deep = matches!(
        mode.trim().to_lowercase().as_str(),
        "deep" | "exploration" | "deep_research" | "recherche_longue" | "long"
    );
    if deep && carnet.mode == ModeMission::Standard {
        carnet.mode = ModeMission::Exploration;
        return true;
    }
    false
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

    /// A strict chat template refuses a trailing `system` message.
    ///
    /// llama.cpp and Ollama serve templates that literally
    /// `raise_exception('System message must be at the beginning')`. Putting the
    /// volatile tier at the tail as a system message made the local server refuse
    /// the whole request with HTTP 400, on the very first turn.
    #[test]
    fn le_bloc_volatil_evite_le_role_systeme_en_queue_si_interdit() {
        let mut carnet = Carnet::ouvrir("explique l architecture", ModeMission::Standard, chrono::Utc::now());
        carnet.historique = vec![Message::utilisateur("explique l architecture")];

        let base = Reglages {
            systeme: "SYS".into(),
            systeme_en_queue_permis: true,
            contexte_volatil: Some("## Now
It is Sunday.".into()),
            ..Reglages::default()
        };

        // Permissive backend: the tail message is kept as `system`.
        let permis = assembler(&carnet, &base, None);
        assert_eq!(permis.last().unwrap().role, Role::Systeme);
        assert!(permis.last().unwrap().contenu.contains("It is Sunday"));

        // Strict backend: no system message after the first one, ever.
        let strict = Reglages { systeme_en_queue_permis: false, ..base };
        let out = assembler(&carnet, &strict, None);
        let positions: Vec<usize> = out
            .iter()
            .enumerate()
            .filter(|(_, m)| m.role == Role::Systeme)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(positions, vec![0], "the only system message must be the first");
        // The clock still travels, merged into the last user turn.
        let dernier_user = out.iter().rev().find(|m| m.role == Role::Utilisateur).unwrap();
        assert!(
            dernier_user.contenu.contains("It is Sunday"),
            "the volatile tier must survive: {}",
            dernier_user.contenu
        );
    }
    use super::*;
    use crate::carnet::ModeMission;
    use crate::evenement::Silencieux;
    use crate::fournisseur::ErreurFournisseur;
    use crate::outils::ResultatOutil;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Mutex;

    fn t0() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn tronque_garde_l_ancre_et_les_recents_dans_le_budget() {
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        carnet.historique.clear();
        for i in 0..50 {
            carnet.historique.push(Message::utilisateur("x".repeat(300) + &i.to_string()));
        }
        let avant = carnet.historique.len();
        // tight budget: must drop old ones, keep the anchor AND at least the last
        tronquer_historique(&mut carnet, "systeme", 1000, 3.0);
        assert!(carnet.historique.len() < avant, "old ones dropped");
        assert!(!carnet.historique.is_empty(), "keeps at least the current turn");
        // the MISSION ANCHOR (first message) is pinned
        assert!(carnet.historique.first().unwrap().contenu.ends_with('0'));
        // the LAST message (the most recent) is preserved
        assert!(carnet.historique.last().unwrap().contenu.ends_with("49"));
    }

    #[test]
    fn tronque_ne_laisse_pas_d_observations_orphelines_en_tete() {
        // Turns of [assistant+calls, obs, obs]: the cut must land on a turn start,
        // never leaving observations whose emitting assistant was dropped.
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        carnet.historique.clear();
        carnet.historique.push(Message::utilisateur("MISSION"));
        for _ in 0..20 {
            carnet.historique.push(Message::assistant_avec_appels(
                "je lance l'outil",
                vec![Appel::nouveau("t", json!({}))],
            ));
            carnet.historique.push(Message::observation_liee("t", "id1", "r".repeat(300)));
            carnet.historique.push(Message::observation_liee("t", "id2", "r".repeat(300)));
        }
        tronquer_historique(&mut carnet, "", 1000, 3.0);
        assert_eq!(carnet.historique[0].contenu, "MISSION");
        assert_ne!(
            carnet.historique[1].role,
            Role::Observation,
            "kept window must start on a turn boundary"
        );
    }

    #[test]
    fn tronque_epingle_la_paire_ancre_resume() {
        // Post-compaction shape: [user mission, system resume, ...] -> both pinned.
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        carnet.historique.clear();
        carnet.historique.push(Message::utilisateur("MISSION"));
        carnet.historique.push(Message::systeme("[Compacted context]"));
        for i in 0..50 {
            carnet.historique.push(Message::assistant("y".repeat(300) + &i.to_string()));
        }
        tronquer_historique(&mut carnet, "", 1000, 3.0);
        assert_eq!(carnet.historique[0].contenu, "MISSION");
        assert_eq!(carnet.historique[1].contenu, "[Compacted context]");
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
                usage: None, ..Default::default()
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
        ReponseModele { texte: t.into(), stop: StopReason::FinTour, appels: vec![], usage: None, ..Default::default() }
    }
    fn rep_appel(nom: &str, args: serde_json::Value) -> ReponseModele {
        ReponseModele { texte: String::new(), stop: StopReason::Outils, appels: vec![Appel::nouveau(nom, args)], usage: None, ..Default::default() }
    }
    fn rep_appels(appels: Vec<Appel>) -> ReponseModele {
        ReponseModele { texte: String::new(), stop: StopReason::Outils, appels, usage: None, ..Default::default() }
    }

    #[tokio::test]
    async fn termine_sur_mission_accomplie() {
        let four = FournisseurScript::scenario(vec![rep_appel(
            "mission_accomplie",
            json!({"resume": "tout est fait", "confiance": 0.9}),
        )]);
        let mut carnet = Carnet::ouvrir("fais X", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None, None)
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
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Accomplie);
        assert_eq!(carnet.recolte_web, 1, "the web call must be counted");
        // a tool observation was re-injected into the history
        assert!(carnet
            .historique
            .iter()
            .any(|m| m.outil.as_deref() == Some("web_search")));
        // the assistant message that emitted the call CARRIES it (coherent transcript)
        assert!(carnet
            .historique
            .iter()
            .any(|m| m.appels.iter().any(|a| a.nom == "web_search")));
        assert_eq!(bilan.passes, 2);
    }

    #[tokio::test]
    async fn mission_accomplie_ne_prime_pas_sur_le_travail() {
        // [web_search + mission_accomplie] same turn: the WORK executes, the completion
        // is dropped; the model concludes on the next turn.
        let four = FournisseurScript::scenario(vec![
            rep_appels(vec![
                Appel::nouveau("web_search", json!({"q": "x"})),
                Appel::nouveau("mission_accomplie", json!({"resume": "trop tôt"})),
            ]),
            rep_appel("mission_accomplie", json!({"resume": "fini pour de vrai"})),
        ]);
        let mut carnet = Carnet::ouvrir("cherche", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None, None)
            .await
            .unwrap();
        assert_eq!(carnet.recolte_web, 1, "the web call executed first");
        assert_eq!(bilan.texte, "fini pour de vrai");
        assert_eq!(bilan.passes, 2);
    }

    #[tokio::test]
    async fn texte_seul_standard_termine_tout_de_suite() {
        let four = FournisseurScript::scenario(vec![rep_texte("voici la réponse directe")]);
        let mut carnet = Carnet::ouvrir("salut", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None, None)
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
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Accomplie);
        assert!(carnet.auto_continue >= 1 || carnet.passe >= 2, "auto-continuation must have triggered");
    }

    #[tokio::test]
    async fn reponse_vide_est_relancee_puis_aboutit() {
        // Completely empty response: one bounded relaunch instead of an empty answer.
        let four = FournisseurScript::scenario(vec![rep_texte(""), rep_texte("la vraie réponse")]);
        let mut carnet = Carnet::ouvrir("question", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Accomplie);
        assert_eq!(bilan.texte, "la vraie réponse");
        assert_eq!(bilan.passes, 2);
    }

    /// Provider that never answers: only the model-call timeout brings it back.
    struct FournisseurPendu;
    #[async_trait]
    impl Fournisseur for FournisseurPendu {
        async fn repondre(
            &self,
            _m: &[Message],
            _s: &[serde_json::Value],
        ) -> Result<ReponseModele, ErreurFournisseur> {
            tokio::time::sleep(Duration::from_secs(1_000_000)).await;
            unreachable!("the hung call must be cut by the timeout");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn appel_modele_pendu_est_borne_par_le_timeout() {
        let reglages = Reglages {
            timeout_modele_secs: 2,
            max_transitoire: 1, // 1 retry then give up (keeps the test tight)
            ..Reglages::default()
        };
        let mut carnet = Carnet::ouvrir("x", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &reglages, &FournisseurPendu, &OutilsMock, &Silencieux, None, None, None)
            .await
            .unwrap();
        match bilan.fin {
            FinDeVol::Erreur(motif) => assert!(motif.contains("timed out"), "got: {motif}"),
            autre => panic!("expected Erreur(timeout), got {autre:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn annulation_pendant_l_appel_modele_est_honoree() {
        // The stop is raised WHILE the provider hangs: honored within ~1 s, without
        // waiting for the stream to come back.
        use std::sync::Arc;
        let flag = Arc::new(AtomicBool::new(false));
        let f2 = flag.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(3)).await;
            f2.store(true, Ordering::Relaxed);
        });
        let reglages = Reglages { timeout_modele_secs: 0, ..Reglages::default() }; // unbounded call
        let mut carnet = Carnet::ouvrir("x", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &reglages, &FournisseurPendu, &OutilsMock, &Silencieux, None, None, Some(&flag))
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Interrompue);
    }

    #[tokio::test]
    async fn budget_sans_usage_s_appuie_sur_l_estimation() {
        // The provider never reports usage: the budget must still trigger, fed by
        // the jauge estimate (otherwise the spend ceiling silently never fires).
        let four = FournisseurScript::scenario(vec![
            rep_appel("web_search", json!({"q": "x"})),
            rep_texte("jamais atteint"),
        ]);
        let mut carnet = Carnet::ouvrir("mission", ModeMission::Standard, t0());
        // Big history: the estimate alone (~10k tokens) exceeds the budget.
        carnet.historique.push(Message::utilisateur("mission"));
        carnet.historique.push(Message::assistant("y".repeat(40_000)));
        let reglages = Reglages { budget_tokens: 5_000, ..Reglages::default() };
        let bilan = butiner(&mut carnet, &reglages, &four, &OutilsMock, &Silencieux, None, None, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Budget);
        assert!(carnet.tokens_total() >= 5_000, "estimated spend recorded");
    }

    #[tokio::test]
    async fn self_check_en_exploration_puis_atterrissage() {
        // First mission_accomplie of an exploration run: bounced ONCE for self-check;
        // the second lands. The nudge is internal (not persisted).
        let four = FournisseurScript::scenario(vec![
            rep_appel("web_search", json!({"q": "a"})),
            rep_appel("mission_accomplie", json!({"resume": "v1", "confiance": 0.95})),
            rep_appel("mission_accomplie", json!({"resume": "v2 vérifiée", "confiance": 0.95})),
        ]);
        let mut carnet = Carnet::ouvrir("cherche tout sur X", ModeMission::Exploration, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Accomplie);
        assert_eq!(bilan.texte, "v2 vérifiée");
        assert!(carnet
            .historique
            .iter()
            .any(|m| m.interne && m.contenu.contains("SELF-CHECK")));
    }

    #[tokio::test]
    async fn erreur_fatale_arrete_proprement() {
        let four = FournisseurScript::en_erreur(400); // invalid request: fatal, no fallback
        let mut carnet = Carnet::ouvrir("x", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None, None)
            .await
            .unwrap();
        assert!(matches!(bilan.fin, FinDeVol::Erreur(_)));
    }

    #[tokio::test]
    async fn annulation_rend_la_main_immediatement() {
        let four = FournisseurScript::scenario(vec![rep_texte("jamais atteint")]);
        let mut carnet = Carnet::ouvrir("x", ModeMission::Standard, t0());
        let flag = AtomicBool::new(true);
        let bilan = butiner(
            &mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None,
            Some(&flag),
        )
        .await
        .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Interrompue);
    }

    #[tokio::test]
    async fn winddown_previent_avant_le_plafond() {
        // 5-pass ceiling, model keeps calling tools: at 80% a single wind-down nudge
        // must be injected before the guillotine.
        let reponses: Vec<ReponseModele> =
            (0..6).map(|i| rep_appel("web_search", json!({"q": i}))).collect();
        let four = FournisseurScript::scenario(reponses);
        let mut carnet = Carnet::ouvrir("mission", ModeMission::Standard, t0());
        let reglages = Reglages { plafond_passes: 5, ..Reglages::default() };
        let bilan = butiner(&mut carnet, &reglages, &four, &OutilsMock, &Silencieux, None, None, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Plafond);
        let nudges: Vec<_> = carnet
            .historique
            .iter()
            .filter(|m| m.interne && m.contenu.contains("[Wind-down]"))
            .collect();
        assert_eq!(nudges.len(), 1, "exactly one wind-down warning");
    }

    #[tokio::test]
    async fn budget_epuise_pose_avec_resultats_partiels() {
        // The provider reports usage; the budget cuts before the 2nd pass.
        let four = FournisseurScript::scenario(vec![
            ReponseModele {
                texte: String::new(),
                stop: StopReason::Outils,
                appels: vec![Appel::nouveau("web_search", json!({"q": "x"}))],
                usage: Some(crate::fournisseur::Usage { entree: 900, sortie: 200 }), ..Default::default()
            },
            rep_texte("jamais atteint"),
        ]);
        let mut carnet = Carnet::ouvrir("x", ModeMission::Standard, t0());
        let reglages = Reglages { budget_tokens: 1000, ..Reglages::default() };
        let bilan = butiner(&mut carnet, &reglages, &four, &OutilsMock, &Silencieux, None, None, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Budget);
        assert_eq!(carnet.tokens_total(), 1100);
    }

    #[tokio::test]
    async fn research_mode_escalade_en_exploration_et_injecte_le_protocole() {
        let four = FournisseurScript::scenario(vec![
            rep_appel("research_mode", json!({"mode": "deep", "reason": "large sweep"})),
            rep_appel("web_search", json!({"q": "a"})),
            rep_appel("mission_accomplie", json!({"resume": "ok"})),
        ]);
        let mut carnet = Carnet::ouvrir("cherche tout sur X", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Accomplie);
        assert_eq!(carnet.mode, ModeMission::Exploration, "self-declared escalation");
        // the deep-research protocol was injected as an internal nudge
        assert!(carnet
            .historique
            .iter()
            .any(|m| m.interne && m.contenu.contains("Deep-research protocol")));
        // research_mode was intercepted: never executed as a tool
        assert!(!carnet.historique.iter().any(|m| m.outil.as_deref() == Some("research_mode")));
    }

    #[tokio::test]
    async fn finding_alimente_le_ledger_et_le_contexte() {
        // [finding + web_search] same turn: the fact is recorded, the search executes.
        let four = FournisseurScript::scenario(vec![
            rep_appels(vec![
                Appel::nouveau("finding", json!({"fact": "Le jeu est sorti en 2002", "source": "https://ex.org"})),
                Appel::nouveau("web_search", json!({"q": "suite"})),
            ]),
            rep_appel("mission_accomplie", json!({"resume": "ok"})),
        ]);
        let mut carnet = Carnet::ouvrir("cherche", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None, None, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Accomplie);
        assert_eq!(carnet.decouvertes.len(), 1);
        assert!(carnet.decouvertes[0].contains("2002"));
        assert!(carnet.decouvertes[0].contains("https://ex.org"));
        // finding was intercepted: never executed as a tool
        assert!(!carnet.historique.iter().any(|m| m.outil.as_deref() == Some("finding")));
        // The ledger reaches the model at the TAIL of the outbound context. The
        // transport differs by backend (a trailing `system` message where templates
        // accept one, merged into the last user turn where they do not), so assert
        // the intent on both rather than one particular shape.
        for permis in [true, false] {
            let sortant = assembler(
                &carnet,
                &Reglages { systeme_en_queue_permis: permis, ..Reglages::default() },
                None,
            );
            assert!(
                sortant.last().unwrap().contenu.contains("Findings ledger"),
                "the ledger must close the context (systeme_en_queue_permis={permis})"
            );
        }
    }

    #[test]
    fn les_capacites_forgees_ferment_le_contexte_sur_les_deux_transports() {
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        carnet.historique.push(Message::utilisateur("fabrique un outil"));
        // A capability created mid-mission is callable but absent from the frozen
        // catalogs. It rides in the volatile tail tier, whose transport depends on
        // the backend, so assert the intent on both rather than one shape.
        for permis in [true, false] {
            let sortant = assembler(
                &carnet,
                &Reglages { systeme_en_queue_permis: permis, ..Reglages::default() },
                Some("- tool `meteo_ville`: registered and callable via tool_call"),
            );
            let fin = &sortant.last().unwrap().contenu;
            assert!(
                fin.contains("meteo_ville"),
                "a forged capability must close the context (systeme_en_queue_permis={permis})"
            );
            assert!(
                fin.contains("NOT a failure"),
                "the model must be told the absence from its tool list is expected"
            );
        }
        // Nothing forged: not a single wasted token, and the tail stays untouched.
        let vide = assembler(&carnet, &Reglages::default(), None);
        assert!(!vide.last().unwrap().contenu.contains("Capabilities you created"));
        let blanc = assembler(&carnet, &Reglages::default(), Some("   "));
        assert!(!blanc.last().unwrap().contenu.contains("Capabilities you created"));
    }

    #[test]
    fn ledger_borne_et_deduplique() {
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        carnet.ajouter_decouverte("fait A", Some("src1"));
        carnet.ajouter_decouverte("Fait a", Some("SRC1")); // dup (case-insensitive)
        assert_eq!(carnet.decouvertes.len(), 1);
        for i in 0..50 {
            carnet.ajouter_decouverte(&format!("fait {i}"), None);
        }
        assert_eq!(carnet.decouvertes.len(), 40, "FIFO cap");
        carnet.ajouter_decouverte("", None); // empty: no-op
        assert_eq!(carnet.decouvertes.len(), 40);
    }

    #[test]
    fn escalade_est_a_sens_unique() {
        let mut carnet = Carnet::ouvrir("m", ModeMission::Exploration, t0());
        assert!(!escalader_mode(&mut carnet, "standard"), "no downgrade");
        assert_eq!(carnet.mode, ModeMission::Exploration);
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        assert!(escalader_mode(&mut carnet, "deep_research"));
        assert_eq!(carnet.mode, ModeMission::Exploration);
    }

    #[test]
    fn analyser_gate_les_heuristiques_sur_profil_natif() {
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        let rep = ReponseModele {
            texte: "here is a json with \"name\" and \"arguments\" fields, purely discussed".into(),
            stop: StopReason::FinTour,
            appels: vec![],
            usage: None, ..Default::default()
        };
        // Robuste: text heuristic active -> malformed
        if let Issue::TexteSeul(t) = analyser(&rep, &mut carnet, ProfilModele::Robuste) {
            assert!(t.malforme);
        } else {
            panic!("expected TexteSeul");
        }
        // NatifOutils: heuristic gated -> clean text
        if let Issue::TexteSeul(t) = analyser(&rep, &mut carnet, ProfilModele::NatifOutils) {
            assert!(!t.malforme);
        } else {
            panic!("expected TexteSeul");
        }
    }

    #[test]
    fn stop_outils_sans_appel_est_malforme_meme_en_natif() {
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        let rep = ReponseModele {
            texte: "…".into(),
            stop: StopReason::Outils, // provider says "tools" but nothing was parsed
            appels: vec![],
            usage: None, ..Default::default()
        };
        if let Issue::TexteSeul(t) = analyser(&rep, &mut carnet, ProfilModele::NatifOutils) {
            assert!(t.malforme, "stop=Outils with zero calls is a strong malformed signal");
        } else {
            panic!("expected TexteSeul");
        }
    }
}
