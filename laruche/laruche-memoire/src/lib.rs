//! # laruche-memoire: the cognitive memory of LaRuche
//!
//! A **single interface** ([`MemoireCognitive`]) that the agent engine (`brain.rs`)
//! consumes, with **several interchangeable backends** behind it:
//!
//! - [`SidecarBackend`]: talks to `paradigm serve` via JSON-RPC over the loopback
//!   HTTP bridge (`:8765`). Quick prototype, see `sidecar.rs`.
//! - `NativeBackend` (upcoming, P3): pure Rust port of the paradigm engine (rusqlite + FTS5
//!   + embeddings) -> single binary.
//!
//! `brain.rs` only knows the trait. Migrating from sidecar to native is a drop-in swap:
//! no agent code changes.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod curator;
mod embed;
mod native;
mod sidecar;
mod sqlite;
pub use curator::{maybe_run_curator, Curator, CuratorState};
pub use embed::{cosine, Embedder, HttpEmbedder, OllamaEmbedder};
pub use native::NativeBackend;
pub use sidecar::{SidecarBackend, SidecarConfig};
pub use sqlite::SqliteBackend;

/// A memory item to write into the cognitive map.
///
/// Mirrors the paradigm schema (`memory_write` / `memory_propose_write`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    /// Dotted node identifier, e.g. `projects.laruche`, `decisions.archi`, `people.fabien`.
    pub node_id: String,
    /// The fact/text to memorize.
    pub content: String,
    // `default` is REQUIRED alongside `skip_serializing_if`: an empty tags vec is
    // omitted on serialize, so without a default it fails to round-trip ("missing
    // field tags") when a proposal's stored MemoryItem is deserialized on approval.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importance: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
}

impl MemoryItem {
    /// Builds a minimal item (node + content).
    pub fn new(node_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            content: content.into(),
            tags: Vec::new(),
            source: None,
            importance: None,
            confidence: None,
        }
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// Verdict of the contradiction arbiter comparing an existing fact with a new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictArbitre {
    /// Same or superseding fact (paraphrase, or an UPDATE like 4070 -> 5080):
    /// the existing item is retired (`superseded`) in favor of the new one.
    Remplace,
    /// Unrelated facts that merely share vocabulary: keep both.
    Distinct,
}

/// Resolves near-miss contradictions at write time. Cosine similarity catches
/// paraphrases (>0.83) but NOT semantic updates ("4070 Ti" vs "5080" measure ~0.71),
/// which look moderately similar yet contradict. In the ambiguity band the backend
/// asks this arbiter (an aux LLM, wired by the node) whether the new fact REPLACES
/// the old one. Dependency inversion: `laruche-memoire` stays provider-agnostic.
#[async_trait]
pub trait Arbitre: Send + Sync {
    async fn trancher(&self, existant: &str, nouveau: &str) -> VerdictArbitre;
}

/// Cognitive search options.
#[derive(Debug, Clone, Default)]
pub struct SearchOpts {
    /// Expansion depth in the map (0..=4).
    pub depth: Option<u8>,
    /// Max number of evidence items.
    pub limit: Option<u8>,
}

/// Search result: the raw "context pack" returned by the engine,
/// plus a text rendering ready to inject into the prompt.
#[derive(Debug, Clone)]
pub struct ContextPack {
    /// Raw engine response (activated nodes, evidence items, token budget...).
    pub raw: Value,
}

