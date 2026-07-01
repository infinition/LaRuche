//! The **éclaireuses**: sub-agents dispatched on a sub-mission.
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
    /// Éclaireuse: broad search, gathering facts and sources.
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

    /// Minimal web calls before the exploration rail accepts an end. Only the scout
    /// searches broadly; the other roles must not be forced into web quotas sized
    /// for the parent's long research.
    fn min_web(self) -> usize {
        match self {
            Role::Eclaireuse => 6,
            _ => 0,
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
    annulation: Option<&std::sync::atomic::AtomicBool>,
) -> Result<Rapport> {
    // The parent's system prompt may carry the deep-research protocol, which orders a
    // `delegate` fan-out — but delegation is DISABLED in the child (single recursion
    // level). Left in place, every scout would waste its first passes hitting the
    // anti-recursion wall. Strip it, and state the boundary explicitly.
    let systeme_parent = reglages_parent
        .systeme
        .replace(crate::cap::boussole::PROTOCOLE_EXPLORATION, "");
    let reglages_enfant = Reglages {
        plafond_passes: ordre.role.plafond(),
        min_web_exploration: ordre.role.min_web(),
        systeme: format!(
            "{}\n\n## Sub-mission role\n{}\nYou CANNOT delegate further: `delegate` is \
             unavailable at your level. Do the work YOURSELF with your direct tools \
             (searches, fetches, files, shell).",
            systeme_parent.trim_end(),
            ordre.role.directive()
        ),
        chemin_carnet: None, // the child does not need a disk checkpoint
        supervision: None,   // the parent's Tier 3 watches the PARENT, not the child
        ..reglages_parent.clone()
    };

    let mission = match &ordre.contexte {
        Some(c) if !c.trim().is_empty() => format!("{}\n\nContext from the parent:\n{}", ordre.tache, c),
        _ => ordre.tache.clone(),
    };

    let mut carnet = Carnet::ouvrir(&mission, ordre.role.mode(), now);
    // Éclaireuses are bounded and short, so no memory consolidation (source None).
    // The parent's cancellation flag propagates: killing the run kills the children.
    let bilan = crate::cycle::butiner(
        &mut carnet,
        &reglages_enfant,
        fournisseur,
        outils,
        emet,
        None,
        None,
        annulation,
    )
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

    /// Captures the system message of the first model call (child prompt inspection).
    struct FournisseurEspion(Mutex<Option<String>>);
    #[async_trait]
    impl Fournisseur for FournisseurEspion {
        async fn repondre(
            &self,
            m: &[crate::messagerie::Message],
            _s: &[serde_json::Value],
        ) -> std::result::Result<ReponseModele, ErreurFournisseur> {
            let sys = m
                .iter()
                .find(|x| x.role == crate::messagerie::Role::Systeme)
                .map(|x| x.contenu.clone())
                .unwrap_or_default();
            *self.0.lock().unwrap() = Some(sys);
            Ok(ReponseModele {
                texte: String::new(),
                stop: StopReason::Outils,
                appels: vec![Appel::nouveau("task_complete", json!({"summary": "ok"}))],
                usage: None,
            })
        }
    }

    #[tokio::test]
    async fn le_prompt_enfant_est_purge_du_protocole_fan_out() {
        let four = FournisseurEspion(Mutex::new(None));
        let parent = Reglages::default().avec_systeme(format!(
            "IDENTITE\n\n{}",
            crate::cap::boussole::PROTOCOLE_EXPLORATION
        ));
        let ordre = OrdreEclaireuse {
            role: Role::Eclaireuse,
            tache: "angle 1".into(),
            contexte: None,
        };
        depecher(ordre, &parent, &four, &OutilsVides, &Silencieux, t0(), None)
            .await
            .unwrap();
        let sys = four.0.lock().unwrap().clone().unwrap();
        assert!(sys.contains("IDENTITE"), "parent identity kept");
        assert!(
            !sys.contains("Deep-research protocol"),
            "fan-out protocol stripped: the child cannot delegate"
        );
        assert!(sys.contains("CANNOT delegate further"), "boundary stated explicitly");
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
        let rapport =
            depecher(ordre, &Reglages::default(), &four, &OutilsVides, &Silencieux, t0(), None)
                .await
                .unwrap();
        assert_eq!(rapport.role, Role::Eclaireuse);
        assert_eq!(rapport.synthese, "3 sources trouvées");
        assert!(rapport.en_observation().contains("trouver des sources"));
    }
}
