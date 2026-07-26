//! Embedding layer for semantic search (T1 of the fusion).
//!
//! A minimal [`Embedder`] trait: any provider implements it. The [`SqliteBackend`]
//! uses it to move from purely lexical recall to **semantic** recall.
//!
//! [`HttpEmbedder`] is the universal implementation: it auto-detects the endpoint
//! format - Ollama (`/api/embed`) or OpenAI-compatible (`/v1/embeddings`, e.g. a
//! llama.cpp `llama-server --embeddings`) - remembers what worked, and opens a
//! CIRCUIT BREAKER when the server is down so a dead embedder costs one failed
//! probe every few minutes instead of a timeout per memory operation.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::atomic::{AtomicI64, AtomicU8, Ordering};

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

const MODE_INCONNU: u8 = 0;
const MODE_OLLAMA: u8 = 1;
const MODE_OPENAI: u8 = 2;
/// Circuit-breaker cooldown after consecutive failures (seconds).
const DISJONCTEUR_SECS: i64 = 300;

/// Universal HTTP embedder: Ollama `/api/embed` or OpenAI-compat `/v1/embeddings`
/// (llama.cpp llama-server, LM Studio...). Format detected on first success.
pub struct HttpEmbedder {
    client: reqwest::Client,
    url: String,
    model: String,
    /// Detected wire format (sticky after first success).
    mode: AtomicU8,
    /// Epoch seconds until which the breaker is open (0 = closed).
    down_until: AtomicI64,
}

impl HttpEmbedder {
    pub fn new(url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(8))
                .build()
                .unwrap_or_default(),
            url: url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            mode: AtomicU8::new(MODE_INCONNU),
            down_until: AtomicI64::new(0),
        }
    }

    async fn essayer_ollama(&self, text: &str) -> Result<Vec<f32>> {
        let resp = self
            .client
            .post(format!("{}/api/embed", self.url))
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
            .ok_or_else(|| anyhow!("unexpected ollama embed response"))?;
        Ok(arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
    }

    async fn essayer_openai(&self, text: &str) -> Result<Vec<f32>> {
        let resp = self
            .client
            .post(format!("{}/v1/embeddings", self.url))
            .json(&serde_json::json!({ "model": self.model, "input": text }))
            .send()
            .await?;
        let body: serde_json::Value = resp.json().await?;
        let arr = body["data"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|d| d["embedding"].as_array())
            .ok_or_else(|| anyhow!("unexpected openai embed response"))?;
        Ok(arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
    }
}

#[async_trait]
impl Embedder for HttpEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let now = chrono::Utc::now().timestamp();
        if self.down_until.load(Ordering::Relaxed) > now {
            return Err(anyhow!("embedder circuit open (server down, retrying later)"));
        }
        let mode = self.mode.load(Ordering::Relaxed);
        let res = match mode {
            MODE_OLLAMA => self.essayer_ollama(text).await,
            MODE_OPENAI => self.essayer_openai(text).await,
            _ => match self.essayer_ollama(text).await {
                Ok(v) => {
                    self.mode.store(MODE_OLLAMA, Ordering::Relaxed);
                    Ok(v)
                }
                Err(_) => match self.essayer_openai(text).await {
                    Ok(v) => {
                        self.mode.store(MODE_OPENAI, Ordering::Relaxed);
                        Ok(v)
                    }
                    Err(e) => Err(e),
                },
            },
        };
        match &res {
            Ok(v) if !v.is_empty() => {
                self.down_until.store(0, Ordering::Relaxed);
            }
            _ => {
                // Server unreachable/mute: open the breaker so memory ops stay fast.
                self.down_until
                    .store(now + DISJONCTEUR_SECS, Ordering::Relaxed);
            }
        }
        res.and_then(|v| {
            if v.is_empty() {
                Err(anyhow!("empty embedding"))
            } else {
                Ok(v)
            }
        })
    }
}

/// Embedder via Ollama (kept for API compatibility; prefer [`HttpEmbedder`]).
pub type OllamaEmbedder = HttpEmbedder;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_basique() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert_eq!(cosine(&[], &[]), 0.0);
        assert_eq!(cosine(&[1.0], &[1.0, 2.0]), 0.0);
    }

    #[tokio::test]
    async fn disjoncteur_ouvre_apres_echec() {
        // Nothing listens on this port: the first call fails (fast), the second is
        // rejected instantly by the open breaker.
        let e = HttpEmbedder::new("http://127.0.0.1:9", "x");
        assert!(e.embed("test").await.is_err());
        let t0 = std::time::Instant::now();
        assert!(e.embed("test").await.is_err());
        assert!(t0.elapsed().as_millis() < 100, "breaker must reject instantly");
    }
}
