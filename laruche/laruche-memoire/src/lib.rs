//! # laruche-memoire — la mémoire cognitive de LaRuche
//!
//! Une **seule interface** ([`MemoireCognitive`]) que le moteur d'agent (`brain.rs`)
//! consomme, et **plusieurs backends** interchangeables derrière :
//!
//! - [`SidecarBackend`] : parle à `paradigm serve` en JSON-RPC sur le pont HTTP
//!   loopback (`:8765`). Prototype rapide — voir `sidecar.rs`.
//! - `NativeBackend` (à venir, P3) : port Rust pur du moteur paradigm (rusqlite + FTS5
//!   + embeddings) → mono-binaire.
//!
//! `brain.rs` ne connaît QUE le trait. Migrer du sidecar au natif est un swap drop-in :
//! aucun code d'agent ne change.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;

mod curator;
mod embed;
mod native;
mod sidecar;
mod sqlite;
pub use curator::{maybe_run_curator, Curator, CuratorState};
pub use embed::{cosine, Embedder, OllamaEmbedder};
pub use native::NativeBackend;
pub use sidecar::{SidecarBackend, SidecarConfig};
pub use sqlite::SqliteBackend;

/// Un item de mémoire à écrire dans la carte cognitive.
///
/// Reprend le schéma paradigm (`memory_write` / `memory_propose_write`).
#[derive(Debug, Clone, Serialize)]
pub struct MemoryItem {
    /// Identifiant pointé du nœud, ex. `projects.laruche`, `decisions.archi`, `people.fabien`.
    pub node_id: String,
    /// Le fait/texte à mémoriser.
    pub content: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importance: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
}

impl MemoryItem {
    /// Construit un item minimal (node + contenu).
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

/// Options de recherche cognitive.
#[derive(Debug, Clone, Default)]
pub struct SearchOpts {
    /// Profondeur d'expansion dans la carte (0..=4).
    pub depth: Option<u8>,
    /// Nombre max d'items de preuve.
    pub limit: Option<u8>,
}

/// Résultat d'une recherche : le « context pack » brut renvoyé par le moteur,
/// + un rendu texte prêt à injecter dans le prompt.
#[derive(Debug, Clone)]
pub struct ContextPack {
    /// Réponse brute du moteur (nœuds activés, items de preuve, budget tokens…).
    pub raw: Value,
}

impl ContextPack {
    /// Rend le pack en texte compact à injecter dans le system prompt.
    ///
    /// Best-effort : extrait les items de preuve connus, sinon retombe sur un JSON
    /// compact tronqué. (Affiné en P2 quand on câble dans `brain.rs`.)
    pub fn to_prompt_text(&self) -> String {
        let mut out = String::new();

        // Nœuds activés (one-liners).
        if let Some(nodes) = self.raw.get("nodes").and_then(Value::as_array) {
            for n in nodes {
                if let Some(label) = n
                    .get("id")
                    .or_else(|| n.get("label"))
                    .and_then(Value::as_str)
                {
                    let one = n.get("one_liner").and_then(Value::as_str).unwrap_or("");
                    out.push_str(&format!("• {label} — {one}\n"));
                }
            }
        }

        // Items de preuve (le contenu réel).
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
            // Repli : JSON compact tronqué.
            let pretty = serde_json::to_string(&self.raw).unwrap_or_default();
            out = pretty.chars().take(2000).collect();
        }
        out.trim().to_string()
    }
}

/// L'interface unique de mémoire cognitive consommée par le moteur d'agent.
///
/// Tous les backends (sidecar paradigm, natif Rust) l'implémentent. Voir le module-level doc.
#[async_trait]
pub trait MemoireCognitive: Send + Sync {
    /// Récupération : activation de la carte + retrieval hybride → context pack.
    async fn search(&self, query: &str, opts: SearchOpts) -> Result<ContextPack>;

    /// Écriture directe d'un item actif (appelant de confiance). Audité.
    async fn write(&self, item: MemoryItem) -> Result<Value>;

    /// Écriture proposée (statut `proposed`) — passe par la revue humaine. Audité.
    async fn propose_write(&self, item: MemoryItem) -> Result<Value>;

