//! The messagerie: the conversation history of a butinage.
//!
//! Minimal, provider-neutral type. The adapters ([`crate::fournisseur`])
//! translate it to the OpenAI/Anthropic/Ollama format. Serializable, lives in the [`Carnet`].

use crate::issue::Appel;
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
    /// [`crate::carnet::Carnet::sauver`] filters these out of the checkpoint.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub interne: bool,
    /// Multimodal attachments (multiple images, audio...): only on a user
    /// message. Empty for everything else.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pieces: Vec<Piece>,
    /// Tool calls emitted with this assistant message. Keeping them in the history lets
    /// the adapter reconstruct a coherent transcript (the model must see WHICH calls
    /// produced the observations that follow), and survives crash-resume.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub appels: Vec<Appel>,
    /// For an observation: id of the [`Appel`] that produced it (native `tool_result`
    /// correlation for providers that support it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appel_id: Option<String>,
}

impl Message {
    fn base(role: Role, contenu: String) -> Self {
        Self {
            role,
            contenu,
            outil: None,
            interne: false,
            pieces: Vec::new(),
            appels: Vec::new(),
            appel_id: None,
        }
    }

    pub fn systeme(c: impl Into<String>) -> Self {
        Self::base(Role::Systeme, c.into())
    }
    pub fn utilisateur(c: impl Into<String>) -> Self {
        Self::base(Role::Utilisateur, c.into())
    }
    /// User message with multimodal attachments (images/audio).
    pub fn utilisateur_multimodal(c: impl Into<String>, pieces: Vec<Piece>) -> Self {
        Self { pieces, ..Self::base(Role::Utilisateur, c.into()) }
    }
    pub fn assistant(c: impl Into<String>) -> Self {
        Self::base(Role::Assistant, c.into())
    }
    /// Assistant message carrying the tool calls it emitted this turn.
    pub fn assistant_avec_appels(c: impl Into<String>, appels: Vec<Appel>) -> Self {
        Self { appels, ..Self::base(Role::Assistant, c.into()) }
    }
    pub fn observation(outil: impl Into<String>, c: impl Into<String>) -> Self {
        Self { outil: Some(outil.into()), ..Self::base(Role::Observation, c.into()) }
    }
    /// Observation correlated to the tool call that produced it.
    pub fn observation_liee(
        outil: impl Into<String>,
        appel_id: impl Into<String>,
        c: impl Into<String>,
    ) -> Self {
        Self {
            outil: Some(outil.into()),
            appel_id: Some(appel_id.into()),
            ..Self::base(Role::Observation, c.into())
        }
    }
    /// Steering nudge: user role (the model follows it), but marked internal
    /// so it is not persisted, not displayed.
    pub fn nudge(c: impl Into<String>) -> Self {
        Self { interne: true, ..Self::base(Role::Utilisateur, c.into()) }
    }

    /// Estimated cost of this message in CHARACTERS, as seen by the provider.
    /// `contenu.len()` alone is blind to two real payloads:
    /// - `appels`: the serialized tool calls (name + JSON args) travel in the request;
    /// - `pieces`: a base64 image is huge on the wire but its token cost is the
    ///   provider's VISION cost, not its byte count. We charge a bounded heuristic
    ///   (`len/8`, clamped to [2048, 16384] chars ≈ 512–4096 tokens at 4 chars/token);
    ///   the jauge's learned calibration factor absorbs the per-provider remainder.
    /// Single source of truth for the jauge and the truncation guardrail.
    pub fn cout_chars(&self) -> usize {
        let mut n = self.contenu.len();
        for a in &self.appels {
            n += a.nom.len() + a.args.to_string().len() + 24;
        }
        for p in &self.pieces {
            n += (p.data.len() / 8).clamp(2048, 16_384);
        }
        n
    }
}
