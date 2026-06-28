//! The **éclaireuses**: sub-agents (scout tools) dispatched on a sub-mission.
//!
//! Orchestrator-worker pattern: to avoid polluting the parent's context, a broad
//! search (or a verification, a synthesis) is handed to an éclaireuse with an
//! **isolated context** and a **separate budget**; it only reports back a **compact
//! report**. It is a plain child `butiner()`: delegation is "just a tool" from the
//! parent loop's point of view.
//!
//! The bridge (`laruche-essaim`) wires the `delegate` tool onto [`depecher`], disabling
//! `delegate` in the child (anti-recursion guard: a single level).

use crate::carnet::{Carnet, ModeMission};
use crate::evenement::Emetteur;
use crate::fournisseur::Fournisseur;
use crate::outils::Outils;
use crate::reglages::Reglages;
use anyhow::Result;

/// Role of the éclaireuse: sets its directive and its budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Scout: broad search, gathering facts and sources.
    Eclaireuse,
    /// Worker: execution/computation of a sub-task.
    Ouvriere,
    /// Guardian: critical verification of a result/claim.
    Gardienne,
    /// Architect: structured synthesis of provided material.
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

    /// Directive (English) appended to the child's system prompt.
    pub fn directive(self) -> &'static str {
        match self {
            Role::Eclaireuse => "You are a SCOUT bee on a focused research sub-mission. Search broadly across \
                several angles, gather concrete facts with sources, and report concisely. Never ask the user \
                questions: you are autonomous. Call task_complete with a tight factual summary when done.",
            Role::Ouvriere => "You are a WORKER bee executing one focused sub-task (computation, file work, \
                code). Do the work with tools, verify it, and report the concrete result via task_complete.",
            Role::Gardienne => "You are a GUARDIAN bee. Critically verify the given claim or result. Try to \
                refute it; default to skepticism. Report whether it holds and why via task_complete.",
            Role::Architecte => "You are an ARCHITECT bee. Synthesize the provided material into a clear, \
                structured report. No new research. Deliver the report via task_complete.",
        }
    }

    /// Default pass budget for the role.
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

/// Mission order for an éclaireuse.
#[derive(Debug, Clone)]
pub struct OrdreEclaireuse {
    pub role: Role,
    pub tache: String,
    pub contexte: Option<String>,
}

/// Compact report sent back to the parent.
#[derive(Debug, Clone)]
pub struct Rapport {
    pub tache: String,
    pub role: Role,
    pub synthese: String,
}

impl Rapport {
    /// Rendering ready to reinject as an observation in the parent.
    pub fn en_observation(&self) -> String {
        format!("[Éclaireuse report: {:?}]\nTask: {}\n\n{}", self.role, self.tache, self.synthese)
    }
}

/// Dispatches an éclaireuse: runs a child `butiner()` with an isolated context and its
/// own budget, then returns its report. `fournisseur`/`outils` are the CHILD adapters
/// (the bridge disables `delegate` there to prevent recursion).
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
        chemin_carnet: None, // the child does not need a disk checkpoint
        ..reglages_parent.clone()
    };

    let mission = match &ordre.contexte {
        Some(c) if !c.trim().is_empty() => format!("{}\n\nContext from the parent:\n{}", ordre.tache, c),
        _ => ordre.tache.clone(),
    };

    let mut carnet = Carnet::ouvrir(&mission, ordre.role.mode(), now);
    // Éclaireuses are bounded and short, so no memory consolidation (source None).
    let bilan =
        crate::cycle::butiner(&mut carnet, &reglages_enfant, fournisseur, outils, emet, None, None)
            .await?;

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
