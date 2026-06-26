//! Le **carnet de bord** — l'état persistable d'un butinage.
//!
//! Sérialisé à chaque fin de passe (`sauver`). Un crash en pleine mission (passe 60
//! d'une recherche de 3 h) se reprend exactement où il en était, au lieu de tout
//! recommencer. Ne contient que l'essentiel à la reprise ; les compteurs de la
//! [`crate::cap::vigie::Vigie`] sont éphémères (ils se reconstituent au fil des passes).

use crate::cap::boussole::ContexteCap;
use crate::itineraire::Itineraire;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Mode de mission — influe sur la politique de la boussole et les rails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModeMission {
    /// Tâche normale : on s'arrête dès que l'itinéraire est bouclé.
    #[default]
    Standard,
    /// Recherche longue : on refuse les conclusions prématurées (cf. `cap`).
    Exploration,
}

/// L'état repris-après-crash d'un butinage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Carnet {
    pub id: String,
    pub mission: String,
    pub mode: ModeMission,
    pub itineraire: Itineraire,
    /// Numéro de la passe courante (0-based).
    pub passe: usize,
    /// Auto-continuations consommées depuis la dernière récolte d'outil.
    pub auto_continue: usize,
    /// Appels d'outils web/réseau réellement effectués (preuve de recherche).
    pub recolte_web: usize,
    pub cree_le: chrono::DateTime<chrono::Utc>,
    pub maj_le: chrono::DateTime<chrono::Utc>,
}

impl Carnet {
    /// Nouveau carnet pour une mission. `now` injecté (les horloges ne sont pas
    /// déterministes ; l'appelant fournit l'instant — utile pour les tests).
    pub fn ouvrir(mission: impl Into<String>, mode: ModeMission, now: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            mission: mission.into(),
            mode,
            itineraire: Itineraire::vide(),
            passe: 0,
            auto_continue: 0,
            recolte_web: 0,
            cree_le: now,
            maj_le: now,
        }
    }

    /// Réarme le budget d'auto-continuation (appelé quand un outil s'exécute = vrai progrès).
    pub fn rearmer_auto(&mut self) {
        self.auto_continue = 0;
    }

    /// Consomme une auto-continuation.
    pub fn consommer_auto(&mut self) {
        self.auto_continue += 1;
    }

    /// Construit le contexte de décision pour [`crate::cap::boussole::cap`].
    pub fn contexte_cap(&self, auto_continue_max: usize, min_web_exploration: usize) -> ContexteCap {
        ContexteCap {
            plan_inacheve: self.itineraire.a_des_ouvertes(),
            auto_continue: self.auto_continue,
            auto_continue_max,
            mode_exploration: self.mode == ModeMission::Exploration,
            recolte_web: self.recolte_web,
            min_web_exploration,
        }
    }

    /// Persiste le carnet en JSON (checkpoint). `now` injecté pour `maj_le`.
    pub fn sauver(&mut self, chemin: &Path, now: chrono::DateTime<chrono::Utc>) -> Result<()> {
        self.maj_le = now;
        if let Some(parent) = chemin.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("création du dossier {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(self).context("sérialisation du carnet")?;
        std::fs::write(chemin, json).with_context(|| format!("écriture {}", chemin.display()))?;
        Ok(())
    }

    /// Recharge un carnet depuis le disque (reprise après crash).
    pub fn charger(chemin: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(chemin)
            .with_context(|| format!("lecture {}", chemin.display()))?;
        serde_json::from_str(&json).context("désérialisation du carnet")
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
        let ctx = c.contexte_cap(6, 12);
        assert!(ctx.plan_inacheve);
        assert!(ctx.mode_exploration);
        assert_eq!(ctx.recolte_web, 4);
        assert_eq!(ctx.auto_continue, 2);
        assert_eq!(ctx.auto_continue_max, 6);
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
        assert!(!repris.itineraire.tout_termine()); // étape 2 encore ouverte

        let _ = std::fs::remove_dir_all(&dir);
    }
}
