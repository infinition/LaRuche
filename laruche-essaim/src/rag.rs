//! RAG (Retrieval-Augmented Generation) — Knowledge base with vector search.
//!
//! Stores text chunks with their embeddings (via Ollama /api/embed).
//! Searches by cosine similarity to find relevant context.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A single entry in the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: String,
    pub text: String,
    pub source: Option<String>,
    pub embedding: Vec<f32>,
    pub created_at: String,
}

/// The knowledge base — stores entries with embeddings for vector search.
#[derive(Debug, Serialize, Deserialize)]
pub struct KnowledgeBase {
    pub entries: Vec<KnowledgeEntry>,
    #[serde(skip)]
    file_path: PathBuf,
    #[serde(skip)]
    ollama_url: String,
    #[serde(skip)]
    embed_model: String,
}

impl KnowledgeBase {
    /// Create or load a knowledge base.
    pub fn new(file_path: &Path, ollama_url: &str, embed_model: &str) -> Self {
        let mut kb = if file_path.exists() {
            match std::fs::read_to_string(file_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| KnowledgeBase {
                    entries: Vec::new(),
                    file_path: file_path.to_path_buf(),
                    ollama_url: ollama_url.to_string(),
                    embed_model: embed_model.to_string(),
                }),
                Err(_) => KnowledgeBase {
                    entries: Vec::new(),
                    file_path: file_path.to_path_buf(),
                    ollama_url: ollama_url.to_string(),
                    embed_model: embed_model.to_string(),
                },
            }
        } else {
            KnowledgeBase {
                entries: Vec::new(),
                file_path: file_path.to_path_buf(),
                ollama_url: ollama_url.to_string(),
                embed_model: embed_model.to_string(),
            }
        };
        kb.file_path = file_path.to_path_buf();
        kb.ollama_url = ollama_url.to_string();
        kb.embed_model = embed_model.to_string();
        tracing::info!(entries = kb.entries.len(), "Knowledge base loaded");
        kb
    }

    /// Add a text to the knowledge base (generates embedding via Ollama).
    pub async fn add(&mut self, text: &str, source: Option<&str>) -> Result<String> {
        let embedding = self.get_embedding(text).await?;
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();

        self.entries.push(KnowledgeEntry {
            id: id.clone(),
            text: text.to_string(),
            source: source.map(|s| s.to_string()),
            embedding,
            created_at: chrono::Utc::now().to_rfc3339(),
        });

        self.save()?;
        Ok(id)
    }

    /// Search for the most relevant entries given a query.
    pub async fn search(&self, query: &str, top_k: usize) -> Result<Vec<(f32, &KnowledgeEntry)>> {
        if self.entries.is_empty() {
            return Ok(vec![]);
        }

        let query_embedding = self.get_embedding(query).await?;

        let mut scored: Vec<(f32, &KnowledgeEntry)> = self.entries.iter()
            .map(|entry| {
                let sim = cosine_similarity(&query_embedding, &entry.embedding);
                (sim, entry)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        Ok(scored)
    }

    /// Remove an entry by ID.
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        let removed = self.entries.len() < before;
        if removed {
            let _ = self.save();
        }
        removed
    }

    /// Get an embedding from Ollama.
    async fn get_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/api/embed", self.ollama_url))
            .json(&serde_json::json!({
                "model": self.embed_model,
                "input": text,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Ollama embed error: {}", resp.status());
        }

        let body: serde_json::Value = resp.json().await?;

        // Ollama returns {"embeddings": [[...]]} for /api/embed
        if let Some(embeddings) = body["embeddings"].as_array() {
            if let Some(first) = embeddings.first() {
                if let Some(arr) = first.as_array() {
                    return Ok(arr.iter()
                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                        .collect());
                }
            }
        }

        // Fallback: try older format {"embedding": [...]}
        if let Some(arr) = body["embedding"].as_array() {
            return Ok(arr.iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect());
        }

        anyhow::bail!("Unexpected embedding response format")
    }

    fn save(&self) -> Result<()> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string(self)?;
        std::fs::write(&self.file_path, json)?;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
