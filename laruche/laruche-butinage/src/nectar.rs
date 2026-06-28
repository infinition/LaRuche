//! The **nectar**: the tool's context source (memory), abstracted by a trait.
//!
//! Dependency inversion: the engine does not know the concrete memory (cognitive
//! graph, vectors...), only this trait. The adapter (`laruche-essaim`) implements it
//! on top of `MemoireCognitive`. Optional: a butinage can run without a `Source`.

use async_trait::async_trait;

/// Provider of durable context (memory).
#[async_trait]
pub trait Source: Send + Sync {
    /// Recalls context relevant to the query (just-in-time retrieval).
    /// `None` if nothing relevant.
    async fn rappeler(&self, requete: &str) -> Option<String>;

    /// Records a durable fact under a dotted node identifier (`domaine.sujet`).
    async fn consigner(&self, node_id: &str, fait: &str);
}
