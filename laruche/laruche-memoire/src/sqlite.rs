//! [`SqliteBackend`] — moteur mémoire **persistant 100 % Rust** : SQLite + FTS5 + audit
//! + recherche **hybride sémantique/lexicale** quand un [`Embedder`] est branché.
//!
//! Port fidèle de la partie `sqlite-store` de paradigm (stockage durable, FTS5 BM25,
//! items proposés exclus, journal d'audit) + la couche sémantique (T1). Mono-binaire
//! (rusqlite `bundled`). Sans embedder → recall lexical FTS5 ; avec embedder → hybride.

use crate::embed::{cosine, Embedder};
use crate::{ContextPack, MemoireCognitive, MemoryItem, SearchOpts};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rusqlite::{Connection, OptionalExtension};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SqliteBackend {
    conn: Mutex<Connection>,
    embedder: Option<Arc<dyn Embedder>>,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Échappe une valeur pour un scalaire YAML entre guillemets (frontmatter OKF).
fn yaml_q(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Collecte récursivement tous les `index.md` d'un bundle OKF.
fn collect_index_md(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect_index_md(&p, out);
            } else if p.file_name().map(|n| n == "index.md").unwrap_or(false) {
                out.push(p);
            }
        }
    }
}

/// Parse un `index.md` OKF → (node_id, items du corps). `fallback_id` si pas d'`id:` en frontmatter.
fn parse_okf(content: &str, fallback_id: &str) -> (String, Vec<String>) {
    let mut node_id = fallback_id.to_string();
    let mut items = Vec::new();
    let mut seen_front = 0u8; // 1 = dans le frontmatter, >=2 = corps
    for line in content.lines() {
        if line.trim() == "---" {
            seen_front += 1;
            continue;
        }
        if seen_front == 1 {
            if let Some(v) = line.trim().strip_prefix("id:") {
                let v = v.trim().trim_matches('"').trim();
                if !v.is_empty() {
                    node_id = v.to_string();
                }
            }
        } else if seen_front >= 2 {
            if let Some(rest) = line.trim_start().strip_prefix("- ") {
                let c = rest.split("  _(source:").next().unwrap_or(rest).trim();
                if !c.is_empty() {
                    items.push(c.to_string());
                }
            }
        }
    }
    (node_id, items)
}

fn tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 2)
        .map(|t| t.to_string())
        .collect()
}

fn vec_to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}
fn blob_to_vec(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
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

fn ensure_node(conn: &Connection, node_id: &str) -> Result<()> {
    let node_id = node_id.trim_matches('.');
    if node_id.is_empty() {
        return Ok(());
    }
    let parent_id = node_parent_id(node_id);
    if let Some(parent) = parent_id.as_deref() {
        ensure_node(conn, parent)?;
    }
    let label = node_label(node_id);
    let one_liner = parent_id
        .as_ref()
        .map(|p| format!("Sous-noeud de {p}"))
        .unwrap_or_else(|| "Noeud racine".to_string());
    conn.execute(
        "INSERT OR IGNORE INTO nodes(id,parent_id,label,one_liner,importance,created_at)
         VALUES(?1,?2,?3,?4,?5,?6)",
        rusqlite::params![node_id, parent_id, label, one_liner, 0.5f32, now()],
    )?;
    Ok(())
}

fn node_json(conn: &Connection, node_id: &str) -> Result<Value> {
    let row = conn
        .query_row(
            "SELECT label, one_liner, parent_id, importance FROM nodes WHERE id=?1",
            [node_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, f32>(3)?,
                ))
            },
        )
        .optional()?;
    let (label, one_liner, parent_id, importance) = row.unwrap_or_else(|| {
        (
            node_label(node_id),
            String::new(),
            node_parent_id(node_id),
            0.5,
        )
    });
    Ok(json!({
        "id": node_id,
        "node_id": node_id,
        "label": label,
        "one_liner": one_liner,
        "parent_id": parent_id,
        "importance": importance
    }))
}

