//! Write-time contradiction arbiter for the cognitive memory (aux LLM).
//!
//! Implements [`laruche_memoire::Arbitre`]: when a new fact is moderately similar to
//! an existing one (cosine band the backend cannot decide, e.g. an UPDATE like
//! "RTX 4070 Ti" -> "RTX 5080" that measures ~0.71), the backend asks this arbiter
//! whether the new fact REPLACES the old one. A one-shot classification call on the
//! auxiliary model; any failure defaults to keeping BOTH facts (never destructive).

use async_trait::async_trait;
use futures_util::StreamExt;
use laruche_essaim::brain::EssaimConfig;
use laruche_memoire::{Arbitre, VerdictArbitre};

pub struct ArbitreLLM {
    provider: String,
    model: String,
    api_key: String,
    api_base: Option<String>,
    ollama_url: String,
}

impl ArbitreLLM {
    /// Snapshots the aux-model settings from the essaim config. Prefers `aux_model`
    /// (small/fast) over the main chat model so arbitration never competes with the
    /// chat KV-cache.
    pub fn depuis_config(config: &EssaimConfig) -> Self {
        Self {
            provider: config.provider.clone(),
            model: config.aux_model.clone().unwrap_or_else(|| config.model.clone()),
            api_key: config.api_key.clone(),
            api_base: config.api_base.clone(),
            ollama_url: config.ollama_url.clone(),
        }
    }
}

const PROMPT: &str = "You compare two short memory facts about the SAME subject. Decide if \
the NEW fact REPLACES the OLD one (an update, correction, or paraphrase of the same attribute) \
or if they are DISTINCT facts that merely share vocabulary. Answer with a SINGLE word: \
REPLACE or DISTINCT. No punctuation, no explanation.";

#[async_trait]
impl Arbitre for ArbitreLLM {
    async fn trancher(&self, existant: &str, nouveau: &str) -> VerdictArbitre {
        let user = format!("OLD: {existant}\nNEW: {nouveau}\nAnswer:");
        let messages = vec![
            serde_json::json!({ "role": "system", "content": PROMPT }),
            serde_json::json!({ "role": "user", "content": user }),
        ];
        let key = laruche_essaim::secrets::substituer(&self.api_key);
        let stream = laruche_essaim::providers::provider_chat_stream(
            &self.provider,
            &self.model,
            &messages,
            0.0,
            16,
            &key,
            self.api_base.as_deref(),
            &self.ollama_url,
            None,
        )
        .await;
        let mut texte = String::new();
        if let Ok(mut s) = stream {
            while let Some(chunk) = s.next().await {
                texte.push_str(&chunk.text);
            }
        }
        // Default DISTINCT: an unreachable model or ambiguous answer must NEVER delete
        // a fact. Only an explicit REPLACE supersedes.
        if texte.to_uppercase().contains("REPLACE") {
            VerdictArbitre::Remplace
        } else {
            VerdictArbitre::Distinct
        }
    }
}
