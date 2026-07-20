//! The **events** emitted during a butinage, and the `Emetteur` that receives them.
//!
//! The node adapter maps these events to its `ChatEvent` (WebSocket dashboard).
//! The loop only knows this neutral channel. A no-op [`Silencieux`] serves tests
//! and background runs (cron, unobserved subagents).

/// Observable event of a butinage.
#[derive(Debug, Clone)]
pub enum Evenement {
    /// Status message (orientation, waiting, rotation...).
    Statut(String),
    /// Text fragment from the model (streaming).
    Texte(String),
    /// A tool is about to run.
    AppelOutil { nom: String },
    /// Result of a tool.
    ResultatOutil { nom: String, ok: bool, ms: u64 },
    /// Context compaction/consolidation.
    Escale { avant: usize, apres: usize },
    /// Itinerary changed (plan set or statuses updated). `(titre, statut)` pairs,
    /// statut in {"pending","in_progress","done","blocked","skipped"}. Lets the UI
    /// track plan progress even when the plan arrives as a NATIVE tool call (the
    /// text-based `<plan>` path never fires then).
    Itineraire { etapes: Vec<(String, String)> },
    /// Final response ready.
    Fin(String),
}

/// Event receiver.
pub trait Emetteur: Send + Sync {
    fn emettre(&self, ev: Evenement);
}

/// No-op emitter (tests, background runs).
pub struct Silencieux;

impl Emetteur for Silencieux {
    fn emettre(&self, _ev: Evenement) {}
}