impl ContextPack {
    /// Renders the pack as compact text to inject into the system prompt.
    ///
    /// Best-effort: extracts known evidence items, otherwise falls back to a
    /// truncated compact JSON. (Refined in P2 when wired into `brain.rs`.)
    pub fn to_prompt_text(&self) -> String {
        let mut out = String::new();

        // Activated nodes (one-liners).
        if let Some(nodes) = self.raw.get("nodes").and_then(Value::as_array) {
            for n in nodes {
                if let Some(label) = n
                    .get("id")
                    .or_else(|| n.get("label"))
                    .and_then(Value::as_str)
                {
                    let one = n.get("one_liner").and_then(Value::as_str).unwrap_or("");
                    out.push_str(&format!("• {label} - {one}\n"));
                }
            }
        }

        // Evidence items (the actual content).
        let items = self
            .raw
            .get("items")
            .or_else(|| self.raw.get("evidence"))
            .and_then(Value::as_array);
        if let Some(items) = items {
            for it in items {
                if let Some(content) = it
                    .get("content")
                    .or_else(|| it.get("text"))
                    .and_then(Value::as_str)
                {
                    out.push_str(&format!("- {content}\n"));
                }
            }
        }

        if out.trim().is_empty() {
            // Fallback: truncated compact JSON.
            let pretty = serde_json::to_string(&self.raw).unwrap_or_default();
            out = pretty.chars().take(2000).collect();
        }
        out.trim().to_string()
    }
}

/// The single cognitive memory interface consumed by the agent engine.
///
/// All backends (paradigm sidecar, native Rust) implement it. See the module-level doc.
#[async_trait]
pub trait MemoireCognitive: Send + Sync {
    /// Retrieval: map activation + hybrid retrieval -> context pack.
    async fn search(&self, query: &str, opts: SearchOpts) -> Result<ContextPack>;

    /// Direct write of an active item (trusted caller). Audited.
    async fn write(&self, item: MemoryItem) -> Result<Value>;

    /// Proposed write (status `proposed`): goes through human review. Audited.
    async fn propose_write(&self, item: MemoryItem) -> Result<Value>;

    /// Reads a node, its direct children and its items.
    async fn read_node(&self, node_id: &str) -> Result<Value>;

    /// Lists ALL nodes of the cognitive map (for the Obsidian-style Memory tree).
    /// Each node: `{id, node_id, parent_id?, label, one_liner}`. Default: empty.
    async fn list_nodes(&self) -> Result<Value> {
        Ok(serde_json::json!([]))
    }

    /// Deletes a node; its items (and sub-nodes) are reattached to the parent. For
    /// merging/cleaning up duplicate or generic nodes. Audited. Default: unsupported.
    async fn delete_node(&self, _node_id: &str) -> Result<Value> {
        Err(anyhow!("memory_delete_node unsupported by this backend"))
    }

    /// Creates a node (the parent, if any, is created as needed). `node_id` is dotted
    /// snake_case, e.g. `projects.football_bot`. Audited. Default: unsupported.
    async fn create_node(
        &self,
        _node_id: &str,
        _label: &str,
        _one_liner: Option<&str>,
        _importance: Option<f32>,
        _source: Option<&str>,
    ) -> Result<Value> {
        Err(anyhow!("memory_create_node unsupported by this backend"))
    }

    /// Updates the label / one-liner / importance of an existing node (renaming a generic
    /// node into a meaningful one). Audited. Default: unsupported.
    async fn update_node(
        &self,
        _node_id: &str,
        _label: Option<&str>,
        _one_liner: Option<&str>,
        _importance: Option<f32>,
    ) -> Result<Value> {
        Err(anyhow!("memory_update_node unsupported by this backend"))
    }

    /// Migration: renames the whole `old_prefix.*` subtree (and the `old_prefix` node) to
    /// `new_prefix.*`, preserving items and hierarchy. Returns the number of moved nodes.
    /// Idempotent (no-op if nothing remains under `old_prefix`). Default: unsupported.
    async fn renommer_sous_arbre(&self, _old_prefix: &str, _new_prefix: &str) -> Result<usize> {
        Err(anyhow!("renommer_sous_arbre unsupported by this backend"))
    }

    /// Migration: permanently deletes the whole `prefix.*` subtree (and the `prefix` node).
    /// For purging a regenerable projection. Returns the number of deleted nodes. Default: 0.
    async fn supprimer_sous_arbre(&self, _prefix: &str) -> Result<usize> {
        Ok(0)
    }

