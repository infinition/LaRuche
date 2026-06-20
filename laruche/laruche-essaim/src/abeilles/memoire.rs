//! Abeilles mémoire — l'agent lit/écrit la carte cognitive (paradigm) via le trait
//! [`MemoireCognitive`]. Remplace à terme `knowledge.rs` (RAG plat).
//!
//! Ces outils sont agnostiques du backend : qu'il s'agisse du `SidecarBackend`
//! (paradigm sur :8765) ou du futur `NativeBackend` Rust, le code ne change pas.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use laruche_memoire::{MemoireCognitive, MemoryItem, SearchOpts};
use std::sync::Arc;

/// Recherche cognitive dans la mémoire.
pub struct MemoireSearch {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireSearch {
    fn nom(&self) -> &str {
        "memory_search"
    }
    fn description(&self) -> &str {
        "Recherche dans la mémoire cognitive durable (carte de nœuds + items, activation \
         + retrieval hybride). Renvoie les faits/décisions/préférences pertinents stockés \
         lors de conversations précédentes. À appeler avant un travail de fond pour s'orienter."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Termes de recherche (intention de l'utilisateur)" },
                "limit": { "type": "integer", "description": "Nombre max d'items (défaut 8)" }
            },
            "required": ["query"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'query' manquant"))?;
        let opts = SearchOpts {
            depth: None,
            limit: args["limit"].as_u64().map(|l| l as u8),
        };
        match self.mem.search(query, opts).await {
            Ok(pack) => {
                let text = pack.to_prompt_text();
                if text.is_empty() {
                    Ok(ResultatAbeille::ok("Aucun souvenir pertinent."))
                } else {
                    Ok(ResultatAbeille::ok(format!("Mémoire pertinente :\n{text}")))
                }
            }
            Err(e) => Ok(ResultatAbeille::err(format!(
                "Recherche mémoire échouée : {e}"
            ))),
        }
    }
}

/// Mémorise un fait durable dans la carte cognitive.
pub struct MemoireWrite {
    pub mem: Arc<dyn MemoireCognitive>,
    /// Si vrai, passe par la file de revue (`propose_write`) au lieu d'écrire directement.
    pub propose: bool,
}

#[async_trait]
impl Abeille for MemoireWrite {
    fn nom(&self) -> &str {
        "memory_write"
    }
    fn description(&self) -> &str {
        "Mémorise un fait, une décision ou une préférence durable dans la carte cognitive. \
         Utiliser un node_id pointé : `projects.<nom>`, `decisions.<sujet>`, `people.<nom>`. \
         À appeler après une décision ou un fait qui doit survivre aux conversations."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "node_id": { "type": "string", "description": "Nœud pointé, ex. decisions.archi" },
                "content": { "type": "string", "description": "Le fait à mémoriser" },
                "source": { "type": "string", "description": "Provenance optionnelle" }
            },
            "required": ["node_id", "content"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let node_id = args["node_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'node_id' manquant"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'content' manquant"))?;
        let mut item = MemoryItem::new(node_id, content);
        if let Some(src) = args["source"].as_str() {
            item = item.with_source(src);
        }

        let res = if self.propose {
            self.mem.propose_write(item).await
        } else {
            self.mem.write(item).await
        };

        match res {
            Ok(_) => Ok(ResultatAbeille::ok(format!(
                "Mémorisé dans `{node_id}`{}.",
                if self.propose {
                    " (proposé, en attente de revue)"
                } else {
                    ""
                }
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!("Échec de mémorisation : {e}"))),
        }
    }
}

pub struct MemoireUpdateItem {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireUpdateItem {
    fn nom(&self) -> &str {
        "memory_update_item"
    }
    fn description(&self) -> &str {
        "Met a jour le contenu d'un item de memoire existant. Audite par le backend."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "item_id": { "type": "string", "description": "ID de l'item, ex. itm_42" },
                "content": { "type": "string", "description": "Nouveau contenu durable" }
            },
            "required": ["item_id", "content"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let item_id = args["item_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'item_id' manquant"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'content' manquant"))?;
        match self.mem.update_item(item_id, content).await {
            Ok(_) => Ok(ResultatAbeille::ok(format!(
                "Memoire mise a jour: {item_id}"
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!("Echec update memoire: {e}"))),
        }
    }
}

pub struct MemoireDelete {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireDelete {
    fn nom(&self) -> &str {
        "memory_delete"
    }
    fn description(&self) -> &str {
        "Supprime logiquement un item de memoire existant. A utiliser quand l'utilisateur demande d'oublier, retirer ou corriger un souvenir."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "item_id": { "type": "string", "description": "ID de l'item, ex. itm_42" },
                "reason": { "type": "string", "description": "Raison de suppression" }
            },
            "required": ["item_id"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let item_id = args["item_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'item_id' manquant"))?;
        let reason = args["reason"].as_str();
        match self.mem.delete_item(item_id, reason).await {
            Ok(_) => Ok(ResultatAbeille::ok(format!("Memoire supprimee: {item_id}"))),
            Err(e) => Ok(ResultatAbeille::err(format!("Echec delete memoire: {e}"))),
        }
    }
}

pub struct MemoireMoveItem {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireMoveItem {
    fn nom(&self) -> &str {
        "memory_move_item"
    }
    fn description(&self) -> &str {
        "Deplace un item de memoire vers un autre node_id."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "item_id": { "type": "string" },
                "node_id": { "type": "string", "description": "Nouveau node_id" }
            },
            "required": ["item_id", "node_id"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let item_id = args["item_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'item_id' manquant"))?;
        let node_id = args["node_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'node_id' manquant"))?;
        match self.mem.move_item(item_id, node_id).await {
            Ok(_) => Ok(ResultatAbeille::ok(format!(
                "Memoire deplacee: {item_id} -> {node_id}"
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!("Echec move memoire: {e}"))),
        }
    }
}

pub struct MemoireReview {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireReview {
    fn nom(&self) -> &str {
        "memory_review"
    }
    fn description(&self) -> &str {
        "Accepte ou rejette un item propose en attente de revue."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "item_id": { "type": "string" },
                "action": { "type": "string", "enum": ["accept", "reject"] },
                "reason": { "type": "string" }
            },
            "required": ["item_id", "action"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let item_id = args["item_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'item_id' manquant"))?;
        let action = args["action"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'action' manquant"))?;
        let reason = args["reason"].as_str();
        match self.mem.review_item(item_id, action, reason).await {
            Ok(_) => Ok(ResultatAbeille::ok(format!("Review {action}: {item_id}"))),
            Err(e) => Ok(ResultatAbeille::err(format!("Echec review memoire: {e}"))),
        }
    }
}

pub struct MemoireListProposed {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireListProposed {
    fn nom(&self) -> &str {
        "memory_list_proposed"
    }
    fn description(&self) -> &str {
        "Liste les items de memoire proposes en attente de revue."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "limit": { "type": "integer" } }
        })
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        match self
            .mem
            .list_proposed(args["limit"].as_u64().map(|l| l as u8))
            .await
        {
            Ok(value) => Ok(ResultatAbeille::ok(serde_json::to_string_pretty(&value)?)),
            Err(e) => Ok(ResultatAbeille::err(format!("Echec liste proposed: {e}"))),
        }
    }
}

pub struct MemoireSuggestNodes {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireSuggestNodes {
    fn nom(&self) -> &str {
        "memory_suggest_nodes"
    }
    fn description(&self) -> &str {
        "Suggere des node_id existants pour autocomplete ou classement d'un souvenir."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "limit": { "type": "integer" }
            }
        })
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let query = args["query"].as_str().unwrap_or("");
        match self
            .mem
            .suggest_nodes(query, args["limit"].as_u64().map(|l| l as u8))
            .await
        {
            Ok(value) => Ok(ResultatAbeille::ok(serde_json::to_string_pretty(&value)?)),
            Err(e) => Ok(ResultatAbeille::err(format!(
                "Echec suggestions noeuds: {e}"
            ))),
        }
    }
}

/// Statistiques de la mémoire cognitive.
pub struct MemoireStats {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireStats {
    fn nom(&self) -> &str {
        "memory_stats"
    }
    fn description(&self) -> &str {
        "Renvoie des statistiques de la mémoire cognitive : compteurs d'items (actifs, proposés, supprimés), de nœuds et de mutations."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        _args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        match self.mem.stats().await {
            Ok(v) => Ok(ResultatAbeille::ok(serde_json::to_string_pretty(&v)?)),
            Err(e) => Ok(ResultatAbeille::err(format!("Echec stats memoire: {e}"))),
        }
    }
}

/// Journal d'audit : mutations récentes de la mémoire.
pub struct MemoireMutations {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireMutations {
    fn nom(&self) -> &str {
        "memory_mutations"
    }
    fn description(&self) -> &str {
        "Liste le journal d'audit des mutations récentes de la mémoire (write, update, delete, review, move). Utile pour expliquer ce qui a été mémorisé/modifié et quand."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "limit": { "type": "integer", "description": "Nombre d'entrées (défaut 50)" } }
        })
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        match self
            .mem
            .mutations(args["limit"].as_u64().map(|l| l as u8))
            .await
        {
            Ok(v) => Ok(ResultatAbeille::ok(serde_json::to_string_pretty(&v)?)),
            Err(e) => Ok(ResultatAbeille::err(format!("Echec audit memoire: {e}"))),
        }
    }
}
