//! Embedding layer for semantic search (T1 of the fusion).
//!
//! A minimal [`Embedder`] trait: any provider (Ollama today,
//! `fastembed`/ONNX tomorrow for the single binary) implements it. The [`SqliteBackend`]
//! uses it to move from purely lexical recall to **semantic** recall
//! (vocabulary-independent).

use anyhow::{anyhow, Result};
use async_trait::async_trait;

/// Produces an embedding vector for a text.
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

/// Cosine similarity (0 if dimensions mismatch or vectors are empty/zero).
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

/// Embedder via Ollama (`/api/embed`). Reuses the pattern from `laruche-essaim::rag`.
/// Single-binary compatible: Ollama is an optional external service, as already assumed by LaRuche.
pub struct OllamaEmbedder {
    client: reqwest::Client,
    url: String,
    model: String,
}

impl OllamaEmbedder {
    pub fn new(url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: url.into(),
            model: model.into(),
        }
    }
}

#[async_trait]
impl Embedder for OllamaEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let resp = self
            .client
            .post(format!("{}/api/embed", self.url.trim_end_matches('/')))
            .json(&serde_json::json!({ "model": self.model, "input": text }))
            .send()
            .await?;
        let body: serde_json::Value = resp.json().await?;
        // Recent Ollama format: {"embeddings":[[...]]} ; older: {"embedding":[...]}
        let arr = body["embeddings"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.as_array())
            .or_else(|| body["embedding"].as_array())
            .ok_or_else(|| anyhow!("unexpected embed response"))?;
        Ok(arr
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect())
    }
}
