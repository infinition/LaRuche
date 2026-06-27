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

/// Une **pièce jointe** multimodale portée par un message utilisateur : image (vision),
/// audio (modèles audio) ou fichier. Neutre vis-à-vis du provider — l'adaptateur la traduit
/// (ex. Ollama : `images: [base64]` pour les images, `attachments: [...]` pour le reste).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Piece {
    /// `"image"` | `"audio"` | `"file"`.
    pub kind: String,
    /// Type MIME (ex. `image/png`, `audio/wav`).
    #[serde(default)]
    pub mime: String,
    /// Données encodées en base64.
    pub data: String,
}

impl Piece {
    pub fn est_image(&self) -> bool {
        self.kind == "image"
    }
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
    /// Pièces jointes multimodales (images multiples, audio…) — uniquement sur un message
    /// utilisateur. Vide pour tout le reste.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pieces: Vec<Piece>,
}

impl Message {
    pub fn systeme(c: impl Into<String>) -> Self {
        Self { role: Role::Systeme, contenu: c.into(), outil: None, interne: false, pieces: Vec::new() }
    }
    pub fn utilisateur(c: impl Into<String>) -> Self {
        Self { role: Role::Utilisateur, contenu: c.into(), outil: None, interne: false, pieces: Vec::new() }
    }
    /// Message utilisateur avec pièces jointes multimodales (images/audio).
    pub fn utilisateur_multimodal(c: impl Into<String>, pieces: Vec<Piece>) -> Self {
        Self { role: Role::Utilisateur, contenu: c.into(), outil: None, interne: false, pieces }
    }
    pub fn assistant(c: impl Into<String>) -> Self {
        Self { role: Role::Assistant, contenu: c.into(), outil: None, interne: false, pieces: Vec::new() }
    }
    pub fn observation(outil: impl Into<String>, c: impl Into<String>) -> Self {
        Self { role: Role::Observation, contenu: c.into(), outil: Some(outil.into()), interne: false, pieces: Vec::new() }
    }
    /// Nudge de steering : rôle utilisateur (le modèle le suit), mais marqué interne
    /// → non persisté, non affiché.
    pub fn nudge(c: impl Into<String>) -> Self {
        Self { role: Role::Utilisateur, contenu: c.into(), outil: None, interne: true, pieces: Vec::new() }
    }
}
