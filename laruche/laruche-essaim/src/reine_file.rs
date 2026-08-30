//! The proposals queue: the "pull request" backlog the Reine drains. When the
//! Reine gates memory (and other self-modifications), agent-proposed changes land
//! here as [`Proposition`]s instead of being applied directly, and an approver
//! (the autonomous Reine, or a human) decides their fate.
//!
//! This module is the **pure core**: the proposal type, its status lifecycle,
//! risk classification, the mode-aware disposition policy, and staleness
//! detection. Two invariants are encoded and tested here:
//!
//! 1. **The queue outlives the Reine toggle.** Disabling the Reine never
//!    discards or auto-applies pending proposals (no silent data loss); it only
//!    stops gating *new* writes. See [`transition_desactivation`].
//! 2. **Destructive changes never auto-apply.** Deletes and overwrites are always
//!    escalated to a human, whatever the mode. See [`disposition`].
//!
//! Persistence and the `memoire.db` write path are wired in the integration
//! layer. Timestamps are passed in rather than read from the clock, so the core
//! stays deterministic and testable.

use laruche_butinage::cap::reine::ModeReine;
use serde::{Deserialize, Serialize};

/// What a proposal would do if applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeProposition {
    /// Add a new memory record.
    MemoireAjout,
    /// Update an existing memory record.
    MemoireMaj,
    /// Delete a memory record.
    MemoireSuppr,
    /// Register a newly self-created skill.
    SkillNouveau,
    /// Register a newly self-created tool.
    ToolNouveau,
    /// A self-created mission.
    Mission,
    /// Memory hygiene suggested by the dream pass (dedup of exact duplicates in
    /// a node). Applying it soft-deletes redundant copies, so it is critical and
    /// always requires a human click.
    MemoireHygiene,
}

/// How risky applying a proposal is. Drives whether it can ever auto-apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Risque {
    /// New, non-colliding information. Auto-approvable in autonomous modes.
    Sur,
    /// Changes existing state without destroying it. Auto-approvable with
    /// sufficient confidence.
    Sensible,
    /// Destroys or overwrites an existing record. Never auto-applied.
    Critique,
}

/// Lifecycle of a proposal. `EnAttente` is the only mutable-by-the-Reine state;
/// the others are terminal except `Obsolete`, which requires a re-review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Statut {
    /// Waiting in the backlog.
    EnAttente,
    /// Accepted and applied.
    Approuve,
    /// Declined; kept for audit and as a learning signal for the curateur.
    Rejete,
    /// Aged out without review.
    Perime,
    /// The target changed since the proposal was made; needs a rebase/re-review
    /// before it can be applied (the "PR has conflicts" case).
    Obsolete,
}

impl Statut {
    /// Can the approver still act on this proposal?
    pub fn actionnable(self) -> bool {
        matches!(self, Statut::EnAttente | Statut::Obsolete)
    }
}

/// A single queued change. `base_version` is the version of the target record at
/// proposal time, used to detect staleness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposition {
    pub id: String,
    pub type_: TypeProposition,
    /// Target key for memory update/delete (None for pure additions/artifacts).
    pub cible: Option<String>,
    /// Version of the target when the proposal was made (for staleness checks).
    pub base_version: Option<u64>,
    /// Proposed value, diff, or artifact body.
    pub contenu: String,
    /// Who proposed it (agent, mission, turn id).
    pub provenance: String,
    /// Why, in one line.
    pub raison: String,
    /// Does this overwrite or contradict an existing record?
    pub ecrase_existant: bool,
    pub statut: Statut,
    /// Creation time (unix seconds), supplied by the caller.
    pub cree_a: i64,
}

impl Proposition {
    /// Risk class of this proposal.
    pub fn risque(&self) -> Risque {
        classifier_risque(self.type_, self.ecrase_existant)
    }

    /// Has the proposal aged past `ttl_secondes` without review? `maintenant` is
    /// the current unix time, supplied by the caller.
    pub fn perime(&self, maintenant: i64, ttl_secondes: i64) -> bool {
        self.statut == Statut::EnAttente && maintenant - self.cree_a >= ttl_secondes
    }

