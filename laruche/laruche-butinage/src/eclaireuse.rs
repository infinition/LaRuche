//! Les **éclaireuses** — sous-agents (abeilles scouts) dépêchés sur une sous-mission.
//!
//! Pattern orchestrateur-ouvrière : pour ne pas polluer le contexte du parent, une
//! recherche large (ou une vérification, une synthèse) est confiée à une éclaireuse
//! au **contexte isolé** et au **budget séparé** ; elle ne remonte qu'un **rapport
//! compact**. C'est un simple `butiner()` enfant — la délégation est « juste un outil »
//! du point de vue de la boucle parente.
//!
//! Le pont (`laruche-essaim`) câble l'outil `delegate` sur [`depecher`], en désactivant
//! `delegate` chez l'enfant (garde anti-récursion : un seul niveau).

use crate::carnet::{Carnet, ModeMission};
use crate::evenement::Emetteur;
use crate::fournisseur::Fournisseur;
use crate::outils::Outils;
use crate::reglages::Reglages;
use anyhow::Result;

/// Rôle de l'éclaireuse — fixe sa directive et son budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Scout : recherche large, collecte de faits et sources.
    Eclaireuse,
    /// Ouvrière : exécution/calcul d'une sous-tâche.
    Ouvriere,
    /// Gardienne : vérification critique d'un résultat/affirmation.
    Gardienne,
    /// Architecte : synthèse structurée d'un matériau fourni.
    Architecte,
}

impl Role {
    pub fn depuis(s: &str) -> Role {
        match s.to_lowercase().as_str() {
            "ouvriere" | "ouvrière" | "worker" | "exec" | "code" => Role::Ouvriere,
            "gardienne" | "critic" | "review" | "verify" => Role::Gardienne,
            "architecte" | "synthese" | "synthèse" | "synthesis" | "report" => Role::Architecte,
            _ => Role::Eclaireuse,
        }
    }

    /// Directive (anglais) ajoutée au prompt système de l'enfant.
    pub fn directive(self) -> &'static str {
        match self {
            Role::Eclaireuse => "You are a SCOUT bee on a focused research sub-mission. Search broadly across \
                several angles, gather concrete facts with sources, and report concisely. Never ask the user \
                questions — you are autonomous. Call task_complete with a tight factual summary when done.",
            Role::Ouvriere => "You are a WORKER bee executing one focused sub-task (computation, file work, \
                code). Do the work with tools, verify it, and report the concrete result via task_complete.",
            Role::Gardienne => "You are a GUARDIAN bee. Critically verify the given claim or result. Try to \
                refute it; default to skepticism. Report whether it holds and why via task_complete.",
            Role::Architecte => "You are an ARCHITECT bee. Synthesize the provided material into a clear, \
                structured report. No new research. Deliver the report via task_complete.",
        }
    }

    /// Budget de passes par défaut du rôle.
    pub fn plafond(self) -> usize {
        match self {
            Role::Eclaireuse => 30,
            Role::Ouvriere => 20,
            Role::Architecte => 12,
            Role::Gardienne => 10,
        }
    }

    fn mode(self) -> ModeMission {
        match self {
            Role::Eclaireuse => ModeMission::Exploration,
            _ => ModeMission::Standard,
        }
    }
}

/// Ordre de mission d'une éclaireuse.
#[derive(Debug, Clone)]
pub struct OrdreEclaireuse {
    pub role: Role,
    pub tache: String,
    pub contexte: Option<String>,
}

/// Rapport compact remonté au parent.
#[derive(Debug, Clone)]
pub struct Rapport {
    pub tache: String,
    pub role: Role,
    pub synthese: String,
}

impl Rapport {
    /// Rendu prêt à réinjecter comme observation chez le parent.
    pub fn en_observation(&self) -> String {
        format!("[Rapport d'éclaireuse — {:?}]\nTâche : {}\n\n{}", self.role, self.tache, self.synthese)
    }
}

/// Dépêche une éclaireuse : exécute un `butiner()` enfant à contexte isolé et budget
/// propre, puis renvoie son rapport. `fournisseur`/`outils` sont les adaptateurs ENFANT
/// (le pont y désactive `delegate` pour empêcher la récursion).
pub async fn depecher(
    ordre: OrdreEclaireuse,
    reglages_parent: &Reglages,
    fournisseur: &dyn Fournisseur,
    outils: &dyn Outils,
    emet: &dyn Emetteur,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Rapport> {
    let reglages_enfant = Reglages {
        plafond_passes: ordre.role.plafond(),
        systeme: format!("{}\n\n## Sub-mission role\n{}", reglages_parent.systeme, ordre.role.directive()),
        chemin_carnet: None, // l'enfant n'a pas besoin de checkpoint disque
        ..reglages_parent.clone()
    };

    let mission = match &ordre.contexte {
        Some(c) if !c.trim().is_empty() => format!("{}\n\nContext from the parent:\n{}", ordre.tache, c),
        _ => ordre.tache.clone(),
    };

    let mut carnet = Carnet::ouvrir(&mission, ordre.role.mode(), now);
    let bilan = crate::cycle::butiner(&mut carnet, &reglages_enfant, fournisseur, outils, emet).await?;

    Ok(Rapport {
        tache: ordre.tache,
        role: ordre.role,
        synthese: bilan.texte,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evenement::Silencieux;
    use crate::fournisseur::{ErreurFournisseur, ReponseModele};
    use crate::issue::{Appel, StopReason};
    use crate::outils::{Outils, ResultatOutil};
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Mutex;

    struct FournisseurUneFin(Mutex<Option<ReponseModele>>);
    #[async_trait]
    impl Fournisseur for FournisseurUneFin {
        async fn repondre(
            &self,
            _m: &[crate::messagerie::Message],
            _s: &[serde_json::Value],
        ) -> std::result::Result<ReponseModele, ErreurFournisseur> {
            Ok(self.0.lock().unwrap().take().unwrap_or(ReponseModele {
                texte: "(fin)".into(),
                stop: StopReason::FinTour,
                appels: vec![],
                usage: None,
            }))
        }
    }

    struct OutilsVides;
    #[async_trait]
    impl Outils for OutilsVides {
        async fn executer(&self, _a: &Appel) -> ResultatOutil {
            ResultatOutil::ok("")
        }
    }

    fn t0() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn role_depuis_chaine() {
        assert_eq!(Role::depuis("worker"), Role::Ouvriere);
        assert_eq!(Role::depuis("verify"), Role::Gardienne);
        assert_eq!(Role::depuis("report"), Role::Architecte);
        assert_eq!(Role::depuis("n'importe"), Role::Eclaireuse);
    }

    #[tokio::test]
    async fn depecher_renvoie_la_synthese_de_l_enfant() {
        let four = FournisseurUneFin(Mutex::new(Some(ReponseModele {
            texte: String::new(),
            stop: StopReason::Outils,
            appels: vec![Appel::nouveau("task_complete", json!({"summary": "3 sources trouvées"}))],
            usage: None,
        })));
        let ordre = OrdreEclaireuse {
            role: Role::Eclaireuse,
            tache: "trouver des sources sur X".into(),
            contexte: None,
        };
        let rapport = depecher(ordre, &Reglages::default(), &four, &OutilsVides, &Silencieux, t0())
            .await
            .unwrap();
        assert_eq!(rapport.role, Role::Eclaireuse);
        assert_eq!(rapport.synthese, "3 sources trouvées");
        assert!(rapport.en_observation().contains("trouver des sources"));
    }
}
