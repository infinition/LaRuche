//! [`NativeBackend`]: pure-Rust, in-memory implementation of the
//! [`MemoireCognitive`] trait. This is the **P3 seed**: no Node, no sidecar.
//!
//! Simple storage (HashMap node -> items) with lexical keyword-based retrieval.
//! Sufficient for a working POC and to demonstrate that `brain.rs` is entirely
//! backend-agnostic. The full cognitive engine (FTS5 + embeddings + activation)
//! will later replace this store, behind the same interface.

use crate::{ContextPack, MemoireCognitive, MemoryItem, SearchOpts};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Splits into normalized keywords (lowercase, alphanumeric, length > 2).
fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 2)
        .map(|t| t.to_string())
        .collect()
}

fn node_parent_id(node_id: &str) -> Option<String> {
    let trimmed = node_id.trim_matches('.');
    trimmed
        .rfind('.')
        .map(|idx| trimmed[..idx].to_string())
        .filter(|s| !s.is_empty())
}

fn node_label(node_id: &str) -> String {
    node_id
        .trim_matches('.')
        .rsplit('.')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(node_id)
        .to_string()
}

fn node_json(node_id: &str) -> Value {
    json!({
        "id": node_id,
        "node_id": node_id,
        "label": node_label(node_id),
        "one_liner": node_parent_id(node_id)
            .map(|p| format!("Subnode of {p}"))
            .unwrap_or_else(|| "Root node".to_string()),
        "parent_id": node_parent_id(node_id),
        "importance": 0.5
    })
}

fn collect_node_ids(store: &HashMap<String, Vec<StoredItem>>) -> HashSet<String> {
    let mut ids = HashSet::new();
    for node in store.keys() {
        let mut current = Some(node.as_str());
        while let Some(id) = current {
            ids.insert(id.to_string());
            current = id
                .rfind('.')
                .map(|idx| &id[..idx])
                .filter(|s| !s.is_empty());
        }
    }
    ids
}

#[derive(Clone)]
struct StoredItem {
    id: String,
    content: String,
    #[allow(dead_code)]
    tags: Vec<String>,
    #[allow(dead_code)]
    source: Option<String>,
    proposed: bool,
    deleted: bool,
}

/// Explicit node metadata (label/one_liner/importance) set by
/// `create_node`/`update_node`. Otherwise the label is derived from the node_id.
#[derive(Clone, Default)]
struct NodeMeta {
    label: Option<String>,
    one_liner: Option<String>,
    importance: Option<f32>,
    /// Provenance, written at node creation. Not read by the native backend
    /// (SQLite persists and serves it); kept so both backends share the shape.
    #[allow(dead_code)]
    source: Option<String>,
}

/// Native (in-RAM) memory backend. Clone via `Arc`.
#[derive(Default)]
pub struct NativeBackend {
    store: Mutex<HashMap<String, Vec<StoredItem>>>,
    meta: Mutex<HashMap<String, NodeMeta>>,
    counter: AtomicU64,
}

/// Applies explicit metadata (if present) onto a derived `node_json`.
fn overlay_meta(mut node: Value, meta: &HashMap<String, NodeMeta>) -> Value {
    if let Some(id) = node.get("id").and_then(Value::as_str) {
        if let Some(m) = meta.get(id) {
            if let Some(l) = &m.label {
                node["label"] = json!(l);
            }
            if let Some(o) = &m.one_liner {
                node["one_liner"] = json!(o);
            }
            if let Some(i) = m.importance {
                node["importance"] = json!(i);
            }
        }
    }
    node
}

impl NativeBackend {
    pub fn new() -> Self {
        Self::default()
    }

    fn next_id(&self) -> String {
        format!("itm_{}", self.counter.fetch_add(1, Ordering::Relaxed))
    }

    fn insert(&self, item: MemoryItem, proposed: bool) -> String {
        let id = self.next_id();
        let mut store = self.store.lock().unwrap();
        store
            .entry(item.node_id.clone())
            .or_default()
            .push(StoredItem {
                id: id.clone(),
                content: item.content,
                tags: item.tags,
                source: item.source,
                proposed,
                deleted: false,
            });
        id
    }
}