    /// Is the proposal stale against the target's current version? A memory
    /// update/delete whose base no longer matches must be re-reviewed.
    pub fn obsolete(&self, version_actuelle_cible: Option<u64>) -> bool {
        match (self.base_version, version_actuelle_cible) {
            (Some(base), Some(actuelle)) => base != actuelle,
            _ => false,
        }
    }
}

/// Classify the risk of a proposal type given whether it overwrites/contradicts
/// an existing record.
pub fn classifier_risque(type_: TypeProposition, ecrase_existant: bool) -> Risque {
    match type_ {
        TypeProposition::MemoireSuppr | TypeProposition::MemoireHygiene => Risque::Critique,
        TypeProposition::MemoireMaj if ecrase_existant => Risque::Critique,
        TypeProposition::MemoireAjout if ecrase_existant => Risque::Critique,
        TypeProposition::MemoireMaj => Risque::Sensible,
        TypeProposition::SkillNouveau | TypeProposition::ToolNouveau | TypeProposition::Mission => {
            Risque::Sensible
        }
        TypeProposition::MemoireAjout => Risque::Sur,
    }
}

/// What to do with a freshly proposed change when the Reine gate is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Apply immediately (still recorded in the queue as Approuve for audit).
    AutoApprouver,
    /// Park in the backlog for review.
    MettreEnFile,
    /// Hand to a human to decide.
    EscaladerHumain,
}

/// Decide the disposition of a proposal, given the Reine mode, the proposal's
/// risk, and the judge's confidence vs the escalation threshold.
///
/// Invariant: [`Risque::Critique`] is never auto-applied, in any mode. In
/// [`ModeReine::Humaine`] everything is parked for human review. In autonomous
/// modes, safe and confident-enough sensible changes auto-apply; uncertain
/// sensible changes are parked (Auto) or escalated (Hybride).
pub fn disposition(mode: ModeReine, risque: Risque, confiance: u8, seuil: u8) -> Disposition {
    if risque == Risque::Critique {
        return Disposition::EscaladerHumain;
    }
    match mode {
        ModeReine::Off => Disposition::AutoApprouver, // gate inactive: caller bypasses anyway
        ModeReine::Humaine => Disposition::MettreEnFile,
        ModeReine::Auto | ModeReine::Hybride => match risque {
            Risque::Sur => Disposition::AutoApprouver,
            Risque::Sensible if confiance >= seuil => Disposition::AutoApprouver,
            // Uncertain sensible change: Hybride asks a human, Auto just parks it.
            Risque::Sensible if mode == ModeReine::Hybride => Disposition::EscaladerHumain,
            _ => Disposition::MettreEnFile,
        },
    }
}

/// Status of a pending proposal after the Reine is **disabled**. This is the
/// decoupling guarantee: disabling never discards or applies the backlog, so a
/// pending proposal stays pending. Identity for every status.
pub fn transition_desactivation(statut: Statut) -> Statut {
    statut
}