fn parse_item_rowid(item_id: &str) -> Result<i64> {
    item_id
        .strip_prefix("itm_")
        .unwrap_or(item_id)
        .parse::<i64>()
        .map_err(|_| anyhow!("item_id invalide: {item_id}"))
}

fn refresh_fts_row(
    conn: &Connection,
    id: i64,
    node_id: &str,
    content: &str,
    status: &str,
) -> Result<()> {
    conn.execute("DELETE FROM items_fts WHERE rowid=?1", [id])?;
    if status == "active" {
        conn.execute(
            "INSERT INTO items_fts(rowid,content,node_id) VALUES(?1,?2,?3)",
            rusqlite::params![id, content, node_id],
        )?;
    }
    Ok(())
}

impl SqliteBackend {
    /// Ouvre la base (recall lexical FTS5 uniquement).
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_inner(path, None)
    }

    /// Ouvre la base avec recall **hybride sémantique** via l'embedder fourni.
    pub fn open_with_embedder(path: impl AsRef<Path>, embedder: Arc<dyn Embedder>) -> Result<Self> {
        Self::open_inner(path, Some(embedder))
    }

    fn open_inner(path: impl AsRef<Path>, embedder: Option<Arc<dyn Embedder>>) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS items(
               id INTEGER PRIMARY KEY,
               node_id TEXT NOT NULL,
               content TEXT NOT NULL,
               source TEXT,
               status TEXT NOT NULL DEFAULT 'active',
               embedding BLOB,
               created_at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS nodes(
               id TEXT PRIMARY KEY,
               parent_id TEXT,
               label TEXT NOT NULL,
               one_liner TEXT NOT NULL DEFAULT '',
               importance REAL NOT NULL DEFAULT 0.5,
               created_at INTEGER NOT NULL);
             CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(content, node_id);
             CREATE TABLE IF NOT EXISTS mutations(
               id INTEGER PRIMARY KEY,
               op TEXT NOT NULL, node_id TEXT, content TEXT, ts INTEGER NOT NULL);",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
            embedder,
        })
    }

    fn insert(&self, item: &MemoryItem, status: &str, embedding: Option<Vec<u8>>) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        ensure_node(&conn, &item.node_id)?;
        conn.execute(
            "INSERT INTO items(node_id,content,source,status,embedding,created_at) VALUES(?1,?2,?3,?4,?5,?6)",
            rusqlite::params![item.node_id, item.content, item.source, status, embedding, now()],
        )?;
        let id = conn.last_insert_rowid();
        if status == "active" {
            conn.execute(
                "INSERT INTO items_fts(rowid,content,node_id) VALUES(?1,?2,?3)",
                rusqlite::params![id, item.content, item.node_id],
            )?;
        }
        conn.execute(
            "INSERT INTO mutations(op,node_id,content,ts) VALUES(?1,?2,?3,?4)",
            rusqlite::params![
                if status == "active" {
                    "write"
                } else {
                    "propose"
                },
                item.node_id,
                item.content,
                now()
            ],
        )?;
        Ok(id)
    }

    async fn embed_opt(&self, text: &str) -> Option<Vec<f32>> {
        match &self.embedder {
            Some(e) => e.embed(text).await.ok(),
            None => None,
        }
    }

    fn pack(nodes: Vec<Value>, items: Vec<Value>) -> ContextPack {
        ContextPack {
            raw: json!({ "nodes": nodes, "items": items }),
        }
    }
}