#[async_trait]
impl MemoireCognitive for NativeBackend {
    async fn search(&self, query: &str, opts: SearchOpts) -> Result<ContextPack> {
        let limit = opts.limit.unwrap_or(8) as usize;
        let qtok = tokenize(query);

        let store = self.store.lock().unwrap();
        // (score, item id, node, content)
        let mut hits: Vec<(usize, String, String, String)> = Vec::new();
        for (node, list) in store.iter() {
            for it in list {
                if it.proposed || it.deleted {
                    continue; // proposed items are excluded from search (like paradigm)
                }
                let ctok = tokenize(&format!("{} {}", node, it.content));
                // Prefix-tolerant lexical overlap (code/coder, aime/aiment, etc.).
                // NB: real semantic matching (embeddings) will come with the cognitive engine.
                let score = qtok
                    .iter()
                    .filter(|q| {
                        ctok.iter()
                            .any(|c| c.starts_with(q.as_str()) || q.starts_with(c.as_str()))
                    })
                    .count();
                if score > 0 {
                    hits.push((score, it.id.clone(), node.clone(), it.content.clone()));
                }
            }
        }
        hits.sort_by(|a, b| b.0.cmp(&a.0));
        hits.truncate(limit);

        let mut seen = HashSet::new();
        let nodes: Vec<Value> = hits
            .iter()
            .filter_map(|(_, _, n, _)| seen.insert(n.clone()).then(|| node_json(n)))
            .collect();
        let items: Vec<Value> = hits
            .iter()
            .map(|(_, id, n, c)| json!({ "id": id, "node_id": n, "content": c }))
            .collect();

        Ok(ContextPack {
            raw: json!({ "nodes": nodes, "items": items }),
        })
    }

    async fn write(&self, item: MemoryItem) -> Result<Value> {
        let node = item.node_id.clone();
        let id = self.insert(item, false);
        Ok(json!({ "ok": true, "item_id": id, "node_id": node }))
    }

    async fn propose_write(&self, item: MemoryItem) -> Result<Value> {
        let node = item.node_id.clone();
        let id = self.insert(item, true);
        Ok(json!({ "ok": true, "item_id": id, "node_id": node, "status": "proposed" }))
    }

    async fn read_node(&self, node_id: &str) -> Result<Value> {
        let store = self.store.lock().unwrap();
        let meta = self.meta.lock().unwrap();
        let all_nodes = collect_node_ids(&store);
        let mut children: Vec<Value> = all_nodes
            .iter()
            .filter(|id| node_parent_id(id).as_deref() == Some(node_id))
            .map(|id| overlay_meta(node_json(id), &meta))
            .collect();
        children.sort_by(|a, b| {
            a["id"]
                .as_str()
                .unwrap_or_default()
                .cmp(b["id"].as_str().unwrap_or_default())
        });
        let items: Vec<Value> = store
            .get(node_id)
            .map(|list| {
                list.iter()
                    .filter(|it| !it.proposed && !it.deleted)
                    .map(|it| json!({ "id": it.id, "node_id": node_id, "content": it.content }))
                    .collect()
            })
            .unwrap_or_default();
        let mut node = overlay_meta(node_json(node_id), &meta);
        node["children"] = json!(children);
        node["items"] = json!(items);
        Ok(node)
    }

    async fn update_item(&self, item_id: &str, content: &str) -> Result<Value> {
        let mut store = self.store.lock().unwrap();
        for (node_id, list) in store.iter_mut() {
            if let Some(it) = list.iter_mut().find(|it| it.id == item_id && !it.deleted) {
                it.content = content.to_string();
                return Ok(
                    json!({ "ok": true, "item_id": item_id, "node_id": node_id, "content": content }),
                );
            }
        }
        Err(anyhow!("unknown item: {item_id}"))
    }

