//! Les **événements** émis pendant un butinage, et l'`Emetteur` qui les reçoit.
//!
//! L'adaptateur node mappe ces événements vers son `ChatEvent` (WebSocket dashboard).
//! La boucle ne connaît que ce canal neutre. Un [`Silencieux`] no-op sert aux tests
//! et aux exécutions de fond (cron, sous-agents non observés).

/// Événement observable d'un butinage.
#[derive(Debug, Clone)]
pub enum Evenement {
    /// Message de statut (orientation, attente, rotation…).
    Statut(String),
    /// Fragment de texte du modèle (streaming).
    Texte(String),
    /// Un outil va s'exécuter.
    AppelOutil { nom: String },
    /// Résultat d'un outil.
    ResultatOutil { nom: String, ok: bool, ms: u64 },
    /// Compaction/consolidation du contexte.
    Escale { avant: usize, apres: usize },
    /// Réponse finale prête.
    Fin(String),
}

/// Récepteur d'événements.
pub trait Emetteur: Send + Sync {
    fn emettre(&self, ev: Evenement);
}

/// Émetteur no-op (tests, exécutions de fond).
pub struct Silencieux;

impl Emetteur for Silencieux {
    fn emettre(&self, _ev: Evenement) {}
}
