//! The **boussole**: `cap()`, the single "continue / land / relaunch" decision.
//!
//! Replaces the tangle of string heuristics from the old `brain.rs`. Every
//! decision rests on **facts** ([`Issue`]): native stop_reason, itineraire
//! state, auto-continuation and web-call counters. No content matching, no
//! hardcoded business examples. A **pure** function, fully testable.

use crate::issue::{FinDeVol, Issue};

/// What the loop should do after a pass.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// End the butinage with this reason.
    Poser(FinDeVol),
    /// Relaunch a pass injecting this advice (in English); do not yield control.
    Relancer(String),
    /// Execute these tools then observe.
    Recolter(Vec<crate::issue::Appel>),
    /// Yield control back to the user for a clarification.
    Clarifier(String),
}

/// Decision context: the relevant butinage state at the moment of choice.
#[derive(Debug, Clone)]
pub struct ContexteCap {
    /// Consecutive STERILE relaunches already consumed (reset to 0 as soon as a tool runs).
    pub auto_continue: usize,
    /// HARD bound on sterile relaunches. Small (~3). Beyond it, yield control instead of
    /// looping. Covers only the "weak model" rails (truncation, malformed tool, exploration).
    pub relance_max: usize,
    /// Long research mission: push a bit further before accepting an end.
    pub mode_exploration: bool,
    /// Number of web/network tool calls actually performed.
    pub recolte_web: usize,
    /// In exploration mode, minimum web calls below which we relaunch (bounded by `relance_max`).
    pub min_web_exploration: usize,
    /// Is `delegate` available in THIS context? False for sub-agents (anti-recursion):
    /// the exploration nudge must not order a fan-out they cannot perform.
    pub delegation_dispo: bool,
    /// Is the pre-landing SELF-CHECK still available? The loop arms it once per
    /// butinage (and disarms it for sub-agents, whose parent cross-checks anyway):
    /// the first `mission_accomplie` of an exploration run — or a low-confidence one
    /// anywhere — gets ONE bounded verification bounce instead of landing unchecked.
    pub verification_dispo: bool,
}

impl ContexteCap {
    fn relance_dispo(&self) -> bool {
        self.auto_continue < self.relance_max
    }
}

/// The deep-research protocol, injected when a mission enters exploration mode
/// (keyword gate at mission start, or mid-run via the model's own `research_mode`
/// declaration). Also appended to the system prompt by the integration layer.
/// English: better instruction following, cacheable prefix.
pub const PROTOCOLE_EXPLORATION: &str = "\
## Deep-research protocol (exploration mode)\n\
This is a LONG-RUNNING research mission. You are autonomous: NEVER hand the search back \
to the user, NEVER ask permission to continue, NEVER conclude after a single query. Even \
if you already recall an answer from memory, a deep-research request requires FRESH \
verification: search anyway, do not answer from memory alone.\n\
1. DECOMPOSE the question into 3 to 4 INDEPENDENT angles MAX (source types, communities, \
time periods, adjacent topics). More angles is NOT better: quality over quantity.\n\
2. FAN OUT: dispatch ONE `delegate` scout PER angle - AT MOST 4 scouts total for the whole \
mission - emit the <tool_call> blocks in the SAME message (role: \"eclaireuse\", each with a \
precise, self-contained brief). They run IN PARALLEL on isolated contexts and each returns a \
compact report. Do NOT dispatch more scouts once you have their reports: synthesize.\n\
3. CROSS-CHECK decisive or conflicting findings with a `delegate` role \"gardienne\".\n\
4. ITERATE: every report opens new angles (names, forums, archives, mirrors). Search \
queries must be SHORT and FOCUSED (2-5 terms; never keyword soup). A blocked page \
(403/paywall/captcha) is an obstacle, NOT a dead end: retry via web archives \
(web.archive.org), search-engine caches, alternate mirrors and sources.\n\
5. RECORD every decisive fact the moment you learn it: call `finding` with the fact and \
its source URL. The findings ledger survives context compaction - anything NOT recorded \
may be lost before the synthesis.\n\
6. SYNTHESIZE (yourself, or a `delegate` role \"architecte\"): a structured answer with \
concrete findings and source URLs, built on the findings ledger.\n\
Only conclude (mission_accomplie) when NEW angles stop yielding NEW information.";

