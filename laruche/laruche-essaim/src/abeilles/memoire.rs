//! Memory tools: the agent reads/writes the cognitive map (paradigm) via the
//! [`MemoireCognitive`] trait. Eventually replaces `knowledge.rs` (flat RAG).
//!
//! These tools are backend-agnostic: whether the `SidecarBackend`
//! (paradigm on :8765) or the future Rust `NativeBackend`, the code stays the same.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use laruche_memoire::{MemoireCognitive, MemoryItem, SearchOpts};
use std::sync::Arc;

/// Nodes managed ONLY by the system: `tools.*` (projection of the tool/skill registry,
/// regenerated at startup) and `system.*` (internal nodes). The agent can READ them (search/tree)
/// but not MUTATE them: otherwise "tidying its memory" would break semantic tool selection.
fn noeud_reserve(node_id: &str) -> bool {
    let id = node_id.trim().trim_matches('.');
    id == "capacities"
        || id == "system"
        || id == "tools" // legacy (before capacities migration)
        || id.starts_with("capacities.")
        || id.starts_with("system.")
        || id.starts_with("tools.")
}

/// Cognitive search in memory.
pub struct MemoireSearch {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireSearch {
    fn nom(&self) -> &str {
        "memory_search"
    }
    fn description(&self) -> &str {
        "Search the persistent cognitive memory (node map + items, hybrid activation + retrieval). \
         Returns relevant facts, decisions and preferences stored in previous conversations. \
         Call before starting a task to orient yourself."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search terms (the user's intent)" },
                "limit": { "type": "integer", "description": "Max items (default 8)" }
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
            .ok_or_else(|| anyhow::anyhow!("'query' required"))?;
        let opts = SearchOpts {
            depth: None,
            limit: args["limit"].as_u64().map(|l| l as u8),
            sans_trace: false,
        };
        match self.mem.search(query, opts).await {
            Ok(pack) => {
                let text = pack.to_prompt_text();
                if text.is_empty() {
                    Ok(ResultatAbeille::ok("No relevant memory found."))
                } else {
                    Ok(ResultatAbeille::ok(format!("Relevant memory:\n{text}")))
                }
            }
            Err(e) => Ok(ResultatAbeille::err(format!(
                "Memory search failed: {e}"
            ))),
        }
    }
}

/// Stores a lasting fact in the cognitive map.
pub struct MemoireWrite {
    pub mem: Arc<dyn MemoireCognitive>,
    /// If true, goes through the review queue (`propose_write`) instead of writing directly.
    pub propose: bool,
}

#[async_trait]
impl Abeille for MemoireWrite {
    fn nom(&self) -> &str {
        "memory_write"
    }
    fn description(&self) -> &str {
        "Store a lasting fact, decision or preference in the cognitive map. \
         Use a dotted node_id: `projects.<name>`, `decisions.<topic>`, `people.<name>`. \
         Call after any decision or fact that must persist across conversations."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "node_id": { "type": "string", "description": "Dotted node, e.g. decisions.archi" },
                "content": { "type": "string", "description": "The fact to memorize" },
                "source": { "type": "string", "description": "Optional provenance" }
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
            .ok_or_else(|| anyhow::anyhow!("'node_id' required"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'content' required"))?;
        // Guardrail: memory_write MUST NOT write into system nodes (capacities.*/
        // system.*). Otherwise the agent dumps "skills" as items into capacities.skills.* and
        // pollutes them (seen in prod: web_research with 2 items, recherche_programme_tv wrongly created).
        // A SKILL is created with skill_create (single item, .md file), not memory_write.
        if noeud_reserve(node_id) {
            return Ok(ResultatAbeille::err(format!(
                "Refused: `{node_id}` is a reserved SYSTEM node (capacities.*/system.*). \
                 To create or update a SKILL, use `skill_create` (never memory_write). \
                 Store lasting facts under projects.*/decisions.*/people.*."
            )));
        }
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
                "Stored in `{node_id}`{}.",
                if self.propose {
                    " (proposed, pending review)"
                } else {
                    ""
                }
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!("Memory write failed: {e}"))),
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
        "Update the content of an existing memory item. Audited by the backend."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "item_id": { "type": "string", "description": "Item ID, e.g. itm_42" },
                "content": { "type": "string", "description": "New lasting content" }
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
            .ok_or_else(|| anyhow::anyhow!("'item_id' required"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'content' required"))?;
        match self.mem.update_item(item_id, content).await {
            Ok(_) => Ok(ResultatAbeille::ok(format!(
                "Memory item updated: {item_id}"
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!("Memory update failed: {e}"))),
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
        "Soft-delete an existing memory item. Use when the user asks to forget, remove or correct a stored fact."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "item_id": { "type": "string", "description": "Item ID, e.g. itm_42" },
                "reason": { "type": "string", "description": "Reason for deletion" }
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
            .ok_or_else(|| anyhow::anyhow!("'item_id' required"))?;
        let reason = args["reason"].as_str();
        match self.mem.delete_item(item_id, reason).await {
            Ok(_) => Ok(ResultatAbeille::ok(format!("Memory item deleted: {item_id}"))),
            Err(e) => Ok(ResultatAbeille::err(format!("Memory delete failed: {e}"))),
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
        "Move a memory item to a different node_id."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "item_id": { "type": "string" },
                "node_id": { "type": "string", "description": "Destination node_id" }
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
            .ok_or_else(|| anyhow::anyhow!("'item_id' required"))?;
        let node_id = args["node_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'node_id' required"))?;
        if noeud_reserve(node_id) {
            return Ok(ResultatAbeille::err(format!(
                "Refused: cannot move item to `{node_id}` (reserved system node)."
            )));
        }
        match self.mem.move_item(item_id, node_id).await {
            Ok(_) => Ok(ResultatAbeille::ok(format!(
                "Memory item moved: {item_id} -> {node_id}"
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!("Memory move failed: {e}"))),
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
        "Accept or reject a proposed memory item pending review."
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
            .ok_or_else(|| anyhow::anyhow!("'item_id' required"))?;
        let action = args["action"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'action' required"))?;
        let reason = args["reason"].as_str();
        match self.mem.review_item(item_id, action, reason).await {
            Ok(_) => Ok(ResultatAbeille::ok(format!("Review {action}: {item_id}"))),
            Err(e) => Ok(ResultatAbeille::err(format!("Memory review failed: {e}"))),
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
        "List proposed memory items pending review."
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
            Err(e) => Ok(ResultatAbeille::err(format!("List proposed failed: {e}"))),
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
        "Suggest existing node_ids for autocomplete or to classify a new memory item."
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
                "Node suggestions failed: {e}"
            ))),
        }
    }
}

/// Cognitive memory statistics.
pub struct MemoireStats {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireStats {
    fn nom(&self) -> &str {
        "memory_stats"
    }
    fn description(&self) -> &str {
        "Return cognitive memory statistics: item counts (active, proposed, deleted), node count, and mutation count."
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
            Err(e) => Ok(ResultatAbeille::err(format!("Memory stats failed: {e}"))),
        }
    }
}

/// Audit log: recent memory mutations.
pub struct MemoireMutations {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireMutations {
    fn nom(&self) -> &str {
        "memory_mutations"
    }
    fn description(&self) -> &str {
        "List the audit log of recent memory mutations (write, update, delete, review, move). Useful to explain what was stored or changed and when."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "limit": { "type": "integer", "description": "Number of entries (default 50)" } }
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
            Err(e) => Ok(ResultatAbeille::err(format!("Memory audit log failed: {e}"))),
        }
    }
}

/// Full tree of the cognitive map (all nodes). For auditing/tidying memory.
pub struct MemoireTree {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireTree {
    fn nom(&self) -> &str {
        "memory_tree"
    }
    fn description(&self) -> &str {
        "List ALL nodes in the cognitive map (full tree: id, parent, label). \
         Use to audit memory: spot duplicate, generic (projects.1, decisions.1) or fragmented \
         nodes before merging them with memory_delete_node."
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
        match self.mem.list_nodes().await {
            Ok(v) => Ok(ResultatAbeille::ok(serde_json::to_string_pretty(&v)?)),
            Err(e) => Ok(ResultatAbeille::err(format!("Memory tree failed: {e}"))),
        }
    }
}

/// Deletes an entire node; its items and child nodes are re-attached to the parent (merge).
pub struct MemoireDeleteNode {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireDeleteNode {
    fn nom(&self) -> &str {
        "memory_delete_node"
    }
    fn description(&self) -> &str {
        "Delete an entire node from the cognitive map: its items (and child nodes) are re-attached \
         to the parent. The MERGE primitive: clean up a duplicate, generic or fragmented node \
         without losing its content. Provide the dotted node_id (e.g. projects.1)."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "node_id": { "type": "string", "description": "Dotted node to delete, e.g. projects.1" }
            },
            "required": ["node_id"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let node_id = args["node_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'node_id' required"))?;
        if noeud_reserve(node_id) {
            return Ok(ResultatAbeille::err(format!(
                "Refused: `{node_id}` is a system node (tools.*/system.*), managed automatically. \
                 Only reorganize your own memory (people/projects/decisions/...)."
            )));
        }
        // LaReine gate: a destructive whole-node deletion is HELD for confirmation
        // (proposals queue) instead of happening immediately. Nothing is removed until
        // you approve it in the Memory tab.
        if crate::reine_queue::gate_actif() {
            crate::reine_queue::proposer_suppression(node_id, "memory_delete_node");
            return Ok(ResultatAbeille::ok(format!(
                "Deletion of `{node_id}` proposed for confirmation. It will be removed only once you approve it in the Memory tab (LaReine gate is on)."
            )));
        }
        match self.mem.delete_node(node_id).await {
            Ok(v) => Ok(ResultatAbeille::ok(format!(
                "Node deleted/merged: {}",
                serde_json::to_string(&v).unwrap_or_default()
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!("delete_node failed: {e}"))),
        }
    }
}

/// Unix timestamp (seconds) -> readable local date "22/06/2026 14:32" (or "?" if missing).
fn fmt_ts(v: &serde_json::Value) -> String {
    match v.as_i64() {
        Some(ts) if ts > 0 => chrono::DateTime::from_timestamp(ts, 0)
            .map(|dt| {
                dt.with_timezone(&chrono::Local)
                    .format("%d/%m/%Y %H:%M")
                    .to_string()
            })
            .unwrap_or_else(|| "?".to_string()),
        _ => "?".to_string(),
    }
}

/// Reads a node: its items WITH timestamps (created/modified), its child nodes and its metadata.
pub struct MemoireReadNode {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireReadNode {
    fn nom(&self) -> &str {
        "memory_read_node"
    }
    fn description(&self) -> &str {
        "Read a cognitive map node: its items with TIMESTAMPS (created / last modified), \
         its child nodes and its metadata. Use to inspect content AND freshness of a node."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "node_id": { "type": "string", "description": "Dotted node, e.g. projects.laruche" }
            },
            "required": ["node_id"]
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
            .ok_or_else(|| anyhow::anyhow!("'node_id' required"))?;
        let node = match self.mem.read_node(node_id).await {
            Ok(n) => n,
            Err(e) => return Ok(ResultatAbeille::err(format!("Cannot read node: {e}"))),
        };
        let mut out = format!(
            "Node `{node_id}` (created {}, updated {})\n",
            fmt_ts(&node["created_at"]),
            fmt_ts(&node["updated_at"])
        );
        if let Some(children) = node["children"].as_array() {
            if !children.is_empty() {
                let noms: Vec<String> = children
                    .iter()
                    .filter_map(|c| c["id"].as_str().map(String::from))
                    .collect();
                out.push_str(&format!("Children: {}\n", noms.join(", ")));
            }
        }
        out.push_str("\nItems:\n");
        match node["items"].as_array() {
            Some(items) if !items.is_empty() => {
                for it in items {
                    let id = it["id"].as_str().unwrap_or("?");
                    let content = it["content"].as_str().unwrap_or("");
                    out.push_str(&format!(
                        "- [{id}] {content}  (created {}, updated {})\n",
                        fmt_ts(&it["created_at"]),
                        fmt_ts(&it["updated_at"])
                    ));
                }
            }
            _ => out.push_str("(no items)\n"),
        }
        Ok(ResultatAbeille::ok(out))
    }
}

/// `memory_doctor`: READ-ONLY audit of memory health (stats, overloaded nodes,
/// duplicates) to decide what to tidy/consolidate. Applies nothing.
pub struct MemoireDoctor {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireDoctor {
    fn nom(&self) -> &str {
        "memory_doctor"
    }
    fn description(&self) -> &str {
        "Read-only memory audit: counters, heaviest nodes, duplicates/overloads. \
         Use to decide what to consolidate (memory_consolidate) or reorganize. Applies no changes."
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
        let stats = self.mem.stats().await.unwrap_or_else(|_| serde_json::json!({}));
        let dream = self.mem.dream().await.unwrap_or_else(|_| serde_json::json!({}));
        let sugg = self
            .mem
            .suggest_nodes("", Some(200))
            .await
            .unwrap_or_else(|_| serde_json::json!({}));
        // Top nodes by item count.
        let mut tops: Vec<(String, u64)> = sugg["nodes"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|n| {
                        let id = n.get("id").and_then(|v| v.as_str())?;
                        let c = n.get("item_count").and_then(|v| v.as_u64()).unwrap_or(0);
                        Some((id.to_string(), c))
                    })
                    .collect()
            })
            .unwrap_or_default();
        tops.sort_by(|a, b| b.1.cmp(&a.1));
        tops.truncate(8);

        let mut out = String::from("# Memory audit\n\n");
        out.push_str(&format!(
            "Stats: {}\n\n",
            serde_json::to_string(&stats).unwrap_or_default()
        ));
        if !tops.is_empty() {
            out.push_str("Heaviest nodes (candidates for consolidation):\n");
            for (id, c) in &tops {
                out.push_str(&format!("- {id}: {c} items\n"));
            }
            out.push('\n');
        }
        if let Some(sug) = dream.get("suggestions").and_then(|s| s.as_array()) {
            if !sug.is_empty() {
                out.push_str(&format!("Suggestions (duplicates/overloads): {}\n", sug.len()));
                for s in sug.iter().take(10) {
                    if let Some(msg) = s.get("message").and_then(|m| m.as_str()) {
                        out.push_str(&format!("- {msg}\n"));
                    }
                }
            }
        }
        out.push_str("\nAction: `memory_consolidate(node_id)` to merge a heavy node.");
        Ok(ResultatAbeille::ok(out))
    }
}

/// `memory_grep`: EXACT substring search in item content (case-insensitive).
/// Complements `memory_search` (semantic): useful to find a specific term, a name,
/// a URL, an id... among all items.
pub struct MemoireGrep {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireGrep {
    fn nom(&self) -> &str {
        "memory_grep"
    }
    fn description(&self) -> &str {
        "Exact substring search (case-insensitive) across all memory item content. \
         Complements memory_search (semantic). Use to find a specific name, URL, or term."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "text to search for" },
                "limit": { "type": "integer", "description": "max results (default 30)" }
            },
            "required": ["pattern"]
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
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'pattern' required"))?;
        match self
            .mem
            .grep(pattern, args["limit"].as_u64().map(|l| l as u8))
            .await
        {
            Ok(v) => Ok(ResultatAbeille::ok(serde_json::to_string_pretty(&v)?)),
            Err(e) => Ok(ResultatAbeille::err(format!("memory_grep failed: {e}"))),
        }
    }
}

/// `memory_consolidate`: merges/deduplicates a node's items into a minimal,
/// synthetic set (e.g. `people.fabien` full of notes -> 1-2 items summarizing everything). Safe: the
/// old items are soft-deleted (recoverable). For tidying/cleaning up memory.
pub struct MemoireConsolidate {
    pub mem: Arc<dyn MemoireCognitive>,
    pub config: crate::brain::EssaimConfig,
}

#[async_trait]
impl Abeille for MemoireConsolidate {
    fn nom(&self) -> &str {
        "memory_consolidate"
    }
    fn description(&self) -> &str {
        "Consolidate a node: merge its items into a minimal, lossless summary \
         (e.g. a 'people.<name>' node packed with notes -> 1-2 items that cover everything). \
         Use to clean up an overloaded node."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "node_id": { "type": "string" } },
            "required": ["node_id"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let node_id = args["node_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'node_id' required"))?;
        match crate::brain::consolider_node(&self.mem, &self.config, node_id).await {
            Ok(v) => Ok(ResultatAbeille::ok(format!(
                "Consolidation: {}",
                serde_json::to_string(&v).unwrap_or_default()
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!("Consolidation failed: {e}"))),
        }
    }
}

// ───────────────────────── SKILLS (self-improvement, third-party-style) ─────────────────────────

fn str_array(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

/// Sync SQL -> disk: writes the skill's `SKILL.md` under `skills/<slug>/` (flat-file,
/// compatible with agentskills.io / third-party: editable, versionable, re-importable).
pub(crate) fn ecrire_skill_md(node_id: &str, content: &str) {
    let slug = node_id.strip_prefix("capacities.skills.").unwrap_or(node_id);
    if slug.is_empty() {
        return;
    }
    let dir = std::path::PathBuf::from("skills").join(slug);
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("SKILL.md"), content);
}

/// Builds a skill's OKF document: frontmatter (type/name/description + DECLARED
/// tools/scripts = bounded skill) + markdown body.
fn build_skill_okf(
    name: &str,
    description: &str,
    tools: &[String],
    scripts: &[String],
    body: &str,
) -> String {
    let mut s = String::from("---\ntype: skill\n");
    s.push_str(&format!("name: {name}\n"));
    if !description.is_empty() {
        s.push_str(&format!("description: {}\n", description.replace('\n', " ")));
    }
    if !tools.is_empty() {
        s.push_str(&format!("tools: [{}]\n", tools.join(", ")));
    }
    if !scripts.is_empty() {
        s.push_str(&format!("scripts: [{}]\n", scripts.join(", ")));
    }
    s.push_str("---\n\n");
    s.push_str(body.trim());
    s.push('\n');
    s
}

/// Replaces ALL content of a skill node with `content` (deletes active items, then writes).
pub(crate) async fn set_skill_content(
    mem: &Arc<dyn MemoireCognitive>,
    node_id: &str,
    content: &str,
) -> Result<()> {
    if let Ok(node) = mem.read_node(node_id).await {
        if let Some(items) = node.get("items").and_then(|i| i.as_array()) {
            for it in items {
                if let Some(id) = it.get("id").and_then(|i| i.as_str()) {
                    let _ = mem.delete_item(id, Some("skill-replace")).await;
                }
            }
        }
    }
    mem.write(MemoryItem::new(node_id.to_string(), content.to_string()).with_source("agent-skill"))
        .await?;
    Ok(())
}

async fn read_skill_content(mem: &Arc<dyn MemoireCognitive>, node_id: &str) -> Option<String> {
    let node = mem.read_node(node_id).await.ok()?;
    let items = node.get("items")?.as_array()?;
    items
        .iter()
        .rev()
        .find_map(|it| it.get("content").and_then(|c| c.as_str()))
        .filter(|c| c.contains("type: skill"))
        .map(String::from)
}

/// `skill_create`: creates OR replaces a skill (reusable procedure) in the right place, well
/// formatted. This is THE way to turn a successful experience into reusable knowledge.
pub struct MemoireSkillCreate {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireSkillCreate {
    fn nom(&self) -> &str {
        "skill_create"
    }
    fn description(&self) -> &str {
        "Create (or replace) a SKILL = reusable procedure, stored under \
         capacities.skills.<name>. Do this AFTER a complex SUCCESSFUL task (>=2 chained tools, \
         errors overcome, non-trivial workflow). Declare the tools/scripts the skill \
         orchestrates (tools/scripts fields) to scope it. For an atomic tool: plugin_create."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "short name, e.g. arxiv-search" },
                "description": { "type": "string", "description": "one line: when to use it" },
                "body": { "type": "string", "description": "Markdown: step-by-step procedure, pitfalls, exact commands" },
                "tools": { "type": "array", "items": { "type": "string" }, "description": "orchestrated tools, e.g. [shell_exec, web_fetch]" },
                "scripts": { "type": "array", "items": { "type": "string" }, "description": "bundled scripts, e.g. [scripts/search.py]" }
            },
            "required": ["name", "description", "body"]
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
        let name = args["name"].as_str().unwrap_or("").trim();
        if name.is_empty() {
            return Ok(ResultatAbeille::err("'name' required"));
        }
        let node_id = crate::abeilles::skill_node_id(name);
        let content = build_skill_okf(
            name,
            args["description"].as_str().unwrap_or(""),
            &str_array(&args["tools"]),
            &str_array(&args["scripts"]),
            args["body"].as_str().unwrap_or(""),
        );
        // LaReine gate: when on, a self-created skill is HELD for human approval
        // (proposals queue in the Memory tab) instead of being written immediately.
        if crate::reine_queue::gate_actif() {
            crate::reine_queue::proposer_skill(&node_id, &content, "skill_create");
            return Ok(ResultatAbeille::ok(format!(
                "Skill `{name}` proposed for review. It will become active once approved in the Memory tab (LaReine gate is on)."
            )));
        }
        match set_skill_content(&self.mem, &node_id, &content).await {
            Ok(_) => {
                ecrire_skill_md(&node_id, &content); // sync SQL -> disk (flat-file)
                Ok(ResultatAbeille::ok(format!(
                    "Skill `{name}` saved to `{node_id}` (+ skills/.../SKILL.md)."
                )))
            }
            Err(e) => Ok(ResultatAbeille::err(format!("skill_create failed: {e}"))),
        }
    }
}

/// `skill_patch`: fixes a skill IN PLACE (find-replace). The "until it works"
/// iteration: when a skill fails or is outdated, patch it immediately.
pub struct MemoireSkillPatch {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireSkillPatch {
    fn nom(&self) -> &str {
        "skill_patch"
    }
    fn description(&self) -> &str {
        "Patch a skill IN PLACE: replace `old` with `new` in its body. Use as soon as a skill \
         fails, is outdated, or a pitfall emerges (iteration). `old` must be unique."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "old": { "type": "string", "description": "exact text to replace" },
                "new": { "type": "string", "description": "replacement text" }
            },
            "required": ["name", "old", "new"]
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
        let name = args["name"].as_str().unwrap_or("").trim();
        let old = args["old"].as_str().unwrap_or("");
        let new = args["new"].as_str().unwrap_or("");
        if name.is_empty() || old.is_empty() {
            return Ok(ResultatAbeille::err("'name' and 'old' required"));
        }
        let node_id = crate::abeilles::skill_node_id(name);
        let Some(content) = read_skill_content(&self.mem, &node_id).await else {
            return Ok(ResultatAbeille::err(format!("Skill not found: {name}")));
        };
        if !content.contains(old) {
            return Ok(ResultatAbeille::err(
                "'old' not found in skill (check exact text)",
            ));
        }
        let patched = content.replacen(old, new, 1);
        match set_skill_content(&self.mem, &node_id, &patched).await {
            Ok(_) => {
                ecrire_skill_md(&node_id, &patched); // sync SQL -> disk
                Ok(ResultatAbeille::ok(format!("Skill `{name}` patched.")))
            }
            Err(e) => Ok(ResultatAbeille::err(format!("skill_patch failed: {e}"))),
        }
    }
}

/// `skill_delete`: deletes a skill (items + scripts folder).
pub struct MemoireSkillDelete {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireSkillDelete {
    fn nom(&self) -> &str {
        "skill_delete"
    }
    fn description(&self) -> &str {
        "Delete a skill (its document and any bundled files/scripts)."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "name": { "type": "string" } },
            "required": ["name"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let name = args["name"].as_str().unwrap_or("").trim();
        if name.is_empty() {
            return Ok(ResultatAbeille::err("'name' required"));
        }
        let node_id = crate::abeilles::skill_node_id(name);
        if let Ok(node) = self.mem.read_node(&node_id).await {
            if let Some(items) = node.get("items").and_then(|i| i.as_array()) {
                for it in items {
                    if let Some(id) = it.get("id").and_then(|i| i.as_str()) {
                        let _ = self.mem.delete_item(id, Some("skill-delete")).await;
                    }
                }
            }
        }
        let slug = node_id
            .strip_prefix("capacities.skills.")
            .unwrap_or(&node_id);
        let _ = std::fs::remove_dir_all(std::path::PathBuf::from("skills").join(slug));
        Ok(ResultatAbeille::ok(format!("Skill `{name}` deleted.")))
    }
}

/// Creates a new node in the cognitive map.
pub struct MemoireCreateNode {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireCreateNode {
    fn nom(&self) -> &str {
        "memory_create_node"
    }
    fn description(&self) -> &str {
        "Create a node in the cognitive map (parent created if needed). node_id in snake_case, \
         e.g. projects.football_bot. Use to organise memory: create a meaningful node before \
         moving items into it (memory_move_item) or merging duplicates."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "node_id": { "type": "string", "description": "Dotted node, e.g. projects.football_bot" },
                "label": { "type": "string", "description": "Human-readable node name" },
                "one_liner": { "type": "string", "description": "Short summary (optional)" },
                "importance": { "type": "number", "description": "0..1 (optional)" }
            },
            "required": ["node_id", "label"]
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
            .ok_or_else(|| anyhow::anyhow!("'node_id' required"))?;
        let label = args["label"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'label' required"))?;
        let one_liner = args["one_liner"].as_str();
        let importance = args["importance"].as_f64().map(|v| v as f32);
        if noeud_reserve(node_id) {
            return Ok(ResultatAbeille::err(format!(
                "Refused: `{node_id}` is under a reserved system prefix (tools.*/system.*)."
            )));
        }
        let res = self
            .mem
            .create_node(node_id, label, one_liner, importance, None)
            .await;
        match res {
            Ok(_) => Ok(ResultatAbeille::ok(format!(
                "Node created: {node_id} ({label})"
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!("create_node failed: {e}"))),
        }
    }
}

/// Renames / updates the metadata of an existing node.
pub struct MemoireUpdateNode {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for MemoireUpdateNode {
    fn nom(&self) -> &str {
        "memory_update_node"
    }
    fn description(&self) -> &str {
        "Update the label, summary (one_liner) or importance of an existing node. \
         Use to RENAME a generic node (e.g. projects.1) to a meaningful name \
         without losing its items."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "node_id": { "type": "string", "description": "Dotted node to update" },
                "label": { "type": "string", "description": "New human-readable name (optional)" },
                "one_liner": { "type": "string", "description": "New short summary (optional)" },
                "importance": { "type": "number", "description": "0..1 (optional)" }
            },
            "required": ["node_id"]
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
            .ok_or_else(|| anyhow::anyhow!("'node_id' required"))?;
        let label = args["label"].as_str();
        let one_liner = args["one_liner"].as_str();
        let importance = args["importance"].as_f64().map(|v| v as f32);
        if noeud_reserve(node_id) {
            return Ok(ResultatAbeille::err(format!(
                "Refused: `{node_id}` is a system node (tools.*/system.*), not editable."
            )));
        }
        match self
            .mem
            .update_node(node_id, label, one_liner, importance)
            .await
        {
            Ok(v) => Ok(ResultatAbeille::ok(format!(
                "Node updated: {}",
                serde_json::to_string(&v).unwrap_or_default()
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!("update_node failed: {e}"))),
        }
    }
}