    /// Lit un nœud, ses enfants directs et ses items.
    async fn read_node(&self, node_id: &str) -> Result<Value>;

    /// Liste TOUS les nœuds de la carte cognitive (pour l'arbre Mémoire type Obsidian).
    /// Chaque nœud : `{id, node_id, parent_id?, label, one_liner}`. Défaut : vide.
    async fn list_nodes(&self) -> Result<Value> {
        Ok(serde_json::json!([]))
    }

    /// Supprime un nœud ; ses items (et sous-nœuds) sont rattachés au parent. Pour
    /// fusionner/nettoyer des nœuds en double ou génériques. Audité. Défaut : non supporté.
    async fn delete_node(&self, _node_id: &str) -> Result<Value> {
        Err(anyhow!("memory_delete_node unsupported by this backend"))
    }

    /// Crée un nœud (le parent, s'il existe, est créé au besoin). `node_id` pointé
    /// snake_case, ex. `projects.football_bot`. Audité. Défaut : non supporté.
    async fn create_node(
        &self,
        _node_id: &str,
        _label: &str,
        _one_liner: Option<&str>,
        _importance: Option<f32>,
    ) -> Result<Value> {
        Err(anyhow!("memory_create_node unsupported by this backend"))
    }

    /// Met à jour le label / one-liner / importance d'un nœud existant (renommer un nœud
    /// générique en nœud parlant). Audité. Défaut : non supporté.
    async fn update_node(
        &self,
        _node_id: &str,
        _label: Option<&str>,
        _one_liner: Option<&str>,
        _importance: Option<f32>,
    ) -> Result<Value> {
        Err(anyhow!("memory_update_node unsupported by this backend"))
    }

    /// Migration : renomme tout le sous-arbre `old_prefix.*` (et le nœud `old_prefix`) vers
    /// `new_prefix.*`, en préservant items et hiérarchie. Renvoie le nb de nœuds déplacés.
    /// Idempotent (no-op si plus rien sous `old_prefix`). Défaut : non supporté.
    async fn renommer_sous_arbre(&self, _old_prefix: &str, _new_prefix: &str) -> Result<usize> {
        Err(anyhow!("renommer_sous_arbre unsupported by this backend"))
    }

    /// Migration : supprime définitivement tout le sous-arbre `prefix.*` (et le nœud `prefix`).
    /// Pour purger une projection régénérable. Renvoie le nb de nœuds supprimés. Défaut : 0.
    async fn supprimer_sous_arbre(&self, _prefix: &str) -> Result<usize> {
        Ok(0)
    }

    /// Passe de consolidation (doublons, périmés, surchargés, orphelins). N'applique rien.
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

    /// Statistiques de la mémoire (compteurs d'items/nœuds/mutations).
    async fn stats(&self) -> Result<Value> {
        Err(anyhow!("memory_stats unsupported by this backend"))
    }

    /// Journal d'audit : mutations récentes (write/update/delete/review/move…).
    async fn mutations(&self, _limit: Option<u8>) -> Result<Value> {
        Err(anyhow!("memory_mutations unsupported by this backend"))
    }

    async fn dream(&self) -> Result<Value>;

    /// Exporte la mémoire en **bundle OKF** (Open Knowledge Format, Google) : arborescence
    /// de fichiers Markdown + frontmatter YAML (`type` obligatoire). Si `prefix` est `Some`,
    /// n'exporte que ce nœud et son sous-arbre (sinon toute la carte). Renvoie le nb de fichiers.
    async fn export_okf(&self, _dir: &std::path::Path, _prefix: Option<&str>) -> Result<usize> {
        Err(anyhow!("export_okf unsupported by this backend"))
    }

    /// Importe un **bundle OKF** (dossier Markdown+YAML) dans la mémoire. Renvoie le nb d'items importés.
    async fn import_okf(&self, _dir: &std::path::Path) -> Result<usize> {
        Err(anyhow!("import_okf unsupported by this backend"))
    }

    /// Vérifie que le backend répond.
    async fn health(&self) -> Result<bool>;
}