/// Is an actionable proposal of this type already queued for this target?
/// Guards recurring producers (the 6h dream pass) against flooding the backlog
/// with the same suggestion on every run. Terminal statuses do not block a
/// re-proposal: a rejected or expired suggestion may legitimately come back.
pub fn deja_en_file(props: &[Proposition], type_: TypeProposition, cible: &str) -> bool {
    props
        .iter()
        .any(|p| p.type_ == type_ && p.statut.actionnable() && p.cible.as_deref() == Some(cible))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop(type_: TypeProposition, ecrase: bool) -> Proposition {
        Proposition {
            id: "p1".into(),
            type_,
            cible: Some("user.role".into()),
            base_version: Some(3),
            contenu: "value".into(),
            provenance: "curateur".into(),
            raison: "learned from turn".into(),
            ecrase_existant: ecrase,
            statut: Statut::EnAttente,
            cree_a: 1000,
        }
    }

    #[test]
    fn deletes_and_overwrites_are_critical() {
        assert_eq!(
            classifier_risque(TypeProposition::MemoireSuppr, false),
            Risque::Critique
        );
        assert_eq!(
            classifier_risque(TypeProposition::MemoireMaj, true),
            Risque::Critique
        );
        assert_eq!(
            classifier_risque(TypeProposition::MemoireAjout, true),
            Risque::Critique
        );
        // Dream hygiene soft-deletes redundant copies: never auto-applied.
        assert_eq!(
            classifier_risque(TypeProposition::MemoireHygiene, false),
            Risque::Critique
        );
    }

    #[test]
    fn recurring_suggestions_do_not_flood_the_queue() {
        let mut hygiene = prop(TypeProposition::MemoireHygiene, false);
        hygiene.cible = Some("projets.laruche".into());
        let props = vec![hygiene.clone()];
        // Same target, still actionable: blocked.
        assert!(deja_en_file(
            &props,
            TypeProposition::MemoireHygiene,
            "projets.laruche"
        ));
        // Different target or type: allowed.
        assert!(!deja_en_file(
            &props,
            TypeProposition::MemoireHygiene,
            "projets.autre"
        ));
        assert!(!deja_en_file(
            &props,
            TypeProposition::MemoireSuppr,
            "projets.laruche"
        ));
        // Terminal status: a fresh identical suggestion may come back.
        hygiene.statut = Statut::Rejete;
        assert!(!deja_en_file(
            &[hygiene],
            TypeProposition::MemoireHygiene,
            "projets.laruche"
        ));
    }

    #[test]
    fn fresh_add_is_safe_plain_update_is_sensitive() {
        assert_eq!(
            classifier_risque(TypeProposition::MemoireAjout, false),
            Risque::Sur
        );
        assert_eq!(
            classifier_risque(TypeProposition::MemoireMaj, false),
            Risque::Sensible
        );
    }

    #[test]
    fn critical_never_auto_applies_in_any_mode() {
        for mode in [ModeReine::Auto, ModeReine::Hybride, ModeReine::Humaine] {
            assert_eq!(
                disposition(mode, Risque::Critique, 100, 60),
                Disposition::EscaladerHumain
            );
        }
    }

    #[test]
    fn auto_mode_auto_approves_safe_and_confident_sensitive() {
        assert_eq!(
            disposition(ModeReine::Auto, Risque::Sur, 0, 60),
            Disposition::AutoApprouver
        );
        assert_eq!(
            disposition(ModeReine::Auto, Risque::Sensible, 80, 60),
            Disposition::AutoApprouver
        );
        // Uncertain sensible change is parked, not applied.
        assert_eq!(
            disposition(ModeReine::Auto, Risque::Sensible, 50, 60),
            Disposition::MettreEnFile
        );
    }

    #[test]
    fn hybride_escalates_uncertain_sensitive() {
        assert_eq!(
            disposition(ModeReine::Hybride, Risque::Sensible, 50, 60),
            Disposition::EscaladerHumain
        );
    }

    #[test]
    fn humaine_parks_everything_non_critical() {
        assert_eq!(
            disposition(ModeReine::Humaine, Risque::Sur, 100, 60),
            Disposition::MettreEnFile
        );
        assert_eq!(
            disposition(ModeReine::Humaine, Risque::Sensible, 100, 60),
            Disposition::MettreEnFile
        );
    }

    #[test]
    fn disabling_the_reine_preserves_the_backlog() {
        // The decoupling guarantee: a pending proposal stays pending, never
        // discarded or auto-applied, when the Reine is turned off.
        assert_eq!(
            transition_desactivation(Statut::EnAttente),
            Statut::EnAttente
        );
        assert_eq!(transition_desactivation(Statut::Obsolete), Statut::Obsolete);
    }

    #[test]
    fn staleness_detected_when_target_moved() {
        let p = prop(TypeProposition::MemoireMaj, false); // base_version = 3
        assert!(p.obsolete(Some(4)));
        assert!(!p.obsolete(Some(3)));
        assert!(!p.obsolete(None));
    }

    #[test]
    fn expiry_only_after_ttl_and_only_while_pending() {
        let p = prop(TypeProposition::MemoireAjout, false); // cree_a = 1000
        assert!(!p.perime(1500, 1000)); // 500 < 1000 ttl
        assert!(p.perime(2000, 1000)); // 1000 >= 1000 ttl
        let mut applique = p.clone();
        applique.statut = Statut::Approuve;
        assert!(!applique.perime(9999, 1000)); // terminal status never expires
    }

    #[test]
    fn obsolete_proposals_remain_actionable() {
        assert!(Statut::Obsolete.actionnable());
        assert!(Statut::EnAttente.actionnable());
        assert!(!Statut::Approuve.actionnable());
        assert!(!Statut::Rejete.actionnable());
    }
}
