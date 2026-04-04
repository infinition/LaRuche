//! Model capability differentiation for the Miel protocol.
//!
//! Each LaRuche node advertises its capabilities so clients can route
//! requests to the appropriate node and model type.

use serde::{Deserialize, Serialize};
use std::fmt;

/// All supported AI capability types in the Miel protocol.
///
/// The protocol natively differentiates model types so clients
/// can intelligently route requests to the right node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capability {
    /// Large Language Model - text to text (Mistral, Llama, Gemma)
    Llm,
    /// Vision-Language Model - image + text understanding (LLaVA, Qwen-VL)
    Vlm,
    /// Vision-Language-Action - robotics (FluidVLA, RT-2)
    Vla,
    /// Retrieval Augmented Generation - document Q&A pipeline
    Rag,
    /// Generic audio processing (legacy, prefer Stt/Tts)
    Audio,
    /// Image generation or analysis (Stable Diffusion, SDXL)
    Image,
    /// Vector embeddings (BGE, E5)
    Embed,
    /// Code generation and analysis (DeepSeek-Coder, CodeLlama)
    Code,
    /// Agent node — capable of running Abeilles (Essaim tools)
    Agent,
    /// Speech-to-Text (Voxtral, Whisper)
    Stt,
    /// Text-to-Speech (Kokoro, Voxtral TTS)
    Tts,
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Llm => write!(f, "llm"),
            Self::Vlm => write!(f, "vlm"),
            Self::Vla => write!(f, "vla"),
            Self::Rag => write!(f, "rag"),
            Self::Audio => write!(f, "audio"),
            Self::Image => write!(f, "image"),
            Self::Embed => write!(f, "embed"),
            Self::Code => write!(f, "code"),
            Self::Agent => write!(f, "agent"),
            Self::Stt => write!(f, "stt"),
            Self::Tts => write!(f, "tts"),
        }
    }
}

impl Capability {
    /// Parse a capability from its Miel flag string.
    pub fn from_flag(flag: &str) -> Option<Self> {
        match flag.to_lowercase().as_str() {
            "llm" => Some(Self::Llm),
            "vlm" => Some(Self::Vlm),
            "vla" => Some(Self::Vla),
            "rag" => Some(Self::Rag),
            "audio" => Some(Self::Audio),
            "image" => Some(Self::Image),
            "embed" => Some(Self::Embed),
            "code" => Some(Self::Code),
            "agent" => Some(Self::Agent),
            "stt" => Some(Self::Stt),
            "tts" => Some(Self::Tts),
            _ => None,
        }
    }

    /// Return the Miel protocol flag for this capability.
    pub fn as_flag(&self) -> String {
        format!("capability:{self}")
    }

    /// Human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Llm => "Text-to-Text (Large Language Model)",
            Self::Vlm => "Vision + Language Understanding",
            Self::Vla => "Vision-Language-Action (Robotics)",
            Self::Rag => "Retrieval Augmented Generation",
            Self::Audio => "Speech-to-Text / Text-to-Speech (generic)",
            Self::Image => "Image Generation / Analysis",
            Self::Embed => "Vector Embeddings",
            Self::Code => "Code Generation / Analysis",
            Self::Agent => "Agentic Node (Essaim tools)",
            Self::Stt => "Speech-to-Text",
            Self::Tts => "Text-to-Speech",
        }
    }
}

/// A set of capabilities with metadata about each.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub capabilities: Vec<CapabilityInfo>,
}

/// Detailed info about a single capability on this node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityInfo {
    pub capability: Capability,
    pub model_name: String,
    pub model_size: Option<String>,
    pub quantization: Option<String>,
    pub max_context_length: Option<u32>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self {
            capabilities: Vec::new(),
        }
    }

    pub fn add(&mut self, info: CapabilityInfo) {
        self.capabilities.push(info);
    }

    /// Check if this set has a specific capability.
    pub fn has(&self, cap: Capability) -> bool {
        self.capabilities.iter().any(|c| c.capability == cap)
    }

    /// Check if this set has ALL of the requested capabilities.
    pub fn has_all(&self, caps: &[Capability]) -> bool {
        caps.iter().all(|c| self.has(*c))
    }

    /// Get the Miel flags for mDNS TXT record.
    pub fn to_flags(&self) -> Vec<String> {
        self.capabilities.iter().map(|c| c.capability.as_flag()).collect()
    }
}

impl Default for CapabilitySet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_roundtrip() {
        let cap = Capability::Vlm;
        assert_eq!(cap.to_string(), "vlm");
        assert_eq!(Capability::from_flag("vlm"), Some(Capability::Vlm));
        assert_eq!(cap.as_flag(), "capability:vlm");
    }

    #[test]
    fn test_capability_set() {
        let mut set = CapabilitySet::new();
        set.add(CapabilityInfo {
            capability: Capability::Llm,
            model_name: "mistral-7b".into(),
            model_size: Some("7B".into()),
            quantization: Some("Q4_K_M".into()),
            max_context_length: Some(8192),
        });
        set.add(CapabilityInfo {
            capability: Capability::Rag,
            model_name: "bge-small".into(),
            model_size: None,
            quantization: None,
            max_context_length: None,
        });

        assert!(set.has(Capability::Llm));
        assert!(set.has(Capability::Rag));
        assert!(!set.has(Capability::Vlm));
        assert!(set.has_all(&[Capability::Llm, Capability::Rag]));
        assert!(!set.has_all(&[Capability::Llm, Capability::Vlm]));
    }
}