/// Injected advice (English: better instruction following, cacheable prefix).
mod nudge {
    pub const REPRISE_TRONQUEE: &str = "Your previous message was cut off mid-output. Continue exactly \
        where it stopped - do not repeat what you already wrote; finish the sentence or the tool-call block.";
    pub const REFORMER_OUTIL: &str = "You emitted something that looks like a tool call but is not valid \
        and executable. Re-emit ONLY one valid tool call now - no markdown, no prose.";
    pub const REPONSE_VIDE: &str = "Your previous response was completely empty (no text, no tool call). \
        Respond now: either call the tool you need, or write your answer to the user.";
    pub const VERIFIER_AVANT_DE_POSER: &str = "Before landing: SELF-CHECK your conclusion. Re-read it \
        critically. Every decisive claim must be backed by a source or observation from THIS run - if \
        any is unverified or shaky, verify it NOW with your tools (or a `delegate` role \"gardienne\" \
        cross-check if available). Then re-emit mission_accomplie: with the corrected conclusion, or \
        the same one if it fully holds.";
    pub const DEMARRER_PLAN: &str = "Plan recorded. Now EXECUTE the first step by calling the needed \
        tool - do not just restate the plan or ask for confirmation.";
    pub const EXPLORER_PLUS: &str = "This is a long-running research mission and you have not searched \
        enough yet. Do not conclude, and do NOT hand the search back to the user. Open NEW angles NOW: \
        either dispatch several parallel `delegate` scouts (role: eclaireuse, one per angle, in the same \
        message), or call web tools directly. Queries must be SHORT and FOCUSED (2-5 terms; never keyword \
        soup) - vary them (synonyms, EN/FR), try archives/forums/source sites and advanced operators. \
        A 403/paywall is not a dead end: use web archives, caches, mirrors.";
    /// Variant for contexts WITHOUT delegation (sub-agents): direct tools only.
    pub const EXPLORER_PLUS_SOLO: &str = "This is a long-running research mission and you have not searched \
        enough yet. Do not conclude, and do NOT hand the search back to the user. Open NEW angles NOW with \
        your direct web tools (web_deep_search, web_fetch). Queries must be SHORT and FOCUSED (2-5 terms; \
        never keyword soup) - vary them (synonyms, EN/FR), try archives/forums/source sites and advanced \
        operators. A 403/paywall is not a dead end: use web archives, caches, mirrors.";
}

/// Below this self-declared confidence, a `mission_accomplie` gets a self-check
/// bounce even outside exploration mode (when the check is still available).
const SEUIL_CONFIANCE_VERIF: f32 = 0.6;

