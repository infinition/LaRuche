//! Couche d'embeddings pour la recherche sémantique (T1 de la fusion).
//!
//! Un trait [`Embedder`] minimal : n'importe quel fournisseur (Ollama aujourd'hui,
//! `fastembed`/ONNX demain pour le mono-binaire) l'implémente. Le [`SqliteBackend`]
//! l'utilise pour passer d'un recall purement lexical à un recall **sémantique**
//! (vocabulaire-indépendant).

use anyhow::{anyhow, Result};
use async_trait::async_trait;

/// Produit un vecteur d'embedding pour un texte.
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

/// Similarité cosinus (0 si dimensions incompatibles ou vecteurs nuls).
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

/// Embedder via Ollama (`/api/embed`). Réutilise le pattern de `laruche-essaim::rag`.
/// Mono-binaire compatible : Ollama est un service externe optionnel, comme déjà supposé par LaRuche.
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
        // Format Ollama récent : {"embeddings":[[...]]} ; ancien : {"embedding":[...]}
        let arr = body["embeddings"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.as_array())
            .or_else(|| body["embedding"].as_array())
            .ok_or_else(|| anyhow!("réponse embed inattendue"))?;
        Ok(arr
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect())
    }
}
