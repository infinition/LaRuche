//! [`SqliteBackend`]: 100% Rust persistent memory engine: SQLite + FTS5 + audit
//! + hybrid semantic/lexical search when an [`Embedder`] is wired in.
//!
//! Faithful port of paradigm's `sqlite-store` part (durable storage, FTS5 BM25,
//! proposed items excluded, audit log) + the semantic layer (T1). Single binary
//! (rusqlite `bundled`). Without embedder: FTS5 lexical recall; with embedder: hybrid.

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
    /// Optional LLM arbiter for near-miss contradictions at write time (see [`crate::Arbitre`]).
    arbitre: std::sync::RwLock<Option<Arc<dyn crate::Arbitre>>>,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Converts a dotted node_id into a safe folder path (one segment per level),
/// replacing characters forbidden in a Windows file name (`< > : " | ? * / \`
/// and control characters) with `_`. Prevents polluted nodes (e.g. `a|b`) from breaking the export.
fn safe_path_segments(id: &str) -> String {
    id.split('.')
        .map(|seg| {
            seg.chars()
                .map(|c| {
                    if matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '/' | '\\')
                        || c.is_control()
                    {
                        '_'
                    } else {
                        c
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Escapes a value for a quoted YAML scalar (OKF frontmatter).
fn yaml_q(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Recursively collects all `index.md` files of an OKF bundle.
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

/// Parses an OKF `index.md`: (node_id, body items). `fallback_id` if no `id:` in frontmatter.
fn parse_okf(content: &str, fallback_id: &str) -> (String, Vec<String>) {
    let mut node_id = fallback_id.to_string();
    let mut items = Vec::new();
    let mut seen_front = 0u8; // 1 = inside frontmatter, >=2 = body
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
            // Seul un `- ` en COLONNE 0 ouvre un item; tout ce qui est indente en
            // dessous lui appartient. Sans cette distinction, une liste imbriquee
            // dans un item se serait relue comme autant d'items separes, et un item
            // sur plusieurs lignes n'aurait jamais pu revenir entier.
            if let Some(rest) = line.strip_prefix("- ") {
                let c = rest.split("  _(source:").next().unwrap_or(rest).trim_end();
                items.push(c.to_string());
            } else if !items.is_empty() {
                let indente = line.starts_with("  ") || line.starts_with('\t');
                if indente {
                    let suite = line.strip_prefix("  ").unwrap_or_else(|| line.trim_start());
                    // La ligne de source est une decoration de l'export, pas du contenu.
                    if !suite.trim_start().starts_with("_(source:") {
                        let dernier = items.last_mut().expect("items non vide");
                        dernier.push('\n');
                        dernier.push_str(suite.trim_end());
                    }
                } else if line.trim().is_empty() {
                    // Une ligne vide au milieu d'un item en fait partie; une ligne vide
                    // suivie de texte non indente le termine, et c'est le `- ` suivant
                    // ou la fin du fichier qui tranche. On la garde en attente.
                    let dernier = items.last_mut().expect("items non vide");
                    dernier.push('\n');
                }
            }
        }
    }
    // Les lignes vides gardees en attente ne doivent pas trainer en fin d'item.
    for it in items.iter_mut() {
        while it.ends_with('\n') {
            it.pop();
        }
    }
    items.retain(|c| !c.trim().is_empty());
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
    b.as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect()
}

fn node_parent_id(node_id: &str) -> Option<String> {
    let trimmed = node_id.trim_matches('.');
    trimmed
        .rfind('.')
        .map(|idx| trimmed[..idx].to_string())
        .filter(|s| !s.is_empty())
}

/// LIKE pattern matching a node's subtree (`prefix.%`), with the prefix escaped so the
/// snake_case `_` (and `%`/`\`) in node ids are matched literally instead of as wildcards.
/// Must be used with `ESCAPE '\\'` in the query, otherwise deleting/renaming `a_b` would
/// also hit unrelated subtrees like `axb.*`.
fn subtree_like(prefix: &str) -> String {
    format!(
        "{}.%",
        prefix.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
    )
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
        .map(|p| format!("Subnode of {p}"))
        .unwrap_or_else(|| "Root node".to_string());
    conn.execute(
        "INSERT OR IGNORE INTO nodes(id,parent_id,label,one_liner,importance,created_at,updated_at)
         VALUES(?1,?2,?3,?4,?5,?6,?6)",
        rusqlite::params![node_id, parent_id, label, one_liner, 0.5f32, now()],
    )?;
    Ok(())
}

fn node_json(conn: &Connection, node_id: &str) -> Result<Value> {
    let row = conn
        .query_row(
            "SELECT label, one_liner, parent_id, importance, created_at, COALESCE(updated_at, created_at), source \
             FROM nodes WHERE id=?1",
            [node_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, f32>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                    r.get::<_, Option<i64>>(5)?,
                    r.get::<_, Option<String>>(6)?,
                ))
            },
        )
        .optional()?;
    let (label, one_liner, parent_id, importance, created_at, updated_at, source) = row
        .unwrap_or_else(|| {
            (
                node_label(node_id),
                String::new(),
                node_parent_id(node_id),
                0.5,
                None,
                None,
                None,
            )
        });
    Ok(json!({
        "id": node_id,
        "node_id": node_id,
        "label": label,
        "one_liner": one_liner,
        "parent_id": parent_id,
        "importance": importance,
        "created_at": created_at,
        "updated_at": updated_at,
        "source": source,
    }))
}

fn parse_item_rowid(item_id: &str) -> Result<i64> {
    item_id
        .strip_prefix("itm_")
        .unwrap_or(item_id)
        .parse::<i64>()
        .map_err(|_| anyhow!("invalid item_id: {item_id}"))
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
    /// Opens the database (FTS5 lexical recall only).
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_inner(path, None)
    }

    /// Opens the database with hybrid semantic recall via the provided embedder.
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
               source TEXT,
               created_at INTEGER NOT NULL);
             CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(content, node_id);
             CREATE TABLE IF NOT EXISTS mutations(
               id INTEGER PRIMARY KEY,
               op TEXT NOT NULL, node_id TEXT, content TEXT, ts INTEGER NOT NULL);",
        )?;
        // Migration: last-modified timestamp (ignore error if already present).
        let _ = conn.execute("ALTER TABLE items ADD COLUMN updated_at INTEGER", []);
        let _ = conn.execute("ALTER TABLE nodes ADD COLUMN updated_at INTEGER", []);
        let _ = conn.execute("ALTER TABLE nodes ADD COLUMN source TEXT", []);
        // Mutation actor (source/reason) for the Feed (User vs LaRuche).
        let _ = conn.execute("ALTER TABLE mutations ADD COLUMN src TEXT", []);
        // Value & usage signals (priority decay: ranking, never deletion).
        let _ = conn.execute("ALTER TABLE items ADD COLUMN importance REAL", []);
        let _ = conn.execute("ALTER TABLE items ADD COLUMN confidence REAL", []);
        let _ = conn.execute("ALTER TABLE items ADD COLUMN access_count INTEGER DEFAULT 0", []);
        let _ = conn.execute("ALTER TABLE items ADD COLUMN accessed_at INTEGER", []);
        // Repair: drop FTS rows whose item no longer exists. Past hard deletes left them
        // behind, and because rowids get reused each orphan is a landmine that makes one
        // future write fail with "constraint failed" - one write in two, on the base this
        // was found on (372 FTS rows for 160 items). Cheap, idempotent, runs at open.
        match conn.execute(
            "DELETE FROM items_fts WHERE rowid NOT IN (SELECT id FROM items)",
            [],
        ) {
            Ok(0) => {}
            Ok(n) => tracing::info!("memory: {n} orphan FTS row(s) cleared"),
            Err(e) => tracing::warn!("memory: could not clear orphan FTS rows: {e}"),
        }

        Ok(Self {
            conn: Mutex::new(conn),
            embedder,
            arbitre: std::sync::RwLock::new(None),
        })
    }

    /// Wires the contradiction arbiter (aux LLM), inherent helper.
    fn poser_arbitre(&self, arbitre: Arc<dyn crate::Arbitre>) {
        if let Ok(mut a) = self.arbitre.write() {
            *a = Some(arbitre);
        }
    }

    fn insert(&self, item: &MemoryItem, status: &str, embedding: Option<Vec<u8>>) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        ensure_node(&conn, &item.node_id)?;
        conn.execute(
            "INSERT INTO items(node_id,content,source,status,embedding,created_at,updated_at,importance,confidence) \
             VALUES(?1,?2,?3,?4,?5,?6,?6,?7,?8)",
            rusqlite::params![
                item.node_id,
                item.content,
                item.source,
                status,
                embedding,
                now(),
                item.importance,
                item.confidence
            ],
        )?;
        let id = conn.last_insert_rowid();
        if status == "active" {
            // `items.id` is INTEGER PRIMARY KEY WITHOUT autoincrement, so SQLite REUSES
            // the rowids of deleted rows. Any stale FTS row left behind by a hard delete
            // then collides with the new item and the whole write fails with a bare
            // "constraint failed" - intermittently, depending on which rowid comes up.
            // Clearing the slot first makes the insert idempotent whatever the history.
            conn.execute("DELETE FROM items_fts WHERE rowid=?1", [id])?;
            conn.execute(
                "INSERT INTO items_fts(rowid,content,node_id) VALUES(?1,?2,?3)",
                rusqlite::params![id, item.content, item.node_id],
            )?;
        }
        conn.execute(
            "INSERT INTO mutations(op,node_id,content,ts,src) VALUES(?1,?2,?3,?4,?5)",
            rusqlite::params![
                if status == "active" {
                    "write"
                } else {
                    "propose"
                },
                item.node_id,
                item.content,
                now(),
                item.source
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
        let qvec = self.embed_opt(query).await; // computed BEFORE the lock (no await under mutex)
        let qtoks = tokens(query);

        let conn = self.conn.lock().unwrap();

        // Cognitive activation: each node lights up based on query/path-label overlap
        // + its importance. Items of a relevant subtree are then boosted (paradigm-style).
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
        // Effective activation of an item = its node's activation + decaying share of the parent.
        let activation_of = |node: &str| -> f32 {
            let na = node_act.get(node).copied().unwrap_or(0.0);
            let pa = node
                .rsplit_once('.')
                .and_then(|(p, _)| node_act.get(p))
                .copied()
                .unwrap_or(0.0);
            na + 0.3 * pa
        };

        // Value/usage/freshness bonus - the "priority decay": an item never expires,
        // but a stored-important, frequently-recalled (Hebbian) or recently-updated
        // fact outranks a stale never-used one.
        let now_ts = now();
        let bonus = |imp: Option<f32>, acces: i64, maj: Option<i64>| -> f32 {
            let valeur = 0.2 * imp.unwrap_or(0.5);
            let usage = (0.05 * (acces.max(0) as f32).ln_1p()).min(0.15);
            let age_jours = maj
                .map(|u| ((now_ts - u) as f32 / 86_400.0).max(0.0))
                .unwrap_or(365.0);
            let fraicheur = 0.1 * (-age_jours / 90.0).exp();
            valeur + usage + fraicheur
        };
        // id -> (score, node, content) - semantic and lexical branches MERGE here
        // (max score wins): an item without an embedding stays reachable via FTS.
        let mut fusion: std::collections::HashMap<i64, (f32, String, String)> =
            std::collections::HashMap::new();

        if let Some(qv) = &qvec {
            // Semantic branch: cosine + small lexical flag + activation + bonus.
            // `capacities.*`/`system.*` are system-managed projections (skills catalog,
            // prompts) with their OWN injection channels: surfacing their big bodies
            // here drowned real facts (observed: a GPU question recalled skill guides).
            let mut stmt = conn.prepare(
                "SELECT id, node_id, content, embedding, importance, COALESCE(access_count,0), updated_at \
                 FROM items WHERE status='active' AND embedding IS NOT NULL \
                 AND node_id NOT LIKE 'capacities.%' AND node_id NOT LIKE 'system.%'",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Vec<u8>>(3)?,
                    r.get::<_, Option<f32>>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, Option<i64>>(6)?,
                ))
            })?;
            for row in rows {
                let (id, node, content, blob, imp, acces, maj) = row?;
                let sem = cosine(qv, &blob_to_vec(&blob));
                let hay = format!("{node} {content}").to_lowercase();
                let lex = if qtoks.iter().any(|t| hay.contains(t)) { 0.3 } else { 0.0 };
                // RELEVANCE gates; the value/usage bonus only RE-RANKS relevant
                // hits (a fresh but off-topic item must never surface).
                let pertinence = 0.7 * sem + lex + 0.25 * activation_of(&node);
                if pertinence > 0.05 {
                    fusion.insert(id, (pertinence + bonus(imp, acces, maj), node, content));
                }
            }
        }
        if !qtoks.is_empty() {
            // Lexical branch (ALWAYS runs): FTS5 BM25. Catches items with no
            // embedding (written while the embedder was down) and exact wording.
            let match_expr = qtoks
                .iter()
                .map(|t| format!("\"{t}\"*"))
                .collect::<Vec<_>>()
                .join(" OR ");
            let mut stmt = conn.prepare(
                "SELECT i.id, i.node_id, i.content, i.importance, COALESCE(i.access_count,0), i.updated_at \
                 FROM items_fts f JOIN items i ON i.id = f.rowid \
                 WHERE f.items_fts MATCH ?1 AND i.status='active' \
                 AND i.node_id NOT LIKE 'capacities.%' AND i.node_id NOT LIKE 'system.%' \
                 ORDER BY bm25(items_fts) LIMIT ?2",
            )?;
            let rows = stmt.query_map(
                rusqlite::params![match_expr, (limit * 3) as i64],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, Option<f32>>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, Option<i64>>(5)?,
                    ))
                },
            )?;
            // Base below a strong semantic match, above a weak one: exact wording
            // competes without drowning meaning.
            let base = if qvec.is_some() { 0.55 } else { 1.0 };
            // Noise guard: FTS `OR`s the tokens, so a SINGLE common word ("local",
            // "recherche") drags in off-topic items. When the query is rich (>=3
            // tokens) require the candidate to match at least 2 distinct query tokens.
            // Short queries keep the permissive OR (recall matters more there).
            let min_tokens = if qtoks.len() >= 3 { 2 } else { 1 };
            for row in rows {
                let (id, node, content, imp, acces, maj) = row?;
                let hay = format!("{node} {content}").to_lowercase();
                let matched = qtoks.iter().filter(|t| hay.contains(*t)).count();
                if matched < min_tokens {
                    continue;
                }
                let score = base + 0.25 * activation_of(&node) + bonus(imp, acces, maj);
                let e = fusion.entry(id).or_insert((0.0, node, content));
                if score > e.0 {
                    e.0 = score;
                }
            }
        }

        let mut hits: Vec<(f32, i64, String, String)> = fusion
            .into_iter()
            .map(|(id, (s, n, c))| (s, id, n, c))
            .collect();
        hits.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        hits.truncate(limit);

        let mut nodes = Vec::new();
        let mut items = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (_, id, node, content) in hits {
            if opts.sans_trace {
                // Hebbian level 2 caller: freshness only. The weight comes later,
                // via renforcer(), for the items actually used in the answer.
                let _ = conn.execute(
                    "UPDATE items SET accessed_at=?1 WHERE id=?2",
                    rusqlite::params![now_ts, id],
                );
            } else {
                // Hebbian trace: being recalled strengthens future ranking.
                let _ = conn.execute(
                    "UPDATE items SET access_count = COALESCE(access_count,0)+1, accessed_at=?1 WHERE id=?2",
                    rusqlite::params![now_ts, id],
                );
            }
            if seen.insert(node.clone()) {
                nodes.push(node_json(&conn, &node)?);
            }
            // Cap per-item content: one verbose item must not eat the whole recall
            // budget (full documents stay reachable via memory_read_node).
            let apercu: String = content.chars().take(600).collect();
            let contenu = if apercu.len() < content.len() {
                format!("{apercu} …[truncated: read node {node} for the rest]")
            } else {
                content
            };
            items.push(json!({ "id": format!("itm_{id}"), "node_id": node, "content": contenu }));
        }
        Ok(Self::pack(nodes, items))
    }

    async fn write(&self, item: MemoryItem) -> Result<Value> {
        let emb_vec = self.embed_opt(&item.content).await;
        // Facts do not rot with time - they rot when REPLACED. At write time:
        // exact duplicate (same node) -> no-op; near-duplicate (cosine > 0.88, same
        // node) -> the OLD version is marked `superseded` (kept for audit, excluded
        // from recall). This is what keeps the map clean without any hard decay.
        // Items in the similarity band the cosine pass could not decide: (id, content).
        let mut ambigus: Vec<(i64, String)> = Vec::new();
        {
            let conn = self.conn.lock().unwrap();
            if let Ok(id) = conn.query_row(
                "SELECT id FROM items WHERE node_id=?1 AND content=?2 AND status='active'",
                rusqlite::params![item.node_id, item.content],
                |r| r.get::<_, i64>(0),
            ) {
                return Ok(json!({
                    "ok": true, "item_id": format!("itm_{id}"),
                    "node_id": item.node_id, "dedup": true
                }));
            }
            if let Some(qv) = &emb_vec {
                // Supersede scope = the whole ROOT DOMAIN (`hardware.*`), not just the
                // exact node: the model files the same fact under sibling nodes
                // (observed: 4070 Ti in hardware.local_model_setup, its replacement
                // 5080 in hardware.gpu - both stayed active). Same-node matches keep
                // the 0.88 threshold; cross-node needs a slightly stronger 0.90.
                let domaine = format!(
                    "{}.%",
                    item.node_id.split('.').next().unwrap_or(&item.node_id)
                );
                let mut remplaces: Vec<i64> = Vec::new();
                {
                    let mut stmt = conn.prepare(
                        "SELECT id, node_id, content, embedding FROM items \
                         WHERE (node_id=?1 OR node_id LIKE ?2) AND status='active' AND embedding IS NOT NULL",
                    )?;
                    let rows = stmt.query_map(rusqlite::params![item.node_id, domaine], |r| {
                        Ok((
                            r.get::<_, i64>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, Vec<u8>>(3)?,
                        ))
                    })?;
                    for row in rows {
                        let (id, node, content, blob) = row?;
                        // Thresholds CALIBRATED on real nomic-embed-text measurements
                        // (2026-07-02): a same-fact PARAPHRASE lands at ~0.86 (0.88
                        // silently missed it), unrelated facts at ~0.48, and a fact
                        // UPDATE (4070→5080) at ~0.71 - updates are a contradiction,
                        // not a similarity, so cosine alone cannot catch them.
                        let sim = cosine(qv, &blob_to_vec(&blob));
                        let seuil = if node == item.node_id { 0.83 } else { 0.85 };
                        if sim > seuil {
                            remplaces.push(id);
                        } else if sim >= 0.62 {
                            // AMBIGUITY BAND: moderately similar, might be an update or
                            // just shared vocabulary. Defer to the LLM arbiter (below).
                            ambigus.push((id, content));
                        }
                    }
                }
                for id in remplaces {
                    conn.execute(
                        "UPDATE items SET status='superseded', updated_at=?1 WHERE id=?2",
                        rusqlite::params![now(), id],
                    )?;
                    conn.execute(
                        "DELETE FROM items_fts WHERE rowid=?1",
                        rusqlite::params![id],
                    )?;
                    conn.execute(
                        "INSERT INTO mutations(op,node_id,content,ts,src) VALUES('supersede',?1,?2,?3,?4)",
                        rusqlite::params![
                            item.node_id,
                            format!("itm_{id} superseded by a newer fact"),
                            now(),
                            item.source
                        ],
                    )?;
                }
            }
        }
        // ARBITER pass (async, LLM): for band candidates cosine could not resolve, ask
        // whether the new fact REPLACES the existing one (an UPDATE like 4070 -> 5080).
        // Done OUTSIDE the connection lock (no await under a std::sync lock). Best-effort:
        // no arbiter wired, or a Distinct verdict, leaves the old fact untouched.
        if !ambigus.is_empty() {
            let arbitre = self.arbitre.read().ok().and_then(|a| a.clone());
            if let Some(arbitre) = arbitre {
                let mut a_remplacer: Vec<i64> = Vec::new();
                for (id, existant) in &ambigus {
                    if arbitre.trancher(existant, &item.content).await
                        == crate::VerdictArbitre::Remplace
                    {
                        a_remplacer.push(*id);
                    }
                }
                if !a_remplacer.is_empty() {
                    let conn = self.conn.lock().unwrap();
                    for id in a_remplacer {
                        conn.execute(
                            "UPDATE items SET status='superseded', updated_at=?1 WHERE id=?2",
                            rusqlite::params![now(), id],
                        )?;
                        conn.execute("DELETE FROM items_fts WHERE rowid=?1", rusqlite::params![id])?;
                        conn.execute(
                            "INSERT INTO mutations(op,node_id,content,ts,src) VALUES('supersede',?1,?2,?3,?4)",
                            rusqlite::params![
                                item.node_id,
                                format!("itm_{id} superseded (arbiter: fact update)"),
                                now(),
                                item.source
                            ],
                        )?;
                    }
                }
            }
        }
        let id = self.insert(&item, "active", emb_vec.map(|v| vec_to_blob(&v)))?;
        Ok(json!({ "ok": true, "item_id": format!("itm_{id}"), "node_id": item.node_id }))
    }

    async fn propose_write(&self, item: MemoryItem) -> Result<Value> {
        let emb = self.embed_opt(&item.content).await.map(|v| vec_to_blob(&v));
        let id = self.insert(&item, "proposed", emb)?;
        Ok(json!({ "ok": true, "item_id": format!("itm_{id}"), "status": "proposed" }))
    }

    async fn read_node(&self, node_id: &str) -> Result<Value> {
        let conn = self.conn.lock().unwrap();

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
            "SELECT id, content, source, created_at, COALESCE(updated_at, created_at) FROM items
             WHERE node_id=?1 AND status='active' ORDER BY created_at",
        )?;
        let rows = stmt.query_map([node_id], |r| {
            Ok(json!({
                "id": format!("itm_{}", r.get::<_,i64>(0)?),
                "node_id": node_id,
                "content": r.get::<_,String>(1)?,
                "source": r.get::<_, Option<String>>(2)?,
                "created_at": r.get::<_, i64>(3)?,
                "updated_at": r.get::<_, i64>(4)?,
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
            .ok_or_else(|| anyhow!("unknown item: {item_id}"))?;
        conn.execute(
            "UPDATE items SET content=?1, embedding=?2, updated_at=?3 WHERE id=?4",
            rusqlite::params![content, emb, now(), id],
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
            .ok_or_else(|| anyhow!("unknown item: {item_id}"))?;
        conn.execute(
            "UPDATE items SET node_id=?1, updated_at=?2 WHERE id=?3",
            rusqlite::params![node_id, now(), id],
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
            .ok_or_else(|| anyhow!("unknown or already deleted item: {item_id}"))?;
        conn.execute("UPDATE items SET status='deleted' WHERE id=?1", [id])?;
        conn.execute("DELETE FROM items_fts WHERE rowid=?1", [id])?;
        conn.execute(
            "INSERT INTO mutations(op,node_id,content,ts,src) VALUES(?1,?2,?3,?4,?5)",
            rusqlite::params![
                "delete",
                existing.0,
                format!(
                    "{}: {}",
                    reason.unwrap_or("delete_via_laruche"),
                    existing.1
                ),
                now(),
                reason
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
            .ok_or_else(|| anyhow!("unknown item: {item_id}"))?;
        if existing.2 != "proposed" {
            return Err(anyhow!("item not proposed: {item_id}"));
        }
        let new_status = match action {
            "accept" => "active",
            "reject" => "deleted",
            _ => return Err(anyhow!("invalid review action: {action}")),
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
                    "{}: {}",
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

    async fn purger_tombes_skills(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        // FTS first, and by rowid: dropping the items first would leave nothing to join
        // on, and an orphan FTS row poisons the rowid it sits on for the next insert.
        conn.execute(
            "DELETE FROM items_fts WHERE rowid IN \
             (SELECT id FROM items WHERE status='deleted' AND source='skill-file')",
            [],
        )?;
        let n = conn.execute(
            "DELETE FROM items WHERE status='deleted' AND source='skill-file'",
            [],
        )? as u64;
        if n > 0 {
            // The rows are gone but the file keeps its size until the pages are
            // reclaimed, and the whole point here is the 364 MB on disk.
            let _ = conn.execute_batch("VACUUM");
        }
        Ok(n)
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

    /// Le fil ne veut que l'activite: on ecarte le bruit DANS la requete, pour que la
    /// fenetre de lecture soit remplie d'evenements reels et non de reamorcage.
    ///
    /// Ecarte: l'indexation des outils et la synchronisation disque<->SQL des skills
    /// (plusieurs dizaines de lignes a chaque demarrage), ainsi que la (re)creation
    /// des branches `system` et `capacities`, identiques a chaque fois.
    async fn mutations_activite(&self, limit: Option<u32>) -> Result<Value> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT op, node_id, content, ts, src FROM mutations              WHERE COALESCE(src,'') NOT IN                    ('tool-registry','seed','skill-file','skill-file-sync','skill-file-watch')                AND NOT (op IN ('create_node','update_node')                         AND (node_id LIKE 'system%' OR node_id LIKE 'capacities%'))              ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit.unwrap_or(50) as i64], |r| {
            Ok(json!({
                "op": r.get::<_, String>(0)?,
                "node_id": r.get::<_, Option<String>>(1)?,
                "content": r.get::<_, Option<String>>(2)?,
                "ts": r.get::<_, i64>(3)?,
                "src": r.get::<_, Option<String>>(4)?,
            }))
        })?;
        let entries: Vec<Value> = rows.filter_map(|r| r.ok()).collect();
        Ok(json!({ "mutations": entries }))
    }

    async fn mutations(&self, limit: Option<u8>) -> Result<Value> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT op, node_id, content, ts, src FROM mutations ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit.unwrap_or(50) as i64], |r| {
            Ok(json!({
                "op": r.get::<_, String>(0)?,
                "node_id": r.get::<_, Option<String>>(1)?,
                "content": r.get::<_, Option<String>>(2)?,
                "ts": r.get::<_, i64>(3)?,
                "src": r.get::<_, Option<String>>(4)?,
            }))
        })?;
        let entries: Vec<Value> = rows.filter_map(|r| r.ok()).collect();
        Ok(json!({ "count": entries.len(), "mutations": entries }))
    }

    async fn grep(&self, pattern: &str, limit: Option<u8>) -> Result<Value> {
        let conn = self.conn.lock().unwrap();
        let like = format!(
            "%{}%",
            pattern.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
        );
        let mut stmt = conn.prepare(
            "SELECT id, node_id, content FROM items WHERE status='active' AND content LIKE ?1 ESCAPE '\\' ORDER BY id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![like, limit.unwrap_or(30) as i64], |r| {
            let id: i64 = r.get(0)?;
            Ok(json!({
                "id": format!("itm_{id}"),
                "node_id": r.get::<_, String>(1)?,
                "content": r.get::<_, String>(2)?,
            }))
        })?;
        let items: Vec<Value> = rows.filter_map(|x| x.ok()).collect();
        Ok(json!({ "count": items.len(), "items": items }))
    }

    async fn renforcer(&self, item_ids: &[String]) -> Result<usize> {
        if item_ids.is_empty() {
            return Ok(0);
        }
        let conn = self.conn.lock().unwrap();
        let now_ts = now();
        let mut n = 0usize;
        for id in item_ids {
            if let Ok(rowid) = parse_item_rowid(id) {
                n += conn
                    .execute(
                        "UPDATE items SET access_count = COALESCE(access_count,0)+1, accessed_at=?1 \
                         WHERE id=?2 AND status='active'",
                        rusqlite::params![now_ts, rowid],
                    )
                    .unwrap_or(0);
            }
        }
        Ok(n)
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
                "message": format!("Exact duplicate in {node_id}: {}", content.chars().take(80).collect::<String>())
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
                "message": format!("{node_id} contains {count} active items; consider subnodes.")
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
                "message": format!("{orphan_items} active item(s) point to a missing node.")
            }));
        }

        Ok(json!({ "suggestions": suggestions, "duplicates": dups, "orphan_items": orphan_items }))
    }

    async fn export_okf(&self, dir: &Path, prefix: Option<&str>) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        std::fs::create_dir_all(dir)?;
        let ts = chrono::Utc::now().to_rfc3339();

        // Optional scope: one node + its subtree (`id = prefix OR id LIKE prefix.%`).
        let nodes: Vec<(String, String, String)> = match prefix.map(|p| p.trim_matches('.')) {
            Some(p) if !p.is_empty() => {
                let like = subtree_like(p);
                let mut nstmt = conn
                    .prepare("SELECT id, label, one_liner FROM nodes WHERE id=?1 OR id LIKE ?2 ESCAPE '\\'")?;
                let v: Vec<(String, String, String)> = nstmt
                    .query_map(rusqlite::params![p, like], |r| {
                        Ok((r.get(0)?, r.get(1)?, r.get(2)?))
                    })?
                    .filter_map(|x| x.ok())
                    .collect();
                v
            }
            _ => {
                let mut nstmt = conn.prepare("SELECT id, label, one_liner FROM nodes")?;
                let v: Vec<(String, String, String)> = nstmt
                    .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                    .filter_map(|x| x.ok())
                    .collect();
                v
            }
        };

        let mut files = 0usize;
        for (id, label, one_liner) in &nodes {
            // dotted node_id: folder tree (OKF), one index.md per node.
            let node_dir = dir.join(safe_path_segments(id));
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
            md.push_str("type: memory-node\n"); // `type` = the only mandatory OKF field
            md.push_str(&format!("title: {}\n", yaml_q(label)));
            if !one_liner.is_empty() {
                md.push_str(&format!("description: {}\n", yaml_q(one_liner)));
            }
            md.push_str(&format!("id: {}\n", yaml_q(id)));
            md.push_str(&format!("timestamp: {ts}\n"));
            md.push_str("---\n\n");
            md.push_str(&format!("# {label}\n\n"));
            for (content, source) in &items {
                // Un item sur plusieurs lignes garde ses lignes.
                //
                // Elles etaient ecrasees par un `replace('\n', " ")`, et l'aller-retour
                // ne fonctionnait QUE grace a cet aplatissement, puisque l'import lisait
                // un item par ligne `- `. Autrement dit, chaque cycle export/import
                // detruisait definitivement la mise en forme: perte de donnees, et pas
                // seulement d'affichage, dans un format dont c'est precisement la raison
                // d'etre de faire voyager du markdown.
                //
                // La continuation est indentee de deux espaces: c'est la continuation de
                // liste standard, elle se rend correctement partout (GitHub compris), et
                // elle se relit sans ambiguite puisque seul un `- ` en colonne 0 ouvre un
                // item.
                let mut lignes = content.lines();
                let premiere = lignes.next().unwrap_or("");
                md.push_str(&format!("- {premiere}\n"));
                for l in lignes {
                    if l.trim().is_empty() {
                        md.push('\n');
                    } else {
                        md.push_str(&format!("  {l}\n"));
                    }
                }
                if let Some(s) = source.as_ref().filter(|s| !s.is_empty()) {
                    md.push_str(&format!("  _(source: {s})_\n"));
                }
            }
            std::fs::write(node_dir.join("index.md"), md)?;
            files += 1;
        }

        // Bundle root index (markdown links = OKF graph).
        let mut root = String::from("---\ntype: index\ntitle: \"LaRuche memory bundle\"\n");
        root.push_str(&format!(
            "timestamp: {ts}\n---\n\n# LaRuche - OKF bundle\n\n"
        ));
        for (id, label, _) in &nodes {
            root.push_str(&format!(
                "- [{label}]({}/index.md)\n",
                safe_path_segments(id)
            ));
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
                continue; // bundle root index, not a node
            }
            // node_id from the `id:` frontmatter, else derived from the relative path.
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

    async fn list_nodes(&self) -> Result<Value> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, parent_id, label, one_liner, created_at, COALESCE(updated_at, created_at), source \
             FROM nodes ORDER BY id",
        )?;
        let rows = stmt.query_map([], |r| {
            let id: String = r.get(0)?;
            Ok(json!({
                "id": id.clone(),
                "node_id": id,
                "parent_id": r.get::<_, Option<String>>(1)?,
                "label": r.get::<_, String>(2)?,
                "one_liner": r.get::<_, String>(3)?,
                "created_at": r.get::<_, Option<i64>>(4)?,
                "updated_at": r.get::<_, Option<i64>>(5)?,
                "source": r.get::<_, Option<String>>(6)?,
            }))
        })?;
        let nodes: Vec<Value> = rows.filter_map(|r| r.ok()).collect();
        Ok(json!(nodes))
    }

    async fn delete_node(&self, node_id: &str) -> Result<Value> {
        let id = node_id.trim_matches('.').to_string();
        if id.is_empty() {
            return Err(anyhow::anyhow!("empty node_id"));
        }
        let idlen = id.len() as i64;
        let like = subtree_like(&id);
        let conn = self.conn.lock().unwrap();

        if id == "orphans" || id.starts_with("orphans.") {
            // Hard delete for orphans
            let _ = conn.execute("DELETE FROM items_fts WHERE node_id = ?1 OR node_id LIKE ?2 ESCAPE '\\'", rusqlite::params![id, like]);
            conn.execute("DELETE FROM items WHERE node_id = ?1 OR node_id LIKE ?2 ESCAPE '\\'", rusqlite::params![id, like])?;
            conn.execute("DELETE FROM nodes WHERE id = ?1 OR id LIKE ?2 ESCAPE '\\'", rusqlite::params![id, like])?;
            return Ok(json!({"deleted": id, "hard_delete": true}));
        }

        // Always relocate the whole subtree under `orphans.<base_name>_<timestamp>`
        // This avoids data loss and uniqueness conflicts (UNIQUE constraint failed: nodes.id).
        let base_name = id.split('.').next_back().unwrap_or(&id);
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let dest = format!("orphans.{}_{}", base_name, ts);

        ensure_node(&conn, "orphans")?;

        conn.execute(
            "UPDATE items SET node_id = ?1 || substr(node_id, ?2+1) WHERE node_id = ?3 OR node_id LIKE ?4 ESCAPE '\\'",
            rusqlite::params![dest, idlen, id, like],
        )?;
        let _ = conn.execute(
            "UPDATE items_fts SET node_id = ?1 || substr(node_id, ?2+1) WHERE node_id = ?3 OR node_id LIKE ?4 ESCAPE '\\'",
            rusqlite::params![dest, idlen, id, like],
        );
        conn.execute(
            "UPDATE nodes SET parent_id = ?1 || substr(parent_id, ?2+1) WHERE parent_id = ?3 OR parent_id LIKE ?4 ESCAPE '\\'",
            rusqlite::params![dest, idlen, id, like],
        )?;
        conn.execute(
            "UPDATE nodes SET id = ?1 || substr(id, ?2+1) WHERE id = ?3 OR id LIKE ?4 ESCAPE '\\'",
            rusqlite::params![dest, idlen, id, like],
        )?;
        conn.execute(
            "UPDATE nodes SET parent_id='orphans' WHERE id=?1",
            rusqlite::params![dest],
        )?;

        Ok(json!({"deleted": id, "moved_to": "orphans", "relocated_to": dest}))
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
            return Err(anyhow::anyhow!("empty node_id"));
        }
        let conn = self.conn.lock().unwrap();
        // Creates the parent chain as needed.
        if let Some(parent) = node_parent_id(&id) {
            ensure_node(&conn, &parent)?;
        }
        let parent_id = node_parent_id(&id);
        conn.execute(
            "INSERT INTO nodes(id,parent_id,label,one_liner,importance,created_at,updated_at,source)
             VALUES(?1,?2,?3,?4,?5,?6,?6,?7)
             ON CONFLICT(id) DO UPDATE SET label=excluded.label,
               one_liner=excluded.one_liner, importance=excluded.importance, updated_at=excluded.updated_at, source=excluded.source",
            rusqlite::params![
                id,
                parent_id,
                label,
                one_liner.unwrap_or(""),
                importance.unwrap_or(0.5),
                now(),
                source
            ],
        )?;
        conn.execute(
            "INSERT INTO mutations(op,node_id,content,ts) VALUES('create_node',?1,?2,?3)",
            rusqlite::params![id, label, now()],
        )?;
        Ok(json!({"created": id, "label": label, "parent_id": parent_id}))
    }

    async fn update_node(
        &self,
        node_id: &str,
        label: Option<&str>,
        one_liner: Option<&str>,
        importance: Option<f32>,
    ) -> Result<Value> {
        let id = node_id.trim_matches('.').to_string();
        let conn = self.conn.lock().unwrap();
        ensure_node(&conn, &id)?;
        if let Some(label) = label {
            conn.execute(
                "UPDATE nodes SET label=?1 WHERE id=?2",
                rusqlite::params![label, id],
            )?;
        }
        if let Some(one_liner) = one_liner {
            conn.execute(
                "UPDATE nodes SET one_liner=?1 WHERE id=?2",
                rusqlite::params![one_liner, id],
            )?;
        }
        if let Some(importance) = importance {
            conn.execute(
                "UPDATE nodes SET importance=?1 WHERE id=?2",
                rusqlite::params![importance, id],
            )?;
        }
        conn.execute(
            "UPDATE nodes SET updated_at=?1 WHERE id=?2",
            rusqlite::params![now(), id],
        )?;
        conn.execute(
            "INSERT INTO mutations(op,node_id,content,ts) VALUES('update_node',?1,?2,?3)",
            rusqlite::params![id, label.unwrap_or(""), now()],
        )?;
        node_json(&conn, &id)
    }

    async fn renommer_sous_arbre(&self, old_prefix: &str, new_prefix: &str) -> Result<usize> {
        let old = old_prefix.trim_matches('.');
        let new = new_prefix.trim_matches('.');
        if old.is_empty() || new.is_empty() {
            return Ok(0);
        }
        let like = subtree_like(old);
        let oldlen = old.len() as i64;
        let conn = self.conn.lock().unwrap();
        // Items: node_id `old(.rest)` to `new(.rest)`.
        conn.execute(
            "UPDATE items SET node_id = ?1 || substr(node_id, ?2+1) WHERE node_id = ?3 OR node_id LIKE ?4 ESCAPE '\\'",
            rusqlite::params![new, oldlen, old, like],
        )?;
        // FTS index (same rows, node_id column).
        let _ = conn.execute(
            "UPDATE items_fts SET node_id = ?1 || substr(node_id, ?2+1) WHERE node_id = ?3 OR node_id LIKE ?4 ESCAPE '\\'",
            rusqlite::params![new, oldlen, old, like],
        );
        // Nodes: parent_id first (otherwise the target is lost after renaming the ids).
        conn.execute(
            "UPDATE nodes SET parent_id = ?1 || substr(parent_id, ?2+1) WHERE parent_id = ?3 OR parent_id LIKE ?4 ESCAPE '\\'",
            rusqlite::params![new, oldlen, old, like],
        )?;
        let moved = conn.execute(
            "UPDATE nodes SET id = ?1 || substr(id, ?2+1) WHERE id = ?3 OR id LIKE ?4 ESCAPE '\\'",
            rusqlite::params![new, oldlen, old, like],
        )?;
        if moved > 0 {
            conn.execute(
                "INSERT INTO mutations(op,node_id,content,ts) VALUES('rename_subtree',?1,?2,?3)",
                rusqlite::params![old, new, now()],
            )?;
        }
        Ok(moved)
    }

    async fn supprimer_sous_arbre(&self, prefix: &str) -> Result<usize> {
        let p = prefix.trim_matches('.');
        if p.is_empty() {
            return Ok(0);
        }
        let like = subtree_like(p);
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "DELETE FROM items_fts WHERE node_id = ?1 OR node_id LIKE ?2 ESCAPE '\\'",
            rusqlite::params![p, like],
        );
        conn.execute(
            "DELETE FROM items WHERE node_id = ?1 OR node_id LIKE ?2 ESCAPE '\\'",
            rusqlite::params![p, like],
        )?;
        let removed = conn.execute(
            "DELETE FROM nodes WHERE id = ?1 OR id LIKE ?2 ESCAPE '\\'",
            rusqlite::params![p, like],
        )?;
        Ok(removed)
    }

    fn definir_arbitre(&self, arbitre: Arc<dyn crate::Arbitre>) {
        self.poser_arbitre(arbitre);
    }

    async fn backfill_embeddings(&self, max: usize) -> Result<usize> {
        if self.embedder.is_none() {
            return Ok(0);
        }
        // Collect under the lock, embed WITHOUT the lock (network awaits), update after.
        let manquants: Vec<(i64, String)> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT id, content FROM items \
                 WHERE status='active' AND embedding IS NULL LIMIT ?1",
            )?;
            let rows = stmt.query_map(rusqlite::params![max as i64], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })?;
            rows.filter_map(|r| r.ok()).collect()
        };
        let mut faits = 0usize;
        for (id, content) in manquants {
            let Some(v) = self.embed_opt(&content).await else {
                break; // embedder down (breaker open): retry another day
            };
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "UPDATE items SET embedding=?1 WHERE id=?2",
                rusqlite::params![vec_to_blob(&v), id],
            )?;
            faits += 1;
        }
        Ok(faits)
    }

    async fn health(&self) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        Ok(conn
            .query_row("SELECT 1", [], |r| r.get::<_, i64>(0))
            .is_ok())
    }
}

