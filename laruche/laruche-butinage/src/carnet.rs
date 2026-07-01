//! The **carnet de bord**: the persistable state of a butinage.
//!
//! Serialized at the end of each pass (`sauver`). A crash mid-mission (pass 60
//! of a 3 h search) resumes exactly where it left off, instead of restarting
//! from scratch. Holds only what is needed to resume; the counters of the
//! [`crate::cap::vigie::Vigie`] are ephemeral (rebuilt over the passes).

use crate::cap::boussole::ContexteCap;
use crate::itineraire::Itineraire;
use crate::messagerie::Message;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Mission mode: affects the boussole policy and the rails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModeMission {
    /// Normal task: stop as soon as the itineraire is complete.
    #[default]
    Standard,
    /// Long search: refuse premature conclusions (cf. `cap`).
    Exploration,
}

/// The crash-resumable state of a butinage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Carnet {
    pub id: String,
    pub mission: String,
    pub mode: ModeMission,
    pub itineraire: Itineraire,
    /// Conversation history (system excluded: rebuilt from the reglages).
    #[serde(default)]
    pub historique: Vec<Message>,
    /// Multimodal attachments of the seed message (multiple images, audio...).
    /// Attached to the first user message when the loop injects it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pieces: Vec<crate::messagerie::Piece>,
    /// Index of the current pass (0-based).
    pub passe: usize,
    /// Auto-continuations consumed since the last tool recolte.
    pub auto_continue: usize,
    /// Web/network tool calls actually performed (proof of search).
    pub recolte_web: usize,
    /// Cumulative real input tokens over the whole butinage (provider-reported).
    #[serde(default)]
    pub tokens_entree_total: u64,
    /// Cumulative real output tokens over the whole butinage (provider-reported).
    #[serde(default)]
    pub tokens_sortie_total: u64,
    /// Vigie counters, persisted so a crash-resume keeps its anti-loop memory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vigie: Option<crate::cap::vigie::Vigie>,
    pub cree_le: chrono::DateTime<chrono::Utc>,
    pub maj_le: chrono::DateTime<chrono::Utc>,
}

impl Carnet {
    /// New carnet for a mission. `now` is injected (clocks are not
    /// deterministic; the caller supplies the instant, useful for tests).
    pub fn ouvrir(mission: impl Into<String>, mode: ModeMission, now: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            mission: mission.into(),
            mode,
            itineraire: Itineraire::vide(),
            historique: Vec::new(),
            pieces: Vec::new(),
            passe: 0,
            auto_continue: 0,
            recolte_web: 0,
            tokens_entree_total: 0,
            tokens_sortie_total: 0,
            vigie: None,
            cree_le: now,
            maj_le: now,
        }
    }

    /// Cumulative token spend (input + output), the budget signal.
    pub fn tokens_total(&self) -> u64 {
        self.tokens_entree_total + self.tokens_sortie_total
    }

    /// Rearms the auto-continuation budget (called when a tool runs = real progress).
    pub fn rearmer_auto(&mut self) {
        self.auto_continue = 0;
    }

    /// Consumes one auto-continuation.
    pub fn consommer_auto(&mut self) {
        self.auto_continue += 1;
    }

    /// Builds the decision context for [`crate::cap::boussole::cap`].
    pub fn contexte_cap(&self, relance_max: usize, min_web_exploration: usize) -> ContexteCap {
        ContexteCap {
            auto_continue: self.auto_continue,
            relance_max,
            mode_exploration: self.mode == ModeMission::Exploration,
            recolte_web: self.recolte_web,
            min_web_exploration,
        }
    }

    /// Persists the carnet as JSON (checkpoint). `now` is injected for `maj_le`.
    ///
    /// - **Internal** messages (steering nudges, resume notes) are filtered out: they
    ///   must not reappear on reload.
    /// - **Atomic** write (tmp + rename): a crash mid-write must not corrupt the very
    ///   checkpoint that exists to survive crashes.
    pub fn sauver(&mut self, chemin: &Path, now: chrono::DateTime<chrono::Utc>) -> Result<()> {
        self.maj_le = now;
        if let Some(parent) = chemin.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating directory {}", parent.display()))?;
        }
        let mut copie = self.clone();
        copie.historique.retain(|m| !m.interne);
        let json = serde_json::to_string_pretty(&copie).context("serializing the carnet")?;
        let tmp = chemin.with_extension("json.tmp");
        std::fs::write(&tmp, json).with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, chemin)
            .with_context(|| format!("renaming {} -> {}", tmp.display(), chemin.display()))?;
        Ok(())
    }

    /// Reloads a carnet from disk (crash recovery).
    pub fn charger(chemin: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(chemin)
            .with_context(|| format!("reading {}", chemin.display()))?;
        serde_json::from_str(&json).context("deserializing the carnet")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::itineraire::StatutEtape;

    fn t0() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn contexte_cap_reflete_l_etat() {
        let mut c = Carnet::ouvrir("trouver X", ModeMission::Exploration, t0());
        c.itineraire.definir(vec!["a".into(), "b".into()]);
        c.recolte_web = 4;
        c.auto_continue = 2;
        let ctx = c.contexte_cap(3, 12);
        assert!(ctx.mode_exploration);
        assert_eq!(ctx.recolte_web, 4);
        assert_eq!(ctx.auto_continue, 2);
        assert_eq!(ctx.relance_max, 3);
    }

    #[test]
    fn rearmer_et_consommer_auto() {
        let mut c = Carnet::ouvrir("m", ModeMission::Standard, t0());
        c.consommer_auto();
        c.consommer_auto();
        assert_eq!(c.auto_continue, 2);
        c.rearmer_auto();
        assert_eq!(c.auto_continue, 0);
    }

    #[test]
    fn sauver_puis_charger_reprend_l_etat() {
        let dir = std::env::temp_dir().join(format!("butinage-test-{}", uuid::Uuid::new_v4()));
        let chemin = dir.join("carnet.json");
        let mut c = Carnet::ouvrir("mission longue", ModeMission::Exploration, t0());
        c.itineraire.definir(vec!["étape 1".into(), "étape 2".into()]);
        c.itineraire.marquer(0, StatutEtape::Terminee);
        c.passe = 42;
        c.recolte_web = 7;
        c.sauver(&chemin, t0()).unwrap();

        let repris = Carnet::charger(&chemin).unwrap();
        assert_eq!(repris.id, c.id);
        assert_eq!(repris.passe, 42);
        assert_eq!(repris.recolte_web, 7);
        assert_eq!(repris.mode, ModeMission::Exploration);
        assert_eq!(repris.itineraire.etapes[0].statut, StatutEtape::Terminee);
        assert!(!repris.itineraire.tout_termine()); // step 2 still open

        let _ = std::fs::remove_dir_all(&dir);
    }
}