    /// Consolidation pass (duplicates, stale, overloaded, orphans). Applies nothing.
    async fn update_item(&self, _item_id: &str, _content: &str) -> Result<Value> {
        Err(anyhow!("memory_update_item unsupported by this backend"))
    }

    async fn move_item(&self, _item_id: &str, _node_id: &str) -> Result<Value> {
        Err(anyhow!("memory_move_item unsupported by this backend"))
    }

    async fn delete_item(&self, _item_id: &str, _reason: Option<&str>) -> Result<Value> {
        Err(anyhow!("memory_delete unsupported by this backend"))
    }

    async fn review_item(
        &self,
        _item_id: &str,
        _action: &str,
        _reason: Option<&str>,
    ) -> Result<Value> {
        Err(anyhow!("memory_review unsupported by this backend"))
    }

    async fn list_proposed(&self, _limit: Option<u8>) -> Result<Value> {
        Err(anyhow!("memory_list_proposed unsupported by this backend"))
    }

    async fn suggest_nodes(&self, _query: &str, _limit: Option<u8>) -> Result<Value> {
        Err(anyhow!("memory_suggest_nodes unsupported by this backend"))
    }

    /// Memory statistics (item/node/mutation counters).
    async fn stats(&self) -> Result<Value> {
        Err(anyhow!("memory_stats unsupported by this backend"))
    }

    /// Audit log: recent mutations (write/update/delete/review/move...).
    async fn mutations(&self, _limit: Option<u8>) -> Result<Value> {
        Err(anyhow!("memory_mutations unsupported by this backend"))
    }

    /// SUBSTRING search (case-insensitive) in the content of active items:
    /// "memory grep". Returns `[{id, node_id, content}]`. Default: unsupported.
    async fn grep(&self, _pattern: &str, _limit: Option<u8>) -> Result<Value> {
        Err(anyhow!("memory_grep unsupported by this backend"))
    }

    async fn dream(&self) -> Result<Value>;

    /// Exports the memory as an **OKF bundle** (Open Knowledge Format, Google): a tree of
    /// Markdown files + YAML frontmatter (`type` required). If `prefix` is `Some`,
    /// exports only that node and its subtree (otherwise the whole map). Returns the file count.
    async fn export_okf(&self, _dir: &std::path::Path, _prefix: Option<&str>) -> Result<usize> {
        Err(anyhow!("export_okf unsupported by this backend"))
    }

    /// Imports an **OKF bundle** (Markdown+YAML folder) into the memory. Returns the number of imported items.
    async fn import_okf(&self, _dir: &std::path::Path) -> Result<usize> {
        Err(anyhow!("import_okf unsupported by this backend"))
    }

    /// Backfills missing embeddings for active items (items written while the
    /// embedder was down would otherwise stay invisible to semantic recall).
    /// No-op without an embedder. Returns the number of items embedded.
    async fn backfill_embeddings(&self, _max: usize) -> Result<usize> {
        Ok(0)
    }

    /// Wires the write-time contradiction arbiter (aux LLM). Default: no-op
    /// (backends without an arbiter keep pure cosine-based supersede).
    fn definir_arbitre(&self, _arbitre: std::sync::Arc<dyn Arbitre>) {}

    /// Checks that the backend responds.
    async fn health(&self) -> Result<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_item_round_trips_with_empty_tags() {
        // A curateur item with no tags must survive serialize -> deserialize, otherwise
        // approving a queued proposal fails ("missing field tags").
        let item = MemoryItem::new("people.fabien", "User's name is Fabien");
        let json = serde_json::to_string(&item).unwrap();
        assert!(!json.contains("tags")); // empty tags is omitted
        let back: MemoryItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.node_id, "people.fabien");
        assert!(back.tags.is_empty());
    }
}