#[async_trait]
impl MemoireCognitive for SqliteBackend {
    async fn search(&self, query: &str, opts: SearchOpts) -> Result<ContextPack> {
        let limit = opts.limit.unwrap_or(8) as usize;
        let qvec = self.embed_opt(query).await; // calculé AVANT le lock (pas d'await sous mutex)
        let qtoks = tokens(query);

        let conn = self.conn.lock().unwrap();

        // (score, item id, node, content)
        let mut hits: Vec<(f32, i64, String, String)> = Vec::new();

        // Activation cognitive : chaque nœud s'illumine selon le recouvrement requête ↔ chemin/label
        // + son importance. Les items d'un sous-arbre pertinent sont ensuite boostés (façon paradigm).
        let mut node_act: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
        {
            let mut nstmt = conn.prepare("SELECT id, label, importance FROM nodes")?;
            let rows = nstmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, f32>(2)?,
                ))
            })?;
            for row in rows {
                let (id, label, importance) = row?;
                let ntoks = tokens(&format!("{} {}", id.replace('.', " "), label));
                let overlap = qtoks.iter().filter(|t| ntoks.contains(*t)).count() as f32;
                let act = (overlap * 0.5).min(1.0) + importance * 0.3;
                if act > 0.0 {
                    node_act.insert(id, act);
                }
            }
        }
        // Activation effective d'un item = activation de son nœud + part décroissante du parent.
        let activation_of = |node: &str| -> f32 {
            let na = node_act.get(node).copied().unwrap_or(0.0);
            let pa = node
                .rsplit_once('.')
                .and_then(|(p, _)| node_act.get(p))
                .copied()
                .unwrap_or(0.0);
            na + 0.3 * pa
        };

        if let Some(qv) = qvec {
            // Recall HYBRIDE : cosinus sémantique + petit boost lexical.
            let mut stmt = conn.prepare(
                "SELECT id, node_id, content, embedding FROM items WHERE status='active' AND embedding IS NOT NULL",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Vec<u8>>(3)?,
                ))
            })?;
            for row in rows {
                let (id, node, content, blob) = row?;
                let sem = cosine(&qv, &blob_to_vec(&blob));
                let hay = format!("{node} {content}").to_lowercase();
                let lex = if qtoks.iter().any(|t| hay.contains(t)) {
                    0.3
                } else {
                    0.0
                };
                let score = 0.7 * sem + lex + 0.25 * activation_of(&node);
                if score > 0.05 {
                    hits.push((score, id, node, content));
                }
            }
        } else if !qtoks.is_empty() {
            // Repli LEXICAL : FTS5 BM25.
            let match_expr = qtoks
                .iter()
                .map(|t| format!("\"{t}\""))
                .collect::<Vec<_>>()
                .join(" OR ");
            let mut stmt = conn.prepare(
                "SELECT i.id, i.node_id, i.content FROM items_fts f JOIN items i ON i.id = f.rowid \
                 WHERE f.items_fts MATCH ?1 AND i.status='active' ORDER BY bm25(items_fts) LIMIT ?2",
            )?;
            let rows = stmt.query_map(rusqlite::params![match_expr, limit as i64], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?;
            for row in rows {
                let (id, node, content) = row?;
                let score = 1.0 + 0.25 * activation_of(&node);
                hits.push((score, id, node, content));
            }
        }

        hits.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        hits.truncate(limit);

        let mut nodes = Vec::new();
        let mut items = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (_, id, node, content) in hits {
            if seen.insert(node.clone()) {
                nodes.push(node_json(&conn, &node)?);
            }
            items.push(json!({ "id": format!("itm_{id}"), "node_id": node, "content": content }));
        }
        Ok(Self::pack(nodes, items))
    }

    async fn write(&self, item: MemoryItem) -> Result<Value> {
        let emb = self.embed_opt(&item.content).await.map(|v| vec_to_blob(&v));
        let id = self.insert(&item, "active", emb)?;
        Ok(json!({ "ok": true, "item_id": format!("itm_{id}"), "node_id": item.node_id }))
    }

    async fn propose_write(&self, item: MemoryItem) -> Result<Value> {
        let emb = self.embed_opt(&item.content).await.map(|v| vec_to_blob(&v));
        let id = self.insert(&item, "proposed", emb)?;
        Ok(json!({ "ok": true, "item_id": format!("itm_{id}"), "status": "proposed" }))
    }

    async fn read_node(&self, node_id: &str) -> Result<Value> {
        let conn = self.conn.lock().unwrap();
        ensure_node(&conn, node_id)?;

        let mut child_stmt = conn.prepare(
            "SELECT id, label, one_liner, parent_id, importance FROM nodes
             WHERE parent_id=?1 ORDER BY id",
        )?;
        let child_rows = child_stmt.query_map([node_id], |r| {
            let id: String = r.get(0)?;
            Ok(json!({
                "id": id,
                "node_id": id,
                "label": r.get::<_, String>(1)?,
                "one_liner": r.get::<_, String>(2)?,
                "parent_id": r.get::<_, Option<String>>(3)?,
                "importance": r.get::<_, f32>(4)?,
            }))
        })?;
        let children: Vec<Value> = child_rows.filter_map(|r| r.ok()).collect();

        let mut stmt = conn.prepare(
            "SELECT id, content, source, created_at FROM items
             WHERE node_id=?1 AND status='active' ORDER BY created_at",
        )?;
        let rows = stmt.query_map([node_id], |r| {
            Ok(json!({
                "id": format!("itm_{}", r.get::<_,i64>(0)?),
                "node_id": node_id,
                "content": r.get::<_,String>(1)?,
                "source": r.get::<_, Option<String>>(2)?,
                "created_at": r.get::<_, i64>(3)?,
            }))
        })?;
        let items: Vec<Value> = rows.filter_map(|r| r.ok()).collect();
        let mut node = node_json(&conn, node_id)?;
        node["children"] = json!(children);
        node["items"] = json!(items);
        Ok(node)
    }

    async fn update_item(&self, item_id: &str, content: &str) -> Result<Value> {
        let emb = self.embed_opt(content).await.map(|v| vec_to_blob(&v));
        let id = parse_item_rowid(item_id)?;
        let conn = self.conn.lock().unwrap();
        let existing = conn
            .query_row("SELECT node_id, status FROM items WHERE id=?1", [id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .optional()?
            .ok_or_else(|| anyhow!("item inconnu: {item_id}"))?;
        conn.execute(
            "UPDATE items SET content=?1, embedding=?2 WHERE id=?3",
            rusqlite::params![content, emb, id],
        )?;
        refresh_fts_row(&conn, id, &existing.0, content, &existing.1)?;
        conn.execute(
            "INSERT INTO mutations(op,node_id,content,ts) VALUES(?1,?2,?3,?4)",
            rusqlite::params!["update", existing.0, content, now()],
        )?;
        Ok(
            json!({ "ok": true, "item_id": format!("itm_{id}"), "node_id": existing.0, "content": content }),
        )
    }

    async fn move_item(&self, item_id: &str, node_id: &str) -> Result<Value> {
        let id = parse_item_rowid(item_id)?;
        let conn = self.conn.lock().unwrap();
        ensure_node(&conn, node_id)?;
        let existing = conn
            .query_row(
                "SELECT content, status, node_id FROM items WHERE id=?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("item inconnu: {item_id}"))?;
        conn.execute(
            "UPDATE items SET node_id=?1 WHERE id=?2",
            rusqlite::params![node_id, id],
        )?;
        refresh_fts_row(&conn, id, node_id, &existing.0, &existing.1)?;
        conn.execute(
            "INSERT INTO mutations(op,node_id,content,ts) VALUES(?1,?2,?3,?4)",
            rusqlite::params![
                "move",
                node_id,
                format!("{} -> {node_id}: {}", existing.2, existing.0),
                now()
            ],
        )?;
        Ok(
            json!({ "ok": true, "item_id": format!("itm_{id}"), "node_id": node_id, "from": existing.2 }),
        )
    }

    async fn delete_item(&self, item_id: &str, reason: Option<&str>) -> Result<Value> {
        let id = parse_item_rowid(item_id)?;
        let conn = self.conn.lock().unwrap();
        let existing = conn
            .query_row(
                "SELECT node_id, content FROM items WHERE id=?1 AND status!='deleted'",
                [id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow!("item inconnu ou deja supprime: {item_id}"))?;
        conn.execute("UPDATE items SET status='deleted' WHERE id=?1", [id])?;
        conn.execute("DELETE FROM items_fts WHERE rowid=?1", [id])?;
        conn.execute(
            "INSERT INTO mutations(op,node_id,content,ts) VALUES(?1,?2,?3,?4)",
            rusqlite::params![
                "delete",
                existing.0,
                format!(
                    "{} — {}",
                    reason.unwrap_or("delete_via_laruche"),
                    existing.1
                ),
                now()
            ],
        )?;
        Ok(
            json!({ "ok": true, "item_id": format!("itm_{id}"), "status": "deleted", "node_id": existing.0 }),
        )
    }

    async fn review_item(
        &self,
        item_id: &str,
        action: &str,
        reason: Option<&str>,
    ) -> Result<Value> {
        let id = parse_item_rowid(item_id)?;
        let conn = self.conn.lock().unwrap();
        let existing = conn
            .query_row(
                "SELECT node_id, content, status FROM items WHERE id=?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("item inconnu: {item_id}"))?;
        if existing.2 != "proposed" {
            return Err(anyhow!("item non propose: {item_id}"));
        }
        let new_status = match action {
            "accept" => "active",
            "reject" => "deleted",
            _ => return Err(anyhow!("action de revue invalide: {action}")),
        };
        conn.execute(
            "UPDATE items SET status=?1 WHERE id=?2",
            rusqlite::params![new_status, id],
        )?;
        refresh_fts_row(&conn, id, &existing.0, &existing.1, new_status)?;
        conn.execute(
            "INSERT INTO mutations(op,node_id,content,ts) VALUES(?1,?2,?3,?4)",
            rusqlite::params![
                action,
                existing.0,
                format!(
                    "{} — {}",
                    reason.unwrap_or("review_via_laruche"),
                    existing.1
                ),
                now()
            ],
        )?;
        Ok(
            json!({ "ok": true, "item_id": format!("itm_{id}"), "status": new_status, "node_id": existing.0 }),
        )
    }

    async fn list_proposed(&self, limit: Option<u8>) -> Result<Value> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, node_id, content, source, created_at FROM items
             WHERE status='proposed' ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit.unwrap_or(50) as i64], |r| {
            Ok(json!({
                "id": format!("itm_{}", r.get::<_, i64>(0)?),
                "node_id": r.get::<_, String>(1)?,
                "content": r.get::<_, String>(2)?,
                "source": r.get::<_, Option<String>>(3)?,
                "created_at": r.get::<_, i64>(4)?,
                "status": "proposed",
            }))
        })?;
        let items: Vec<Value> = rows.filter_map(|r| r.ok()).collect();
        Ok(json!({ "count": items.len(), "items": items }))
    }

    async fn suggest_nodes(&self, query: &str, limit: Option<u8>) -> Result<Value> {
        let conn = self.conn.lock().unwrap();
        let like = format!("%{}%", query.trim().to_lowercase());
        let sql = if query.trim().is_empty() {
            "SELECT n.id, n.label, n.one_liner, n.parent_id, n.importance,
                    COUNT(i.id) AS item_count
             FROM nodes n
             LEFT JOIN items i ON i.node_id=n.id AND i.status='active'
             GROUP BY n.id
             ORDER BY n.id
             LIMIT ?1"
        } else {
            "SELECT n.id, n.label, n.one_liner, n.parent_id, n.importance,
                    COUNT(i.id) AS item_count
             FROM nodes n
             LEFT JOIN items i ON i.node_id=n.id AND i.status='active'
             WHERE lower(n.id) LIKE ?2 OR lower(n.label) LIKE ?2 OR lower(n.one_liner) LIKE ?2
             GROUP BY n.id
             ORDER BY n.id
             LIMIT ?1"
        };
        let mut stmt = conn.prepare(sql)?;
        let mapper = |r: &rusqlite::Row<'_>| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "node_id": r.get::<_, String>(0)?,
                "label": r.get::<_, String>(1)?,
                "one_liner": r.get::<_, String>(2)?,
                "parent_id": r.get::<_, Option<String>>(3)?,
                "importance": r.get::<_, f32>(4)?,
                "item_count": r.get::<_, i64>(5)?,
            }))
        };
        let rows = if query.trim().is_empty() {
            stmt.query_map([limit.unwrap_or(12) as i64], mapper)?
        } else {
            stmt.query_map(rusqlite::params![limit.unwrap_or(12) as i64, like], mapper)?
        };
        let nodes: Vec<Value> = rows.filter_map(|r| r.ok()).collect();
        Ok(json!({ "nodes": nodes }))
    }

    async fn stats(&self) -> Result<Value> {
        let conn = self.conn.lock().unwrap();
        let one = |sql: &str| conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0);
        Ok(json!({
            "items_active": one("SELECT COUNT(*) FROM items WHERE status='active'"),
            "items_proposed": one("SELECT COUNT(*) FROM items WHERE status='proposed'"),
            "items_deleted": one("SELECT COUNT(*) FROM items WHERE status='deleted'"),
            "nodes": one("SELECT COUNT(*) FROM nodes"),
            "mutations": one("SELECT COUNT(*) FROM mutations"),
        }))
    }

    async fn mutations(&self, limit: Option<u8>) -> Result<Value> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT op, node_id, content, ts FROM mutations ORDER BY id DESC LIMIT ?1")?;
        let rows = stmt.query_map([limit.unwrap_or(50) as i64], |r| {
            Ok(json!({
                "op": r.get::<_, String>(0)?,
                "node_id": r.get::<_, Option<String>>(1)?,
                "content": r.get::<_, Option<String>>(2)?,
                "ts": r.get::<_, i64>(3)?,
            }))
        })?;
        let entries: Vec<Value> = rows.filter_map(|r| r.ok()).collect();
        Ok(json!({ "count": entries.len(), "mutations": entries }))
    }

    async fn dream(&self) -> Result<Value> {
        let conn = self.conn.lock().unwrap();
        let dups: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(c-1),0) FROM (SELECT COUNT(*) c FROM items \
                 WHERE status='active' GROUP BY node_id, content HAVING c>1)",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let mut suggestions = Vec::new();
        let mut dup_stmt = conn.prepare(
            "SELECT node_id, content, COUNT(*) c FROM items
             WHERE status='active' GROUP BY node_id, content HAVING c>1
             ORDER BY c DESC LIMIT 12",
        )?;
        let dup_rows = dup_stmt.query_map([], |r| {
            let node_id: String = r.get(0)?;
            let content: String = r.get(1)?;
            let count: i64 = r.get(2)?;
            Ok(json!({
                "kind": "duplicate",
                "severity": "medium",
                "node_id": node_id,
                "count": count,
                "message": format!("Doublon exact dans {node_id}: {}", content.chars().take(80).collect::<String>())
            }))
        })?;
        suggestions.extend(dup_rows.filter_map(|r| r.ok()));

        let mut overload_stmt = conn.prepare(
            "SELECT node_id, COUNT(*) c FROM items
             WHERE status='active' GROUP BY node_id HAVING c>12
             ORDER BY c DESC LIMIT 12",
        )?;
        let overload_rows = overload_stmt.query_map([], |r| {
            let node_id: String = r.get(0)?;
            let count: i64 = r.get(1)?;
            Ok(json!({
                "kind": "overloaded",
                "severity": "low",
                "node_id": node_id,
                "count": count,
                "message": format!("{node_id} contient {count} items actifs; envisager des sous-noeuds.")
            }))
        })?;
        suggestions.extend(overload_rows.filter_map(|r| r.ok()));

        let orphan_items: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM items i
                 LEFT JOIN nodes n ON n.id=i.node_id
                 WHERE i.status='active' AND n.id IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if orphan_items > 0 {
            suggestions.push(json!({
                "kind": "orphan",
                "severity": "medium",
                "count": orphan_items,
                "message": format!("{orphan_items} item(s) actifs pointent vers un noeud absent.")
            }));
        }

        Ok(json!({ "suggestions": suggestions, "duplicates": dups, "orphan_items": orphan_items }))
    }

    async fn export_okf(&self, dir: &Path) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        std::fs::create_dir_all(dir)?;
        let ts = chrono::Utc::now().to_rfc3339();

        let mut nstmt = conn.prepare("SELECT id, label, one_liner FROM nodes")?;
        let nodes: Vec<(String, String, String)> = nstmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .filter_map(|x| x.ok())
            .collect();
        drop(nstmt);

        let mut files = 0usize;
        for (id, label, one_liner) in &nodes {
            // node_id pointé → arborescence de dossiers (OKF), un index.md par node.
            let node_dir = dir.join(id.replace('.', "/"));
            std::fs::create_dir_all(&node_dir)?;

            let mut istmt = conn.prepare(
                "SELECT content, source FROM items WHERE node_id=?1 AND status='active' ORDER BY created_at",
            )?;
            let items: Vec<(String, Option<String>)> = istmt
                .query_map([id.as_str()], |r| Ok((r.get(0)?, r.get(1)?)))?
                .filter_map(|x| x.ok())
                .collect();
            drop(istmt);

            let mut md = String::new();
            md.push_str("---\n");
            md.push_str("type: memory-node\n"); // `type` = seul champ OKF obligatoire
            md.push_str(&format!("title: {}\n", yaml_q(label)));
            if !one_liner.is_empty() {
                md.push_str(&format!("description: {}\n", yaml_q(one_liner)));
            }
            md.push_str(&format!("id: {}\n", yaml_q(id)));
            md.push_str(&format!("timestamp: {ts}\n"));
            md.push_str("---\n\n");
            md.push_str(&format!("# {label}\n\n"));
            for (content, source) in &items {
                md.push_str(&format!("- {}", content.replace('\n', " ")));
                if let Some(s) = source.as_ref().filter(|s| !s.is_empty()) {
                    md.push_str(&format!("  _(source: {s})_"));
                }
                md.push('\n');
            }
            std::fs::write(node_dir.join("index.md"), md)?;
            files += 1;
        }

        // Index racine du bundle (liens markdown = graphe OKF).
        let mut root = String::from("---\ntype: index\ntitle: \"LaRuche memory bundle\"\n");
        root.push_str(&format!(
            "timestamp: {ts}\n---\n\n# LaRuche — bundle OKF\n\n"
        ));
        for (id, label, _) in &nodes {
            root.push_str(&format!("- [{label}]({}/index.md)\n", id.replace('.', "/")));
        }
        std::fs::write(dir.join("index.md"), root)?;
        files += 1;

        Ok(files)
    }

    async fn import_okf(&self, dir: &Path) -> Result<usize> {
        let mut files = Vec::new();
        collect_index_md(dir, &mut files);
        let mut imported = 0usize;
        for f in files {
            let content = std::fs::read_to_string(&f).unwrap_or_default();
            if content.contains("type: index") {
                continue; // index racine du bundle, pas un node
            }
            // node_id depuis le frontmatter `id:`, sinon dérivé du chemin relatif.
            let rel = f
                .parent()
                .and_then(|p| p.strip_prefix(dir).ok())
                .map(|p| p.to_string_lossy().replace(['/', '\\'], "."))
                .unwrap_or_default();
            let (node_id, body_items) = parse_okf(&content, &rel);
            if node_id.is_empty() {
                continue;
            }
            for c in body_items {
                let emb = self.embed_opt(&c).await.map(|v| vec_to_blob(&v));
                self.insert(
                    &MemoryItem::new(node_id.clone(), c).with_source("okf-import"),
                    "active",
                    emb,
                )?;
                imported += 1;
            }
        }
        Ok(imported)
    }

    async fn health(&self) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        Ok(conn
            .query_row("SELECT 1", [], |r| r.get::<_, i64>(0))
            .is_ok())
    }
}
