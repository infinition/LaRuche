//! L'itinéraire de butinage = le plan/todo structuré.
//!
//! C'est la **source de vérité de la terminaison** : on ne décide jamais qu'une
//! mission est finie en lisant le texte du modèle (« j'ai terminé »), mais en
//! constatant que toutes les étapes de l'itinéraire sont terminées ou non applicables.

use serde::{Deserialize, Serialize};

/// Statut d'une étape de l'itinéraire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatutEtape {
    /// Pas encore commencée.
    AFaire,
    /// En cours de traitement.
    EnCours,
    /// Terminée avec succès.
    Terminee,
    /// Bloquée (dépendance, erreur externe) — n'empêche pas la terminaison globale.
    Bloquee,
    /// Devenue sans objet (ex. recherche conditionnelle qui n'a rien donné).
    NonApplicable,
}

impl StatutEtape {
    /// Une étape « close » ne demande plus de travail (terminée, abandonnée, ou bloquée
    /// durablement). Sert à savoir si l'itinéraire est globalement accompli.
    pub fn est_close(self) -> bool {
        matches!(
            self,
            StatutEtape::Terminee | StatutEtape::NonApplicable | StatutEtape::Bloquee
        )
    }

    /// Une étape « ouverte » réclame encore une action de l'abeille.
    pub fn est_ouverte(self) -> bool {
        matches!(self, StatutEtape::AFaire | StatutEtape::EnCours)
    }
}

/// Une étape de l'itinéraire.
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

/// Le plan de butinage. Vide au départ : l'abeille le pose elle-même via l'outil `plan`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Itineraire {
    pub etapes: Vec<Etape>,
}

impl Itineraire {
    pub fn vide() -> Self {
        Self::default()
    }

    /// Remplace l'itinéraire par une nouvelle liste de titres (toutes `AFaire`).
    pub fn definir(&mut self, titres: Vec<String>) {
        self.etapes = titres.into_iter().map(Etape::nouvelle).collect();
    }

    /// Fusionne une mise à jour : si l'index existe on change son statut, sinon on
    /// ajoute l'étape. Tolérant aux index hors borne (no-op silencieux côté lecture).
    pub fn marquer(&mut self, index: usize, statut: StatutEtape) {
        if let Some(e) = self.etapes.get_mut(index) {
            e.statut = statut;
        }
    }

    /// Aucune étape posée.
    pub fn est_vide(&self) -> bool {
        self.etapes.is_empty()
    }

    /// Toutes les étapes sont closes (ou l'itinéraire est vide → rien n'empêche la fin).
    pub fn tout_termine(&self) -> bool {
        self.etapes.iter().all(|e| e.statut.est_close())
    }

    /// Au moins une étape réclame encore du travail.
    pub fn a_des_ouvertes(&self) -> bool {
        self.etapes.iter().any(|e| e.statut.est_ouverte())
    }

    /// Index de la prochaine étape ouverte, le cas échéant.
    pub fn prochaine_ouverte(&self) -> Option<usize> {
        self.etapes.iter().position(|e| e.statut.est_ouverte())
    }

    /// Rend l'itinéraire « terminal » pour l'affichage final : toute étape encore
    /// ouverte devient `NonApplicable` (la mission rend la main, ces étapes ne seront
    /// pas faites). Utilisé au moment de poser une réponse finale.
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
        it.marquer(9, StatutEtape::Terminee); // ne panique pas
        assert!(!it.tout_termine());
    }
}
