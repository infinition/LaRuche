//! Le **fournisseur** de réponses du modèle (LLM), abstrait par un trait.
//!
//! Inversion de dépendances : `laruche-butinage` ne connaît pas les providers
//! concrets. L'adaptateur (dans `laruche-essaim`) implémente ce trait et gère en
//! interne le streaming, la rotation de clés (`credential_pool`) et le déroutement
//! modèle ; il ne surface qu'une réponse agrégée ou une erreur terminale. La boucle
//! applique par-dessus la politique [`crate::meteo`] (backoff, abandon).

use crate::issue::{Appel, StopReason};
use crate::messagerie::Message;
use async_trait::async_trait;

/// Consommation de tokens (réelle si le provider la fournit).
#[derive(Debug, Clone, Copy, Default)]
pub struct Usage {
    pub entree: u32,
    pub sortie: u32,
}

/// Réponse agrégée d'un appel au modèle.
#[derive(Debug, Clone)]
pub struct ReponseModele {
    pub texte: String,
    pub stop: StopReason,
    /// Appels d'outils émis par le modèle (natifs API ou parsés par l'adaptateur).
    pub appels: Vec<Appel>,
    pub usage: Option<Usage>,
}

/// Erreur d'un appel modèle, portant de quoi la classer ([`crate::meteo::ClasseErreur`]).
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

/// La source de réponses du modèle.
#[async_trait]
pub trait Fournisseur: Send + Sync {
    /// Un appel complet : messages + schémas d'outils → réponse agrégée.
    async fn repondre(
        &self,
        messages: &[Message],
        schemas: &[serde_json::Value],
    ) -> Result<ReponseModele, ErreurFournisseur>;
}
