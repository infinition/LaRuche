//! The **Reine**: the swarm's outer supervisor. Where the [`vigie`](super::vigie)
//! guards a loop from the inside (anti-loop, budget), the Reine judges the
//! *result* from the outside: relevance, methodology, and progress toward the
//! objective. She scores a draft, then either ships it, sends a corrective
//! instruction back for another round, or escalates to a human.
//!
//! This module is the **pure decision core**, side-effect free and unit-tested,
//! exactly like the rest of `cap`. The LLM call that actually produces a
//! [`Scorecard`] for a draft lives in the integration layer (`laruche-essaim`);
//! here we only decide what to do with that score, with a hard bound on rounds
//! so the supervisor can never run away on cost or latency.

/// Hard ceiling on revision rounds for FINITE budgets. The Reine stops as soon as
/// a draft passes, so this is only a runaway guard. Bypassed by [`REVUES_ILLIMITEES`].
pub const PLAFOND_REVUES: u8 = 10;

/// Sentinel `max_revues` value meaning "no finite cap": rework until the draft
/// passes (or the user stops the run). Useful for experimenting with a local model
/// at no cost. Practically bounded at 255 rounds, which is effectively unlimited.
pub const REVUES_ILLIMITEES: u8 = u8::MAX;

/// How the Reine operates over a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeReine {
    /// Disabled: drafts ship untouched.
    Off,
    /// Autonomous: the Reine judges and revises on her own.
    Auto,
    /// Autonomous, but escalates to a human when her confidence is below the
    /// configured threshold.
    Hybride,
    /// Human-in-the-seat: every draft is handed to a person to approve, revise,
    /// or reject. The Reine only prepares the scorecard.
    Humaine,
}

impl ModeReine {
    /// Parse the value coming from the UI / config (`off|auto|hybride|humaine`).
    pub fn depuis_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "auto" => ModeReine::Auto,
            "hybride" | "hybrid" => ModeReine::Hybride,
            "humaine" | "human" => ModeReine::Humaine,
            _ => ModeReine::Off,
        }
    }

    /// Stable string used in config / API payloads.
    pub fn comme_str(self) -> &'static str {
        match self {
            ModeReine::Off => "off",
            ModeReine::Auto => "auto",
            ModeReine::Hybride => "hybride",
            ModeReine::Humaine => "humaine",
        }
    }
}

/// Which scope of authority the Reine is exercising on this pass. The three
/// tiers are enabled independently in config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Tier 1: review a chat answer before it reaches the user.
    Reponse,
    /// Tier 2: review a self-created artifact (skill, tool, memory edit, mission).
    Artefact,
    /// Tier 3: proactive orchestration (supervisor loop tasking the swarm).
    Supervision,
}

/// User-facing configuration, mirrored from Settings (one provider/model is
/// chosen separately, like the per-channel model).
#[derive(Debug, Clone)]
pub struct ConfigReine {
    pub mode: ModeReine,
    /// Max revision rounds. `0` disables the Reine; values above [`PLAFOND_REVUES`]
    /// are clamped down.
    pub max_revues: u8,
    /// In [`ModeReine::Hybride`], escalate to a human when the judge's confidence
    /// (0..=100) is strictly below this.
    pub seuil_confiance: u8,
    /// Tier toggles.
    pub tier_reponse: bool,
    pub tier_artefacts: bool,
    pub tier_supervision: bool,
}

impl Default for ConfigReine {
    fn default() -> Self {
        Self {
            mode: ModeReine::Off,
            max_revues: 0,
            seuil_confiance: 60,
            tier_reponse: true,
            tier_artefacts: false,
            tier_supervision: false,
        }
    }
}

impl ConfigReine {
    /// Effective round budget. The unlimited sentinel passes through uncapped;
    /// every finite value is clamped to the runaway ceiling.
    pub fn revues_effectives(&self) -> u8 {
        if self.max_revues == REVUES_ILLIMITEES {
            REVUES_ILLIMITEES
        } else {
            self.max_revues.min(PLAFOND_REVUES)
        }
    }

    /// Is the Reine doing anything at all? Off mode or a zero budget means no.
    pub fn active(&self) -> bool {
        self.mode != ModeReine::Off && self.revues_effectives() > 0
    }

    /// Is the given tier enabled?
    pub fn tier_actif(&self, tier: Tier) -> bool {
        match tier {
            Tier::Reponse => self.tier_reponse,
            Tier::Artefact => self.tier_artefacts,
            Tier::Supervision => self.tier_supervision,
        }
    }
}

/// What the LLM judge proposes for a single draft. Produced in the integration
/// layer; consumed by [`Reine::juger`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Avis {
    /// The draft is good enough to ship.
    Approuver,
    /// The draft needs another round; an instruction accompanies it.
    Reviser,
    /// The judge is unsure and wants a human to decide.
    Escalader,
}