    async fn move_item(&self, item_id: &str, node_id: &str) -> Result<Value> {
        let mut store = self.store.lock().unwrap();
        let mut found: Option<(String, StoredItem)> = None;
        for (current_node, list) in store.iter_mut() {
            if let Some(pos) = list.iter().position(|it| it.id == item_id && !it.deleted) {
                found = Some((current_node.clone(), list.remove(pos)));
                break;
            }
        }
        let Some((from, item)) = found else {
            return Err(anyhow!("unknown item: {item_id}"));
        };
        store.entry(node_id.to_string()).or_default().push(item);
        Ok(json!({ "ok": true, "item_id": item_id, "node_id": node_id, "from": from }))
    }

    async fn delete_item(&self, item_id: &str, reason: Option<&str>) -> Result<Value> {
        let mut store = self.store.lock().unwrap();
        for (node_id, list) in store.iter_mut() {
            if let Some(it) = list.iter_mut().find(|it| it.id == item_id && !it.deleted) {
                it.deleted = true;
                return Ok(json!({
                    "ok": true,
                    "item_id": item_id,
                    "node_id": node_id,
                    "status": "deleted",
                    "reason": reason.unwrap_or("delete_via_laruche")
                }));
            }
        }
        Err(anyhow!("unknown item: {item_id}"))
    }

    async fn review_item(
        &self,
        item_id: &str,
        action: &str,
        reason: Option<&str>,
    ) -> Result<Value> {
        let mut store = self.store.lock().unwrap();
        for (node_id, list) in store.iter_mut() {
            if let Some(it) = list.iter_mut().find(|it| it.id == item_id && !it.deleted) {
                if !it.proposed {
                    return Err(anyhow!("item not proposed: {item_id}"));
                }
                match action {
                    "accept" => {
                        it.proposed = false;
                        return Ok(
                            json!({ "ok": true, "item_id": item_id, "node_id": node_id, "status": "active" }),
                        );
                    }
                    "reject" => {
                        it.deleted = true;
                        return Ok(json!({
                            "ok": true,
                            "item_id": item_id,
                            "node_id": node_id,
                            "status": "deleted",
                            "reason": reason.unwrap_or("reject_via_laruche")
                        }));
                    }
                    _ => return Err(anyhow!("invalid review action: {action}")),
                }
            }
        }
        Err(anyhow!("unknown item: {item_id}"))
    }

    async fn list_proposed(&self, limit: Option<u8>) -> Result<Value> {
        let store = self.store.lock().unwrap();
        let mut items = Vec::new();
        for (node_id, list) in store.iter() {
            for it in list {
                if it.proposed && !it.deleted {
                    items.push(json!({
                        "id": it.id,
                        "node_id": node_id,
                        "content": it.content,
                        "source": it.source,
                        "status": "proposed",
                    }));
                }
            }
        }
        items.truncate(limit.unwrap_or(50) as usize);
        Ok(json!({ "count": items.len(), "items": items }))
    }

    async fn suggest_nodes(&self, query: &str, limit: Option<u8>) -> Result<Value> {
        let store = self.store.lock().unwrap();
        let all_nodes = collect_node_ids(&store);
        let q = query.to_lowercase();
        let mut nodes: Vec<Value> = all_nodes
            .iter()
            .filter(|id| q.is_empty() || id.to_lowercase().contains(&q))
            .map(|id| {
                let mut node = node_json(id);
                node["item_count"] = json!(store
                    .get(id)
                    .map(|items| {
                        items
                            .iter()
                            .filter(|it| !it.proposed && !it.deleted)
                            .count()
                    })
                    .unwrap_or(0));
                node
            })
            .collect();
        nodes.sort_by(|a, b| {
            a["id"]
                .as_str()
                .unwrap_or_default()
                .cmp(b["id"].as_str().unwrap_or_default())
        });
        nodes.truncate(limit.unwrap_or(12) as usize);
        Ok(json!({ "nodes": nodes }))
    }

