//! [`SidecarBackend`] — parle à `paradigm serve` via son pont HTTP loopback.
//!
//! Protocole (vérifié dans `paradigm/packages/memory-mcp/src/http-server.mjs`) :
//! - `POST {base}/mcp` avec un corps JSON-RPC 2.0 `{ method: "tools/call",
//!   params: { name, arguments } }`.
//! - La réponse est `{ result: { content: [{ type: "text", text: "<json>" }] } }` ;
//!   le champ `text` est le JSON sérialisé du vrai résultat.
//! - `GET {base}/health` → `{ ok: true, ... }`.
//!
//! On garde SQLite (côté paradigm) comme source de vérité ; ce backend n'est qu'un
//! client réseau loopback (latence négligeable).

use crate::{ContextPack, MemoireCognitive, MemoryItem, SearchOpts};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};

/// Configuration du backend sidecar.
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    /// URL de base du pont paradigm (défaut `http://127.0.0.1:8765`).
    pub base_url: String,
    /// Workspace paradigm à cibler (multi-projets). `None` = workspace par défaut.
    pub workspace: Option<String>,
    /// Jeton bearer si le pont n'est pas en loopback (sinon inutile).
    pub token: Option<String>,
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:8765".to_string(),
            workspace: None,
            token: None,
        }
    }
}

/// Backend mémoire qui délègue à `paradigm serve` sur le pont loopback.
pub struct SidecarBackend {
    client: reqwest::Client,
    cfg: SidecarConfig,
}

impl SidecarBackend {
    pub fn new(cfg: SidecarConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            cfg,
        }
    }

    /// Raccourci : backend loopback par défaut.
    pub fn loopback() -> Self {
        Self::new(SidecarConfig::default())
    }

    /// Appelle un outil MCP paradigm et renvoie le résultat décodé.
    async fn call_tool(&self, name: &str, mut arguments: Value) -> Result<Value> {
        // Injecte le workspace si configuré et absent des arguments.
        if let (Some(ws), Value::Object(map)) = (&self.cfg.workspace, &mut arguments) {
            map.entry("workspace").or_insert_with(|| json!(ws));
        }

        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        });

        let mut req = self
            .client
            .post(format!("{}/mcp", self.cfg.base_url))
            .json(&body);
        if let Some(token) = &self.cfg.token {
            req = req.bearer_auth(token);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| anyhow!("sidecar paradigm injoignable ({}): {e}", self.cfg.base_url))?;
        let v: Value = resp.json().await?;

        if let Some(err) = v.get("error") {
            return Err(anyhow!("erreur paradigm: {err}"));
        }

        // result.content[0].text contient le vrai résultat sérialisé en JSON.
        let text = v
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("réponse MCP inattendue: {v}"))?;

        // Le text est lui-même du JSON ; si jamais ce n'est pas le cas, on le rend tel quel.
        Ok(serde_json::from_str(text).unwrap_or(Value::String(text.to_string())))
    }
}

#[async_trait]
impl MemoireCognitive for SidecarBackend {
    async fn search(&self, query: &str, opts: SearchOpts) -> Result<ContextPack> {
        let mut args = json!({ "query": query });
        if let Some(d) = opts.depth {
            args["depth"] = json!(d);
        }
        if let Some(l) = opts.limit {
            args["limit"] = json!(l);
        }
        let raw = self.call_tool("memory_search", args).await?;
        Ok(ContextPack { raw })
    }

    async fn write(&self, item: MemoryItem) -> Result<Value> {
        self.call_tool("memory_write", serde_json::to_value(&item)?)
            .await
    }

    async fn propose_write(&self, item: MemoryItem) -> Result<Value> {
        self.call_tool("memory_propose_write", serde_json::to_value(&item)?)
            .await
    }

    async fn read_node(&self, node_id: &str) -> Result<Value> {
        self.call_tool(
            "memory_read",
            json!({ "node_id": node_id, "include_items": true }),
        )
        .await
    }

    async fn update_item(&self, item_id: &str, content: &str) -> Result<Value> {
        self.call_tool(
            "memory_update_item",
            json!({ "item_id": item_id, "content": content }),
        )
        .await
    }

    async fn move_item(&self, item_id: &str, node_id: &str) -> Result<Value> {
        self.call_tool(
            "memory_move_item",
            json!({ "item_id": item_id, "node_id": node_id }),
        )
        .await
    }

    async fn delete_item(&self, item_id: &str, reason: Option<&str>) -> Result<Value> {
        self.call_tool(
            "memory_delete",
            json!({ "item_id": item_id, "reason": reason.unwrap_or("delete_via_laruche") }),
        )
        .await
    }

    async fn review_item(
        &self,
        item_id: &str,
        action: &str,
        reason: Option<&str>,
    ) -> Result<Value> {
        self.call_tool(
            "memory_review",
            json!({ "item_id": item_id, "action": action, "reason": reason.unwrap_or("review_via_laruche") }),
        )
        .await
    }

    async fn list_proposed(&self, limit: Option<u8>) -> Result<Value> {
        self.call_tool(
            "memory_list_proposed",
            json!({ "limit": limit.unwrap_or(50) }),
        )
        .await
    }

    async fn stats(&self) -> Result<Value> {
        self.call_tool("memory_stats", json!({})).await
    }

    async fn mutations(&self, limit: Option<u8>) -> Result<Value> {
        self.call_tool("memory_mutations", json!({ "limit": limit.unwrap_or(50) }))
            .await
    }

    async fn suggest_nodes(&self, query: &str, limit: Option<u8>) -> Result<Value> {
        let tree = self
            .call_tool("memory_tree", json!({ "include_items": false }))
            .await?;
        let q = query.to_lowercase();
        let mut nodes = tree
            .get("nodes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|node| {
                if q.is_empty() {
                    return true;
                }
                let hay = format!(
                    "{} {} {}",
                    node.get("id").and_then(Value::as_str).unwrap_or(""),
                    node.get("label").and_then(Value::as_str).unwrap_or(""),
                    node.get("one_liner").and_then(Value::as_str).unwrap_or("")
                )
                .to_lowercase();
                hay.contains(&q)
            })
            .collect::<Vec<_>>();
        nodes.truncate(limit.unwrap_or(12) as usize);
        Ok(json!({ "nodes": nodes }))
    }

    async fn dream(&self) -> Result<Value> {
        self.call_tool("memory_dream", json!({})).await
    }

    async fn health(&self) -> Result<bool> {
        let mut req = self.client.get(format!("{}/health", self.cfg.base_url));
        if let Some(token) = &self.cfg.token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await?;
        let v: Value = resp.json().await?;
        Ok(v.get("ok").and_then(Value::as_bool).unwrap_or(false))
    }
}