/// The judge's structured assessment of one draft. Scores are 0..=100. This is
/// also the eval signal that feeds the curateur over time.
#[derive(Debug, Clone)]
pub struct Scorecard {
    pub pertinence: u8,
    pub methodologie: u8,
    pub objectif: u8,
    pub conformite_marque: u8,
    /// The judge's own confidence in this assessment (drives Hybride escalation).
    pub confiance: u8,
    pub avis: Avis,
    /// Corrective instruction for the worker when `avis == Reviser`.
    pub instruction: String,
    /// Short human-readable rationale, surfaced in the chat trace.
    pub raison: String,
    /// The judge's reasoning (a sentence or two), shown on demand in the UI.
    pub analyse: String,
}

impl Scorecard {
    /// Mean of the four quality axes (0..=100), the headline quality signal.
    pub fn score_global(&self) -> u8 {
        let somme = self.pertinence as u32
            + self.methodologie as u32
            + self.objectif as u32
            + self.conformite_marque as u32;
        (somme / 4) as u8
    }
}

/// The decision returned to the loop after judging a draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Ship this draft. The string explains why (approved, or budget exhausted).
    Expedier(String),
    /// Run another worker round with this corrective instruction.
    Reviser { tour: u8, instruction: String },
    /// Hand off to a human with this rationale.
    Escalader(String),
}

/// Stateful supervisor for the duration of one request. Counts rounds and keeps
/// the best score seen, so a revision that regresses can be rejected in favor of
/// the previous draft.
#[derive(Debug, Clone)]
pub struct Reine {
    config: ConfigReine,
    tour: u8,
    meilleur_score: Option<u8>,
}

impl Reine {
    pub fn nouvelle(config: ConfigReine) -> Self {
        Self {
            config,
            tour: 0,
            meilleur_score: None,
        }
    }

    pub fn config(&self) -> &ConfigReine {
        &self.config
    }

    /// Reworks already issued.
    pub fn tour_courant(&self) -> u8 {
        self.tour
    }

    /// Judge a draft and decide what happens next. The budget counts **reworks**:
    /// `max_revues` is how many times the worker can be sent back. The judge call
    /// itself is free, so a draft can always be judged before any rework.
    ///
    /// Order of precedence:
    /// 1. inactive config -> ship untouched;
    /// 2. [`ModeReine::Humaine`] -> always escalate (a person takes the seat);
    /// 3. judge approves -> ship;
    /// 4. Hybride and confidence below threshold -> escalate;
    /// 5. judge escalates -> escalate;
    /// 6. rework budget exhausted -> ship the best draft so far;
    /// 7. otherwise -> issue a rework with the judge's instruction.
    pub fn juger(&mut self, carte: &Scorecard) -> Action {
        if !self.config.active() {
            return Action::Expedier("reine inactive".into());
        }

        let score = carte.score_global();
        self.meilleur_score = Some(self.meilleur_score.map_or(score, |m| m.max(score)));

        if self.config.mode == ModeReine::Humaine {
            return Action::Escalader(if carte.raison.is_empty() {
                "human review requested".into()
            } else {
                carte.raison.clone()
            });
        }

        if carte.avis == Avis::Approuver {
            return Action::Expedier(if carte.raison.is_empty() {
                "approved by the reine".into()
            } else {
                carte.raison.clone()
            });
        }

        if self.config.mode == ModeReine::Hybride && carte.confiance < self.config.seuil_confiance {
            return Action::Escalader(format!(
                "low confidence ({} < {}): {}",
                carte.confiance, self.config.seuil_confiance, carte.raison
            ));
        }

        if carte.avis == Avis::Escalader {
            return Action::Escalader(if carte.raison.is_empty() {
                "judge escalated".into()
            } else {
                carte.raison.clone()
            });
        }

        // She wants a rework: allow it only while within the rework budget.
        if self.tour >= self.config.revues_effectives() {
            return Action::Expedier(format!(
                "rework budget reached ({} rework(s)), shipping best draft",
                self.config.revues_effectives()
            ));
        }

        self.tour += 1;
        Action::Reviser {
            tour: self.tour,
            instruction: carte.instruction.clone(),
        }
    }

