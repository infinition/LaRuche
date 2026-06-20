//! Déclencheur persistant de maintenance de la mémoire pendant les périodes d'inactivité.
//!
//! Le déclencheur est séparé du runtime node : l'appelant fournit explicitement `now`, ce qui
//! garde la décision déterministe et facile à tester. La passe appelle uniquement `dream()` ;
//! elle ne peut donc jamais emprunter une opération de suppression.

use crate::MemoireCognitive;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// État durable du curator. L'absence de date signifie qu'aucune passe n'a encore réussi.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CuratorState {
    #[serde(default)]
    pub last_run_at: Option<DateTime<Utc>>,
}

/// Décide si une passe curator doit partir à `now`.
///
/// Un premier passage est autorisé. Ensuite, le délai est strict (`>`), conformément au
/// comportement idle : à exactement `interval_h` heures, la passe attend encore.
pub fn maybe_run_curator(state: &CuratorState, now: DateTime<Utc>, interval_h: i64) -> bool {
    if interval_h < 0 {
        return false;
    }

    match state.last_run_at {
        None => true,
        Some(last_run_at) => now.signed_duration_since(last_run_at) > Duration::hours(interval_h),
    }
}

/// Curator avec un état `last_run_at` sauvegardé à côté du store mémoire.
pub struct Curator {
    state_path: PathBuf,
    state: CuratorState,
}

impl Curator {
    /// Ouvre l'état existant, ou démarre sans historique si le fichier n'existe pas.
    pub fn open(state_path: impl AsRef<Path>) -> Result<Self> {
        let state_path = state_path.as_ref().to_path_buf();
        let state = if state_path.exists() {
            let content = std::fs::read_to_string(&state_path)?;
            serde_json::from_str(&content)?
        } else {
            CuratorState::default()
        };
        Ok(Self { state_path, state })
    }

    /// État courant, notamment utile aux intégrations et aux tests.
    pub fn state(&self) -> &CuratorState {
        &self.state
    }

    /// Lance `dream()` lorsque l'intervalle est dépassé, puis persiste la date de succès.
    ///
    /// Si la maintenance échoue, `last_run_at` reste inchangé afin qu'un prochain cycle puisse
    /// retenter. Aucune suppression n'est effectuée par ce déclencheur.
    pub async fn maybe_run_curator(
        &mut self,
        memory: &dyn MemoireCognitive,
        now: DateTime<Utc>,
        interval_h: i64,
    ) -> Result<Option<Value>> {
        if !maybe_run_curator(&self.state, now, interval_h) {
            return Ok(None);
        }

        let report = memory.dream().await?;
        self.state.last_run_at = Some(now);
        self.save()?;
        Ok(Some(report))
    }

    fn save(&self) -> Result<()> {
        if let Some(parent) = self.state_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.state)?;
        std::fs::write(&self.state_path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryItem, NativeBackend};

    fn at(hour: u32) -> DateTime<Utc> {
        format!("2026-06-19T{hour:02}:00:00Z").parse().unwrap()
    }

    fn state_path(name: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "laruche_curator_{name}_{}_{}.json",
            std::process::id(),
            stamp
        ))
    }

    #[test]
    fn curator_decision_is_strictly_after_the_interval() {
        let state = CuratorState {
            last_run_at: Some(at(0)),
        };
        assert!(!maybe_run_curator(&state, at(6), 6));
        assert!(maybe_run_curator(&state, at(7), 6));
        assert!(maybe_run_curator(&CuratorState::default(), at(0), 6));
    }

    #[tokio::test]
    async fn curator_runs_dream_once_and_persists_successful_run() {
        let state_path = state_path("persists");
        let backend = NativeBackend::new();
        backend
            .write(MemoryItem::new("decisions.archi", "Conserver la trace."))
            .await
            .unwrap();
        backend
            .write(MemoryItem::new("decisions.archi", "Conserver la trace."))
            .await
            .unwrap();

        let mut curator = Curator::open(&state_path).unwrap();
        let report = curator
            .maybe_run_curator(&backend, at(0), 6)
            .await
            .unwrap()
            .expect("première passe attendue");
        assert_eq!(report["duplicates"], 1);
        assert_eq!(curator.state().last_run_at, Some(at(0)));
        assert!(curator
            .maybe_run_curator(&backend, at(6), 6)
            .await
            .unwrap()
            .is_none());

        let reopened = Curator::open(&state_path).unwrap();
        assert_eq!(reopened.state().last_run_at, Some(at(0)));

        // dream() ne supprime rien : les deux entrées restent consultables après la passe.
        let node = backend.read_node("decisions.archi").await.unwrap();
        assert_eq!(node["items"].as_array().unwrap().len(), 2);
        let _ = std::fs::remove_file(state_path);
    }
}
