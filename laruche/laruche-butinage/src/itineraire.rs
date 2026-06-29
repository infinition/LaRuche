//! The butinage itineraire = the structured plan/todo list.
//!
//! This is the **source of truth for termination**: we never decide a mission is
//! finished by reading the model's text ("I'm done"), but by observing that every
//! step of the itineraire is terminated or not applicable.

use serde::{Deserialize, Serialize};

/// Status of an itineraire step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatutEtape {
    /// Not yet started.
    AFaire,
    /// Currently being processed.
    EnCours,
    /// Completed successfully.
    Terminee,
    /// Blocked (dependency, external error): does not prevent global termination.
    Bloquee,
    /// No longer relevant (e.g. a conditional search that yielded nothing).
    NonApplicable,
}

impl StatutEtape {
    /// A "closed" step no longer requires work (terminated, abandoned, or durably
    /// blocked). Used to know whether the itineraire is globally accomplished.
    pub fn est_close(self) -> bool {
        matches!(
            self,
            StatutEtape::Terminee | StatutEtape::NonApplicable | StatutEtape::Bloquee
        )
    }

    /// An "open" step still requires an action from the tool.
    pub fn est_ouverte(self) -> bool {
        matches!(self, StatutEtape::AFaire | StatutEtape::EnCours)
    }

    /// Maps a textual status (model FR/EN formats) to the typed status.
    pub fn depuis(s: &str) -> StatutEtape {
        match s.trim().to_lowercase().as_str() {
            "done" | "terminee" | "terminée" | "completed" | "ok" => StatutEtape::Terminee,
            "in_progress" | "en_cours" | "encours" | "doing" | "wip" => StatutEtape::EnCours,
            "blocked" | "bloquee" | "bloquée" | "blocked_by" => StatutEtape::Bloquee,
            "skip" | "skipped" | "non_applicable" | "n/a" | "na" => StatutEtape::NonApplicable,
            _ => StatutEtape::AFaire,
        }
    }
}

/// An itineraire step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Etape {
    pub titre: String,
    pub statut: StatutEtape,
}

impl Etape {
    pub fn nouvelle(titre: impl Into<String>) -> Self {
        Self {
            titre: titre.into(),
            statut: StatutEtape::AFaire,
        }
    }
}

/// The butinage plan. Empty at first: the tool sets it itself via the `plan` tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Itineraire {
    pub etapes: Vec<Etape>,
}

impl Itineraire {
    pub fn vide() -> Self {
        Self::default()
    }

    /// Replaces the itineraire with a new list of titles (all `AFaire`).
    pub fn definir(&mut self, titres: Vec<String>) {
        self.etapes = titres.into_iter().map(Etape::nouvelle).collect();
    }

    /// Merges an update: if the index exists we change its status, otherwise we
    /// add the step. Tolerant of out-of-bounds indices (silent no-op on read).
    pub fn marquer(&mut self, index: usize, statut: StatutEtape) {
        if let Some(e) = self.etapes.get_mut(index) {
            e.statut = statut;
        }
    }

    /// No step set.
    pub fn est_vide(&self) -> bool {
        self.etapes.is_empty()
    }

    /// All steps are closed (or the itineraire is empty, so nothing prevents the end).
    pub fn tout_termine(&self) -> bool {
        self.etapes.iter().all(|e| e.statut.est_close())
    }

    /// At least one step still requires work.
    pub fn a_des_ouvertes(&self) -> bool {
        self.etapes.iter().any(|e| e.statut.est_ouverte())
    }

    /// Number of closed (completed/abandoned) steps. Progress signal for the Tier 3
    /// supervisor: when this stops increasing, the task is stalling.
    pub fn nb_faites(&self) -> u32 {
        self.etapes.iter().filter(|e| e.statut.est_close()).count() as u32
    }

    /// Index of the next open step, if any.
    pub fn prochaine_ouverte(&self) -> Option<usize> {
        self.etapes.iter().position(|e| e.statut.est_ouverte())
    }

    /// Renders the itineraire "terminal" for the final display: every still-open
    /// step becomes `NonApplicable` (the mission hands back control, these steps
    /// will not be done). Used when posting a final answer.
    pub fn finaliser(&mut self) {
        for e in &mut self.etapes {
            if e.statut.est_ouverte() {
                e.statut = StatutEtape::NonApplicable;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn itineraire_vide_est_termine() {
        let it = Itineraire::vide();
        assert!(it.tout_termine());
        assert!(!it.a_des_ouvertes());
        assert!(it.prochaine_ouverte().is_none());
    }

    #[test]
    fn etapes_ouvertes_empechent_la_fin() {
        let mut it = Itineraire::vide();
        it.definir(vec!["chercher".into(), "synthétiser".into()]);
        assert!(!it.tout_termine());
        assert!(it.a_des_ouvertes());
        assert_eq!(it.prochaine_ouverte(), Some(0));

        it.marquer(0, StatutEtape::Terminee);
        assert_eq!(it.prochaine_ouverte(), Some(1));
        assert!(!it.tout_termine());

        it.marquer(1, StatutEtape::Terminee);
        assert!(it.tout_termine());
        assert!(!it.a_des_ouvertes());
    }

    #[test]
    fn bloquee_et_non_applicable_comptent_comme_closes() {
        let mut it = Itineraire::vide();
        it.definir(vec!["a".into(), "b".into()]);
        it.marquer(0, StatutEtape::Bloquee);
        it.marquer(1, StatutEtape::NonApplicable);
        assert!(it.tout_termine());
    }

    #[test]
    fn finaliser_ferme_les_etapes_ouvertes() {
        let mut it = Itineraire::vide();
        it.definir(vec!["a".into(), "b".into()]);
        it.marquer(0, StatutEtape::Terminee);
        it.finaliser();
        assert!(it.tout_termine());
        assert_eq!(it.etapes[1].statut, StatutEtape::NonApplicable);
    }

    #[test]
    fn marquer_hors_borne_est_silencieux() {
        let mut it = Itineraire::vide();
        it.definir(vec!["a".into()]);
        it.marquer(9, StatutEtape::Terminee); // does not panic
        assert!(!it.tout_termine());
    }
}