/// **The** continuation decision. Pure: `(contexte, issue) -> Decision`.
pub fn cap(ctx: &ContexteCap, issue: Issue) -> Decision {
    match issue {
        // Explicit termination: trust the dedicated tool — after ONE bounded
        // verification bounce when the check is armed and warranted (exploration
        // run, or a completion the model itself does not trust). The loop disarms
        // `verification_dispo` after the bounce: no verification loop possible.
        Issue::MissionAccomplie { confiance, .. } => {
            let meritant = ctx.mode_exploration || confiance < SEUIL_CONFIANCE_VERIF;
            if ctx.verification_dispo && meritant && ctx.relance_dispo() {
                return Decision::Relancer(nudge::VERIFIER_AVANT_DE_POSER.to_string());
            }
            Decision::Poser(FinDeVol::Accomplie)
        }
        Issue::Clarification(q) => Decision::Clarifier(q),
        Issue::Outils(appels) => Decision::Recolter(appels),

        // Plan posted alone: relaunch (bounded) so it moves to action, like a tool
        // call chaining on. If it rambles instead of acting, relance_max cuts it off.
        Issue::PlanEnregistre => {
            if ctx.relance_dispo() {
                Decision::Relancer(nudge::DEMARRER_PLAN.to_string())
            } else {
                Decision::Poser(FinDeVol::Accomplie)
            }
        }

        // Text only. STATE OF THE ART (third-party/third-party/Claude Code): a response with no
        // tool call = END OF TURN, yield control. A multi-step task continues *because
        // there are tool calls*, never because of an "unfinished plan".
        // Only a few BOUNDED rails (weak models) may relaunch.
        Issue::TexteSeul(t) => {
            // Rail 1: truncated output (finish_reason=length), exact resume (standard).
            if t.tronquee && ctx.relance_dispo() {
                return Decision::Relancer(nudge::REPRISE_TRONQUEE.to_string());
            }
            // Rail 2: malformed tool call, re-emit. Weak model that botched the syntax.
            if t.malforme && ctx.relance_dispo() {
                return Decision::Relancer(nudge::REFORMER_OUTIL.to_string());
            }
            // Rail 2b: completely empty response (no text, no call): abnormal on any
            // profile (prompt truncation, content filter, backend hiccup). One bounded
            // relaunch beats handing the user an empty answer. After malforme: an empty
            // stop=Outils turn is a tool problem first.
            if t.vide && ctx.relance_dispo() {
                return Decision::Relancer(nudge::REPONSE_VIDE.to_string());
            }
            // Rail 3: long search not thorough enough, push a bit. BOUNDED by
            //   relance_max over STERILE turns (auto_continue resets to 0 as soon as a
            //   tool runs), so as long as the model really searches it continues; once
            //   it rambles (repeated text-only) we cut it off.
            if ctx.mode_exploration
                && ctx.recolte_web < ctx.min_web_exploration
                && ctx.relance_dispo()
            {
                // Never order a fan-out where delegation is unavailable (sub-agents):
                // the model would burn passes against the anti-recursion wall.
                let n = if ctx.delegation_dispo {
                    nudge::EXPLORER_PLUS
                } else {
                    nudge::EXPLORER_PLUS_SOLO
                };
                return Decision::Relancer(n.to_string());
            }
            // Otherwise: end of turn (yield control back to the user).
            Decision::Poser(FinDeVol::Accomplie)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::{Appel, StopReason, TexteSeul};
    use serde_json::json;

    fn ctx() -> ContexteCap {
        ContexteCap {
            auto_continue: 0,
            relance_max: 3,
            mode_exploration: false,
            recolte_web: 0,
            min_web_exploration: 12,
            delegation_dispo: true,
            verification_dispo: false,
        }
    }

    #[test]
    fn self_check_borne_avant_mission_accomplie() {
        let fin = |confiance: f32| Issue::MissionAccomplie { resume: "fini".into(), confiance };
        // Exploration + check armed: one verification bounce.
        let mut c = ctx();
        c.mode_exploration = true;
        c.verification_dispo = true;
        match cap(&c, fin(0.95)) {
            Decision::Relancer(n) => assert!(n.contains("SELF-CHECK")),
            autre => panic!("expected Relancer, got {autre:?}"),
        }
        // Check consumed (loop disarms it): the completion lands.
        c.verification_dispo = false;
        assert_eq!(cap(&c, fin(0.95)), Decision::Poser(FinDeVol::Accomplie));
        // Standard mode: only a LOW-confidence completion gets the bounce.
        let mut c = ctx();
        c.verification_dispo = true;
        assert_eq!(cap(&c, fin(0.9)), Decision::Poser(FinDeVol::Accomplie));
        assert!(matches!(cap(&c, fin(0.3)), Decision::Relancer(_)));
        // Sterile-relaunch budget exhausted: never bounce, land.
        c.auto_continue = 3;
        assert_eq!(cap(&c, fin(0.3)), Decision::Poser(FinDeVol::Accomplie));
    }

    #[test]
    fn exploration_sans_delegation_nudge_les_outils_directs() {
        let mut c = ctx();
        c.mode_exploration = true;
        c.recolte_web = 2;
        c.delegation_dispo = false; // sub-agent context
        match cap(&c, texte(base_texte())) {
            Decision::Relancer(n) => {
                assert!(n.contains("direct web tools"));
                assert!(!n.contains("delegate"), "must not order an impossible fan-out");
            }
            autre => panic!("expected Relancer, got {autre:?}"),
        }
    }

    fn texte(t: TexteSeul) -> Issue {
        Issue::TexteSeul(t)
    }

    fn base_texte() -> TexteSeul {
        TexteSeul {
            texte: "voici la réponse".into(),
            fin_native: Some(StopReason::FinTour),
            plan_inacheve: false,
            malforme: false,
            tronquee: false,
            vide: false,
        }
    }

    #[test]
    fn reponse_vide_relance_puis_rend_la_main() {
        let mut t = base_texte();
        t.texte = String::new();
        t.vide = true;
        match cap(&ctx(), texte(t.clone())) {
            Decision::Relancer(n) => assert!(n.contains("empty")),
            autre => panic!("expected Relancer, got {autre:?}"),
        }
        // bounded: sterile relaunches exhausted -> end of turn even if empty
        let mut c = ctx();
        c.auto_continue = 3;
        assert_eq!(cap(&c, texte(t)), Decision::Poser(FinDeVol::Accomplie));
    }

    #[test]
    fn mission_accomplie_pose() {
        let d = cap(&ctx(), Issue::MissionAccomplie { resume: "fini".into(), confiance: 0.9 });
        assert_eq!(d, Decision::Poser(FinDeVol::Accomplie));
    }

    #[test]
    fn clarify_rend_la_main() {
        let d = cap(&ctx(), Issue::Clarification("quelle ville ?".into()));
        assert_eq!(d, Decision::Clarifier("quelle ville ?".into()));
    }

    #[test]
    fn outils_declenchent_recolte() {
        let appels = vec![Appel::nouveau("web", json!({"q": "x"}))];
        let d = cap(&ctx(), Issue::Outils(appels.clone()));
        assert_eq!(d, Decision::Recolter(appels));
    }

    #[test]
    fn texte_simple_pose_la_fin() {
        let d = cap(&ctx(), texte(base_texte()));
        assert_eq!(d, Decision::Poser(FinDeVol::Accomplie));
    }

    #[test]
    fn troncature_relance() {
        let mut t = base_texte();
        t.tronquee = true;
        match cap(&ctx(), texte(t)) {
            Decision::Relancer(n) => assert!(n.contains("cut off")),
            autre => panic!("expected Relancer, got {autre:?}"),
        }
    }

    #[test]
    fn malforme_relance() {
        let mut t = base_texte();
        t.malforme = true;
        assert!(matches!(cap(&ctx(), texte(t)), Decision::Relancer(_)));
    }

    #[test]
    fn plan_inacheve_ne_force_plus_la_continuation() {
        // State of the art: an unfinished plan does NOT relaunch (anti-rambling). Text only = end.
        let mut t = base_texte();
        t.plan_inacheve = true;
        assert_eq!(cap(&ctx(), texte(t)), Decision::Poser(FinDeVol::Accomplie));
    }

    #[test]
    fn exploration_refuse_fin_precoce() {
        let mut c = ctx();
        c.mode_exploration = true;
        c.recolte_web = 3; // < min_web_exploration (12)
        match cap(&c, texte(base_texte())) {
            Decision::Relancer(n) => assert!(n.contains("research mission")),
            autre => panic!("expected Relancer, got {autre:?}"),
        }
    }

    #[test]
    fn exploration_accepte_fin_apres_effort() {
        let mut c = ctx();
        c.mode_exploration = true;
        c.recolte_web = 15; // >= min
        assert_eq!(cap(&c, texte(base_texte())), Decision::Poser(FinDeVol::Accomplie));
    }

    #[test]
    fn relances_bornees_par_relance_max() {
        // The rails (truncation/exploration) stop after relance_max sterile relaunches.
        let mut c = ctx(); // relance_max = 3
        let mut t = base_texte();
        t.tronquee = true;
        c.auto_continue = 2; // < 3, one more resume
        assert!(matches!(cap(&c, texte(t.clone())), Decision::Relancer(_)));
        c.auto_continue = 3; // == relance_max, yield control
        assert_eq!(cap(&c, texte(t)), Decision::Poser(FinDeVol::Accomplie));
    }

    #[test]
    fn exploration_sterile_finit_par_rendre_la_main() {
        // Model rambling in text only without searching, cut off after relance_max.
        let mut c = ctx();
        c.mode_exploration = true;
        c.recolte_web = 0;
        c.auto_continue = 3; // relances stériles épuisées
        assert_eq!(cap(&c, texte(base_texte())), Decision::Poser(FinDeVol::Accomplie));
    }
}
