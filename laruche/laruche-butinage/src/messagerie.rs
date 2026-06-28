//! The messagerie: the conversation history of a butinage.
//!
//! Minimal, provider-neutral type. The adapters ([`crate::fournisseur`])
//! translate it to the OpenAI/Anthropic/Ollama format. Serializable, lives in the [`Carnet`].

use serde::{Deserialize, Serialize};

/// Role of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Systeme,
    Utilisateur,
    Assistant,
    /// Reinjected tool result (observation).
    Observation,
}

/// A multimodal **attachment** carried by a user message: image (vision),
/// audio (audio models) or file. Provider-neutral, the adapter translates it
/// (e.g. Ollama: `images: [base64]` for images, `attachments: [...]` for the rest).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Piece {
    /// `"image"` | `"audio"` | `"file"`.
    pub kind: String,
    /// MIME type (e.g. `image/png`, `audio/wav`).
    #[serde(default)]
    pub mime: String,
    /// Base64-encoded data.
    pub data: String,
}

impl Piece {
    pub fn est_image(&self) -> bool {
        self.kind == "image"
    }
}

/// A message in the conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub contenu: String,
    /// Tool name for an observation (otherwise `None`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub outil: Option<String>,
    /// **Internal** message (steering nudge, resume...): the model sees it in the context,
    /// but it must NOT be persisted/shown to the user (otherwise it reappears on reload).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub interne: bool,
    /// Multimodal attachments (multiple images, audio...): only on a user
    /// message. Empty for everything else.
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
    /// User message with multimodal attachments (images/audio).
    pub fn utilisateur_multimodal(c: impl Into<String>, pieces: Vec<Piece>) -> Self {
        Self { role: Role::Utilisateur, contenu: c.into(), outil: None, interne: false, pieces }
    }
    pub fn assistant(c: impl Into<String>) -> Self {
        Self { role: Role::Assistant, contenu: c.into(), outil: None, interne: false, pieces: Vec::new() }
    }
    pub fn observation(outil: impl Into<String>, c: impl Into<String>) -> Self {
        Self { role: Role::Observation, contenu: c.into(), outil: Some(outil.into()), interne: false, pieces: Vec::new() }
    }
    /// Steering nudge: user role (the model follows it), but marked internal
    /// so it is not persisted, not displayed.
    pub fn nudge(c: impl Into<String>) -> Self {
        Self { role: Role::Utilisateur, contenu: c.into(), outil: None, interne: true, pieces: Vec::new() }
    }
}