#[cfg(test)]
mod tests_okf_markdown {
    use super::parse_okf;

    /// Un item d'une seule ligne, la forme la plus courante: rien ne change.
    #[test]
    fn un_item_dune_ligne_se_relit_tel_quel() {
        let doc = "---\ntype: memory-node\nid: projects.x\n---\n\n\
                   # Projet X\n\n\
                   - Le build passe en release.  _(source: butinage)_\n\
                   - Le port par defaut est 8419.\n";
        let (id, items) = parse_okf(doc, "fallback");
        assert_eq!(id, "projects.x");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], "Le build passe en release.");
        assert_eq!(items[1], "Le port par defaut est 8419.");
    }

    /// La regression qui comptait: un item en markdown revenait mutile.
    ///
    /// L'export l'aplatissait, l'import lisait une ligne par item, et l'aller-retour
    /// ne "marchait" que grace a cette destruction. Un titre, une liste imbriquee et
    /// une ligne vide doivent survivre au cycle.
    #[test]
    fn un_item_markdown_survit_a_l_aller_retour() {
        let doc = "---\ntype: memory-node\nid: episodes.2026_09_02\n---\n\n\
                   # Episodes\n\n\
                   - Veille du 02/09 : rien de neuf.\n\
                   \x20 \n\
                   \x20 ### Verifications\n\
                   \x20 - GitHub API : 0 resultat\n\
                   \x20 - archive.org : rien\n\
                   \x20 _(source: butinage)_\n\
                   - Un autre fait, sur une ligne.\n";
        let (_, items) = parse_okf(doc, "f");
        assert_eq!(items.len(), 2, "la liste imbriquee ne doit pas devenir des items");
        assert!(items[0].contains("### Verifications"), "le titre survit");
        assert!(items[0].contains("- GitHub API : 0 resultat"), "la puce imbriquee survit");
        assert!(items[0].contains("- archive.org : rien"));
        assert!(!items[0].contains("source:"), "la decoration d'export n'est pas du contenu");
        assert_eq!(items[1], "Un autre fait, sur une ligne.");
    }

    /// Un bundle ECRIT AILLEURS (la raison d'etre d'OKF) reste lisible: seules les
    /// lignes de liste de premier niveau sont des items, le reste est ignore.
    #[test]
    fn un_bundle_etranger_reste_lisible() {
        let doc = "---\ntype: BigQuery Table\ntitle: Orders\n---\n\n\
                   # Schema\n\n\
                   | Column | Type |\n|---|---|\n| id | STRING |\n\n\
                   # Joins\n\n\
                   - Joint avec [customers](/tables/customers.md) sur `customer_id`.\n";
        let (id, items) = parse_okf(doc, "tables.orders");
        assert_eq!(id, "tables.orders", "sans `id:`, le chemin fait foi");
        assert_eq!(items.len(), 1);
        assert!(items[0].contains("[customers](/tables/customers.md)"), "le lien OKF survit");
    }
}
