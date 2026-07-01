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
to the user, NEVER ask permission to continue, NEVER conclude after a single query.\n\
1. DECOMPOSE the question into 3-5 INDEPENDENT angles (source types, communities, \
languages EN/FR, time periods, adjacent topics).\n\
2. FAN OUT: dispatch one `delegate` scout PER angle - emit SEVERAL <tool_call> blocks in \
the SAME message (role: \"eclaireuse\", each with a precise, self-contained brief). They \
run IN PARALLEL on isolated contexts and each returns a compact report.\n\
3. CROSS-CHECK decisive or conflicting findings with a `delegate` role \"gardienne\".\n\
4. ITERATE: every report opens new angles (names, forums, archives, mirrors). A blocked \
page (403/paywall/captcha) is an obstacle, NOT a dead end: retry via web archives \
(web.archive.org), search-engine caches, alternate mirrors and sources.\n\
5. SYNTHESIZE (yourself, or a `delegate` role \"architecte\"): a structured answer with \
concrete findings and source URLs.\n\
Only conclude (mission_accomplie) when NEW angles stop yielding NEW information.";

/// Injected advice (English: better instruction following, cacheable prefix).
mod nudge {
    pub const REPRISE_TRONQUEE: &str = "Your previous message was cut off mid-output. Continue exactly \
        where it stopped - do not repeat what you already wrote; finish the sentence or the tool-call block.";
    pub const REFORMER_OUTIL: &str = "You emitted something that looks like a tool call but is not valid \
        and executable. Re-emit ONLY one valid tool call now - no markdown, no prose.";
    pub const DEMARRER_PLAN: &str = "Plan recorded. Now EXECUTE the first step by calling the needed \
        tool - do not just restate the plan or ask for confirmation.";
    pub const EXPLORER_PLUS: &str = "This is a long-running research mission and you have not searched \
        enough yet. Do not conclude, and do NOT hand the search back to the user. Open NEW angles NOW: \
        either dispatch several parallel `delegate` scouts (role: eclaireuse, one per angle, in the same \
        message), or call web tools directly - vary queries (synonyms, EN/FR), try archives/forums/source \
        sites and advanced operators. A 403/paywall is not a dead end: use web archives, caches, mirrors.";
}

/// **The** continuation decision. Pure: `(contexte, issue) -> Decision`.
pub fn cap(ctx: &ContexteCap, issue: Issue) -> Decision {
    match issue {
        // Explicit terminations: trust the dedicated tools. The `mission_accomplie`
        // summary is carried by the caller (which already has the Issue), so here we
        // just decide the end.
        Issue::MissionAccomplie { .. } => Decision::Poser(FinDeVol::Accomplie),
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
            // Rail 3: long search not thorough enough, push a bit. BOUNDED by
            //   relance_max over STERILE turns (auto_continue resets to 0 as soon as a
            //   tool runs), so as long as the model really searches it continues; once
            //   it rambles (repeated text-only) we cut it off.
            if ctx.mode_exploration
                && ctx.recolte_web < ctx.min_web_exploration
                && ctx.relance_dispo()
            {
                return Decision::Relancer(nudge::EXPLORER_PLUS.to_string());
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
        }
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
