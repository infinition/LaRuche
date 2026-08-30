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
    /// Images produites par l'outil, en base64.
    ///
    /// Une capture d'ecran, une page, une photo de webcam. Elles etaient
    /// remontees jusqu'a l'interface pour l'affichage et perdues ici, donc le
    /// modele ne les a JAMAIS vues: il repondait "capture prise" sans avoir rien
    /// regarde, et disait parfois lui-meme qu'il ne recevait pas l'image. Un
    /// outil qui rend une image et dont l'image n'arrive pas est un outil qui
    /// ment sur ce qu'il fait.
    pub images: Vec<String>,
}

impl ResultatOutil {
    pub fn ok(s: impl Into<String>) -> Self {
        Self { ok: true, sortie: s.into(), images: Vec::new() }
    }
    pub fn echec(s: impl Into<String>) -> Self {
        Self { ok: false, sortie: s.into(), images: Vec::new() }
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

    /// WEIGHT of a call toward the exploration effort counter (`recolte_web`).
    /// Default: 1 per web call. Adapters override it for calls that embody MORE
    /// than one search - e.g. a `delegate` scout runs several real searches in its
    /// own context, so counting it as 1 starves the parent's `min_web_exploration`
    /// and gets a perfect fan-out nudged for "not searching enough".
    fn poids_web(&self, appel: &Appel) -> usize {
        usize::from(self.est_web(appel))
    }

    /// Tool schemas to inject into the prompt.
    fn schemas(&self) -> Vec<serde_json::Value> {
        Vec::new()
    }

    /// Capabilities that appeared AFTER the mission started, as a ready-to-read block.
    ///
    /// [`Self::schemas`] is captured once before the loop, because the prompt and the
    /// native tool set must stay byte-identical for the provider's cached prefix to
    /// hold. A plugin the agent forges at turn 5 is therefore callable (the registry
    /// behind it is live) but absent from every list it can see, which reads as a
    /// failure and gets it created again.
    ///
    /// The adapter reports the difference here and the loop carries it in the VOLATILE
    /// tail tier: last thing the model reads, and behind the cached prefix, so it costs
    /// nothing to keep fresh. `None` when nothing new appeared.
    fn nouvelles_capacites(&self) -> Option<String> {
        None
    }
}
