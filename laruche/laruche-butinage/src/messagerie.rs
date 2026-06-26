//! La messagerie : l'historique de conversation d'un butinage.
//!
//! Type minimal et neutre vis-à-vis du provider. Les adaptateurs ([`crate::fournisseur`])
//! le traduisent au format OpenAI/Anthropic/Ollama. Sérialisable → vit dans le [`Carnet`].

use serde::{Deserialize, Serialize};

/// Rôle d'un message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Systeme,
    Utilisateur,
    Assistant,
    /// Résultat d'outil réinjecté (observation).
    Observation,
}

/// Un message de la conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub contenu: String,
    /// Nom de l'outil pour une observation (sinon `None`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub outil: Option<String>,
    /// Message **interne** (nudge de steering, reprise…) : le modèle le voit dans le contexte,
    /// mais il ne doit PAS être persisté/affiché à l'utilisateur (sinon il réapparaît au reload).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub interne: bool,
}

impl Message {
    pub fn systeme(c: impl Into<String>) -> Self {
        Self { role: Role::Systeme, contenu: c.into(), outil: None, interne: false }
    }
    pub fn utilisateur(c: impl Into<String>) -> Self {
        Self { role: Role::Utilisateur, contenu: c.into(), outil: None, interne: false }
    }
    pub fn assistant(c: impl Into<String>) -> Self {
        Self { role: Role::Assistant, contenu: c.into(), outil: None, interne: false }
    }
    pub fn observation(outil: impl Into<String>, c: impl Into<String>) -> Self {
        Self { role: Role::Observation, contenu: c.into(), outil: Some(outil.into()), interne: false }
    }
    /// Nudge de steering : rôle utilisateur (le modèle le suit), mais marqué interne
    /// → non persisté, non affiché.
    pub fn nudge(c: impl Into<String>) -> Self {
        Self { role: Role::Utilisateur, contenu: c.into(), outil: None, interne: true }
    }
}
