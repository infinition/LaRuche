//! The model (LLM) response **fournisseur**, abstracted behind a trait.
//!
//! Dependency inversion: `laruche-butinage` does not know about concrete
//! providers. The adapter (in `laruche-essaim`) implements this trait and handles
//! streaming, key rotation (`credential_pool`) and model rerouting internally; it
//! only surfaces an aggregated response or a terminal error. On top of it the loop
//! applies the [`crate::meteo`] policy (backoff, give-up).

use crate::issue::{Appel, StopReason};
use crate::messagerie::Message;
use async_trait::async_trait;

/// Token consumption (actual if the provider reports it).
#[derive(Debug, Clone, Copy, Default)]
pub struct Usage {
    pub entree: u32,
    pub sortie: u32,
}

/// Aggregated response of a model call.
#[derive(Debug, Clone)]
pub struct ReponseModele {
    pub texte: String,
    pub stop: StopReason,
    /// Tool calls emitted by the model (API-native or parsed by the adapter).
    pub appels: Vec<Appel>,
    pub usage: Option<Usage>,
}

/// Error of a model call, carrying what is needed to classify it ([`crate::meteo::ClasseErreur`]).
#[derive(Debug, Clone)]
pub struct ErreurFournisseur {
    pub status: u16,
    pub retry_after: Option<String>,
    pub corps: String,
}

impl std::fmt::Display for ErreurFournisseur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fournisseur status={} : {}", self.status, self.corps)
    }
}
impl std::error::Error for ErreurFournisseur {}

/// The source of model responses.
#[async_trait]
pub trait Fournisseur: Send + Sync {
    /// One full call: messages + tool schemas -> aggregated response.
    async fn repondre(
        &self,
        messages: &[Message],
        schemas: &[serde_json::Value],
    ) -> Result<ReponseModele, ErreurFournisseur>;
}