    /// Anti-regression guard: should we keep the previous draft rather than a new
    /// one that scored lower? The loop calls this before committing a revised draft.
    pub fn regression(&self, nouveau_score: u8) -> bool {
        self.meilleur_score
            .is_some_and(|meilleur| nouveau_score < meilleur)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carte(avis: Avis, scores: [u8; 4], confiance: u8) -> Scorecard {
        Scorecard {
            pertinence: scores[0],
            methodologie: scores[1],
            objectif: scores[2],
            conformite_marque: scores[3],
            confiance,
            avis,
            instruction: "tighten the methodology".into(),
            raison: "needs work".into(),
            analyse: "the methodology is thin".into(),
        }
    }

    fn config(mode: ModeReine, max: u8) -> ConfigReine {
        ConfigReine {
            mode,
            max_revues: max,
            ..Default::default()
        }
    }

    #[test]
    fn off_or_zero_budget_ships_untouched() {
        let mut r = Reine::nouvelle(config(ModeReine::Off, 5));
        assert!(matches!(r.juger(&carte(Avis::Reviser, [10, 10, 10, 10], 90)), Action::Expedier(_)));

        let mut r = Reine::nouvelle(config(ModeReine::Auto, 0));
        assert!(!r.config().active());
        assert!(matches!(r.juger(&carte(Avis::Reviser, [10, 10, 10, 10], 90)), Action::Expedier(_)));
    }

    #[test]
    fn approval_ships() {
        let mut r = Reine::nouvelle(config(ModeReine::Auto, 3));
        assert!(matches!(r.juger(&carte(Avis::Approuver, [90, 90, 90, 90], 95)), Action::Expedier(_)));
    }

    #[test]
    fn reworks_up_to_the_budget() {
        // Budget of 2 = up to 2 reworks; the 3rd revise-wanting verdict ships.
        let mut r = Reine::nouvelle(config(ModeReine::Auto, 2));
        assert!(matches!(
            r.juger(&carte(Avis::Reviser, [40, 40, 40, 40], 90)),
            Action::Reviser { tour: 1, .. }
        ));
        assert!(matches!(
            r.juger(&carte(Avis::Reviser, [50, 50, 50, 50], 90)),
            Action::Reviser { tour: 2, .. }
        ));
        assert!(matches!(
            r.juger(&carte(Avis::Reviser, [55, 55, 55, 55], 90)),
            Action::Expedier(_)
        ));
    }

    #[test]
    fn budget_one_allows_one_rework() {
        let mut r = Reine::nouvelle(config(ModeReine::Auto, 1));
        assert!(matches!(
            r.juger(&carte(Avis::Reviser, [40, 40, 40, 40], 90)),
            Action::Reviser { tour: 1, .. }
        ));
        assert!(matches!(
            r.juger(&carte(Avis::Reviser, [50, 50, 50, 50], 90)),
            Action::Expedier(_)
        ));
    }

    #[test]
    fn budget_is_clamped_to_ceiling() {
        let c = config(ModeReine::Auto, 250);
        assert_eq!(c.revues_effectives(), PLAFOND_REVUES);
    }

    #[test]
    fn unlimited_sentinel_bypasses_the_ceiling() {
        let c = config(ModeReine::Auto, REVUES_ILLIMITEES);
        assert_eq!(c.revues_effectives(), REVUES_ILLIMITEES);
        // It is active and keeps asking for reworks well past the finite ceiling.
        let mut r = Reine::nouvelle(config(ModeReine::Auto, REVUES_ILLIMITEES));
        for i in 1..=15u8 {
            assert!(matches!(
                r.juger(&carte(Avis::Reviser, [40, 40, 40, 40], 90)),
                Action::Reviser { tour, .. } if tour == i
            ));
        }
    }

    #[test]
    fn hybride_escalates_below_confidence() {
        let mut r = Reine::nouvelle(ConfigReine {
            mode: ModeReine::Hybride,
            max_revues: 3,
            seuil_confiance: 70,
            ..Default::default()
        });
        assert!(matches!(
            r.juger(&carte(Avis::Reviser, [40, 40, 40, 40], 50)),
            Action::Escalader(_)
        ));
    }

    #[test]
    fn humaine_always_escalates() {
        let mut r = Reine::nouvelle(config(ModeReine::Humaine, 1));
        assert!(matches!(
            r.juger(&carte(Avis::Approuver, [99, 99, 99, 99], 99)),
            Action::Escalader(_)
        ));
    }

    #[test]
    fn regression_detected_against_best() {
        let mut r = Reine::nouvelle(config(ModeReine::Auto, 5));
        r.juger(&carte(Avis::Reviser, [80, 80, 80, 80], 90)); // best = 80
        assert!(r.regression(70));
        assert!(!r.regression(85));
    }

    #[test]
    fn score_global_is_mean_of_axes() {
        let c = carte(Avis::Approuver, [100, 80, 60, 40], 50);
        assert_eq!(c.score_global(), 70);
    }

    #[test]
    fn mode_round_trips_through_str() {
        for m in [ModeReine::Off, ModeReine::Auto, ModeReine::Hybride, ModeReine::Humaine] {
            assert_eq!(ModeReine::depuis_str(m.comme_str()), m);
        }
    }

    #[test]
    fn tier_toggles_respected() {
        let c = ConfigReine {
            tier_reponse: true,
            tier_artefacts: false,
            tier_supervision: true,
            ..Default::default()
        };
        assert!(c.tier_actif(Tier::Reponse));
        assert!(!c.tier_actif(Tier::Artefact));
        assert!(c.tier_actif(Tier::Supervision));
    }
}
