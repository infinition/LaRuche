//! The **tools** (the bee registry), abstracted behind a trait.
//!
//! The `laruche-essaim` adapter wraps its `AbeilleRegistry`. The loop only knows how
//! to execute a call, whether a tool is idempotent (for [`crate::cap::vigie`])
//! and whether it is safe to run in parallel (for concurrent harvesting).

use crate::issue::Appel;
use async_trait::async_trait;

/// Result of a tool call.
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
    /// Fingerprint of the result (stagnation detection by the vigie).
    pub fn empreinte(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.sortie.hash(&mut h);
        h.finish()
    }
}

/// The executable tool registry.
#[async_trait]
pub trait Outils: Send + Sync {
    /// Executes a call and returns its result (errors are never propagated: a failure
    /// becomes a `ResultatOutil { ok: false }` observable by the model).
    async fn executer(&self, appel: &Appel) -> ResultatOutil;

    /// Read-only tool (same args ⇒ same effect) → the vigie watches for stagnation,
    /// and the recolte can run it in parallel.
    fn idempotent(&self, _nom: &str) -> bool {
        false
    }

    /// Safe to run in parallel with other safe calls (no mutation/approval).
    fn concurrence_sure(&self, appel: &Appel) -> bool {
        self.idempotent(&appel.nom)
    }

    /// Per-tool timeout override (seconds). `None` = use `Reglages::timeout_outil_secs`;
    /// `Some(0)` = no timeout (e.g. delegation to a sub-agent, approval waits).
    fn timeout_secs(&self, _nom: &str) -> Option<u64> {
        None
    }

    /// Counts a call as a "web search" (proof of effort in exploration mode).
    fn est_web(&self, appel: &Appel) -> bool {
        appel.nom.starts_with("web_") || appel.nom.starts_with("browser_")
    }

    /// Tool schemas to inject into the prompt.
    fn schemas(&self) -> Vec<serde_json::Value> {
        Vec::new()
    }
}