    async fn dream(&self) -> Result<Value> {
        // POC heuristic: detects exact duplicates per node.
        let store = self.store.lock().unwrap();
        let mut duplicates = 0u64;
        let mut suggestions = Vec::new();
        for (node_id, list) in store.iter() {
            let mut seen = HashSet::new();
            let mut active_count = 0u64;
            for it in list {
                if it.proposed || it.deleted {
                    continue;
                }
                active_count += 1;
                if !seen.insert(it.content.clone()) {
                    duplicates += 1;
                    suggestions.push(json!({
                        "kind": "duplicate",
                        "severity": "medium",
                        "node_id": node_id,
                        "count": 2,
                        "message": format!("Exact duplicate in {node_id}: {}", it.content.chars().take(80).collect::<String>())
                    }));
                }
            }
            if active_count > 12 {
                suggestions.push(json!({
                    "kind": "overloaded",
                    "severity": "low",
                    "node_id": node_id,
                    "count": active_count,
                    "message": format!("{node_id} contains {active_count} active items; consider subnodes.")
                }));
            }
        }
        Ok(json!({ "suggestions": suggestions, "duplicates": duplicates, "orphan_items": 0 }))
    }

    async fn list_nodes(&self) -> Result<Value> {
        let store = self.store.lock().unwrap();
        let meta = self.meta.lock().unwrap();
        let mut nodes: Vec<Value> = collect_node_ids(&store)
            .iter()
            .map(|id| overlay_meta(node_json(id), &meta))
            .collect();
        nodes.sort_by(|a, b| {
            a["id"]
                .as_str()
                .unwrap_or_default()
                .cmp(b["id"].as_str().unwrap_or_default())
        });
        Ok(json!(nodes))
    }

    async fn delete_node(&self, node_id: &str) -> Result<Value> {
        let id = node_id.trim_matches('.').to_string();
        if id.is_empty() {
            return Err(anyhow!("empty node_id"));
        }
        // Root (no parent): relocate the whole subtree under `orphans.<id>` (nothing lost).
        // Otherwise: items + subnodes move up to the parent (the node segment is dropped).
        let (cible_self, prefix_dest, racine) = match node_parent_id(&id) {
            Some(parent) => (parent.clone(), parent, false),
            None => {
                let dest = format!("orphans.{id}");
                (dest.clone(), dest, true)
            }
        };
        let prefix = format!("{id}.");
        let mut store = self.store.lock().unwrap();
        if let Some(items) = store.remove(&id) {
            store.entry(cible_self.clone()).or_default().extend(items);
        }
        let descendants: Vec<String> = store
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .cloned()
            .collect();
        for child in descendants {
            if let Some(items) = store.remove(&child) {
                let suffix = child.strip_prefix(&prefix).unwrap_or(&child);
                // root: orphans.<id>.suffix ; non-root: parent.suffix
                let new_id = format!("{prefix_dest}.{suffix}");
                store.entry(new_id).or_default().extend(items);
            }
        }
        // Metadata: follows the same remap.
        {
            let mut meta = self.meta.lock().unwrap();
            if let Some(m) = meta.remove(&id) {
                meta.insert(cible_self.clone(), m);
            }
            let keys: Vec<String> = meta
                .keys()
                .filter(|k| k.starts_with(&prefix))
                .cloned()
                .collect();
            for k in keys {
                if let Some(m) = meta.remove(&k) {
                    let suffix = k.strip_prefix(&prefix).unwrap_or(&k);
                    meta.insert(format!("{prefix_dest}.{suffix}"), m);
                }
            }
        }
        Ok(json!({ "deleted": id, "moved_to": cible_self, "racine": racine }))
    }

