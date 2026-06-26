//! Les **outils** (le registre d'abeilles), abstraits par un trait.
//!
//! L'adaptateur `laruche-essaim` enveloppe son `AbeilleRegistry`. La boucle ne sait
//! qu'exécuter un appel, savoir si un outil est idempotent (pour la [`crate::cap::vigie`])
//! et s'il est sûr à exécuter en parallèle (pour la récolte concurrente).

use crate::issue::Appel;
use async_trait::async_trait;

/// Résultat d'un appel d'outil.
#[derive(Debug, Clone)]
pub struct ResultatOutil {
    pub ok: bool,
    pub sortie: String,
}

impl ResultatOutil {
    pub fn ok(s: impl Into<String>) -> Self {
        Self { ok: true, sortie: s.into() }
    }
    pub fn echec(s: impl Into<String>) -> Self {
        Self { ok: false, sortie: s.into() }
    }
    /// Empreinte du résultat (détection de stagnation par la vigie).
    pub fn empreinte(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.sortie.hash(&mut h);
        h.finish()
    }
}

/// Le registre d'outils exécutable.
#[async_trait]
pub trait Outils: Send + Sync {
    /// Exécute un appel et renvoie son résultat (jamais d'erreur remontée : un échec
    /// devient un `ResultatOutil { ok: false }` observable par le modèle).
    async fn executer(&self, appel: &Appel) -> ResultatOutil;

    /// Outil en lecture seule (mêmes args ⇒ même effet) → la vigie surveille la stagnation,
    /// et la récolte peut le lancer en parallèle.
    fn idempotent(&self, _nom: &str) -> bool {
        false
    }

    /// Sûr à exécuter en parallèle avec d'autres appels sûrs (pas de mutation/approbation).
    fn concurrence_sure(&self, appel: &Appel) -> bool {
        self.idempotent(&appel.nom)
    }

    /// Compte un appel comme « recherche web » (preuve d'effort en mode exploration).
    fn est_web(&self, appel: &Appel) -> bool {
        appel.nom.starts_with("web_") || appel.nom.starts_with("browser_")
    }

    /// Schémas d'outils à injecter au prompt.
    fn schemas(&self) -> Vec<serde_json::Value> {
        Vec::new()
    }
}