    async fn create_node(
        &self,
        node_id: &str,
        label: &str,
        one_liner: Option<&str>,
        importance: Option<f32>,
        source: Option<&str>,
    ) -> Result<Value> {
        let id = node_id.trim_matches('.').to_string();
        if id.is_empty() {
            return Err(anyhow!("empty node_id"));
        }
        // Create an empty entry: the node becomes enumerable even without items.
        self.store.lock().unwrap().entry(id.clone()).or_default();
        self.meta.lock().unwrap().insert(
            id.clone(),
            NodeMeta {
                label: Some(label.to_string()),
                one_liner: one_liner.map(|s| s.to_string()),
                importance,
                source: source.map(|s| s.to_string()),
            },
        );
        Ok(json!({ "created": id, "label": label, "parent_id": node_parent_id(node_id) }))
    }

    async fn update_node(
        &self,
        node_id: &str,
        label: Option<&str>,
        one_liner: Option<&str>,
        importance: Option<f32>,
    ) -> Result<Value> {
        let id = node_id.trim_matches('.').to_string();
        let mut meta = self.meta.lock().unwrap();
        let entry = meta.entry(id.clone()).or_default();
        if let Some(l) = label {
            entry.label = Some(l.to_string());
        }
        if let Some(o) = one_liner {
            entry.one_liner = Some(o.to_string());
        }
        if let Some(i) = importance {
            entry.importance = Some(i);
        }
        let m = meta.clone();
        Ok(overlay_meta(node_json(&id), &m))
    }

    async fn renommer_sous_arbre(&self, old_prefix: &str, new_prefix: &str) -> Result<usize> {
        let old = old_prefix.trim_matches('.');
        let new = new_prefix.trim_matches('.');
        if old.is_empty() || new.is_empty() {
            return Ok(0);
        }
        let dot = format!("{old}.");
        let remap = |k: &str| -> Option<String> {
            if k == old {
                Some(new.to_string())
            } else {
                k.strip_prefix(&dot).map(|rest| format!("{new}.{rest}"))
            }
        };
        let mut moved = 0usize;
        {
            let mut store = self.store.lock().unwrap();
            let keys: Vec<String> = store.keys().cloned().collect();
            for k in keys {
                if let Some(nk) = remap(&k) {
                    if let Some(items) = store.remove(&k) {
                        store.entry(nk).or_default().extend(items);
                        moved += 1;
                    }
                }
            }
        }
        {
            let mut meta = self.meta.lock().unwrap();
            let keys: Vec<String> = meta.keys().cloned().collect();
            for k in keys {
                if let Some(nk) = remap(&k) {
                    if let Some(m) = meta.remove(&k) {
                        meta.insert(nk, m);
                    }
                }
            }
        }
        Ok(moved)
    }

    async fn supprimer_sous_arbre(&self, prefix: &str) -> Result<usize> {
        let p = prefix.trim_matches('.');
        if p.is_empty() {
            return Ok(0);
        }
        let dot = format!("{p}.");
        let matches = |k: &str| k == p || k.starts_with(&dot);
        let mut removed = 0usize;
        {
            let mut store = self.store.lock().unwrap();
            let keys: Vec<String> = store.keys().filter(|k| matches(k)).cloned().collect();
            for k in keys {
                store.remove(&k);
                removed += 1;
            }
        }
        {
            let mut meta = self.meta.lock().unwrap();
            let keys: Vec<String> = meta.keys().filter(|k| matches(k)).cloned().collect();
            for k in keys {
                meta.remove(&k);
            }
        }
        Ok(removed)
    }

    async fn grep(&self, pattern: &str, limit: Option<u8>) -> Result<Value> {
        let pat = pattern.to_lowercase();
        let store = self.store.lock().unwrap();
        let mut items = Vec::new();
        for (node, list) in store.iter() {
            for it in list {
                if it.proposed || it.deleted {
                    continue;
                }
                if it.content.to_lowercase().contains(&pat) {
                    items.push(json!({ "id": it.id, "node_id": node, "content": it.content }));
                }
            }
        }
        items.truncate(limit.unwrap_or(30) as usize);
        Ok(json!({ "count": items.len(), "items": items }))
    }

    async fn health(&self) -> Result<bool> {
        Ok(true)
    }
}
