//! Proves SEMANTIC recall of the SqliteBackend, offline, via an injected embedder.
//!
//! The stored fact ("tongs ... jazz") and the query ("programmer ... conditions") share
//! NO common word. Only the semantic path (cosine over embeddings) can relate them,
//! so if the test passes the hybrid architecture is correctly wired.

use laruche_memoire::{Embedder, MemoireCognitive, MemoryItem, SearchOpts, SqliteBackend};
use std::path::PathBuf;
use std::sync::Arc;

/// Deterministic fake embedder: projects onto a "code activity" vs "other" axis.
/// Any text evoking code/programming -> [1,0]; otherwise -> [0,1].
struct FakeEmbedder;

#[async_trait::async_trait]
impl Embedder for FakeEmbedder {
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let t = text.to_lowercase();
        let code = t.contains("cod")
            || t.contains("programm")
            || t.contains("jazz")
            || t.contains("tongs");
        Ok(if code { vec![1.0, 0.0] } else { vec![0.0, 1.0] })
    }
}

fn temp_db(name: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "laruche_{name}_{}_{}.db",
        std::process::id(),
        stamp
    ))
}

fn cleanup_db(path: &PathBuf) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path.with_extension("db-wal"));
    let _ = std::fs::remove_file(path.with_extension("db-shm"));
}

#[tokio::test]
async fn recall_semantique_sans_mot_commun() {
    let dir = std::env::temp_dir().join(format!("laruche_sem_{}.db", std::process::id()));
    let _ = std::fs::remove_file(&dir);

    let backend = SqliteBackend::open_with_embedder(&dir, Arc::new(FakeEmbedder)).unwrap();

    backend
        .write(MemoryItem::new(
            "people.fabien",
            "Il code en tongs en écoutant du jazz.",
        ))
        .await
        .unwrap();
    // A "distractor" item on another topic (must NOT come out on top).
    backend
        .write(MemoryItem::new(
            "projects.jardin",
            "Arroser les tomates le matin.",
        ))
        .await
        .unwrap();

    let pack = backend
        .search(
            "dans quelles conditions je programme habituellement",
            SearchOpts::default(),
        )
        .await
        .unwrap();
    let text = pack.to_prompt_text();

    assert!(text.contains("tongs"), "semantic recall failed: {text}");
    assert!(
        !text.contains("tomates"),
        "the distractor should not surface: {text}"
    );

    let _ = std::fs::remove_file(&dir);
}

#[tokio::test]
async fn sqlite_read_node_expose_children_and_items() {
    let dir = temp_db("nodes");
    cleanup_db(&dir);

    let backend = SqliteBackend::open(&dir).unwrap();
    backend
        .write(MemoryItem::new(
            "projects.laruche",
            "La memoire T2 expose les noeuds cognitifs.",
        ))
        .await
        .unwrap();
    backend
        .write(MemoryItem::new(
            "projects.laruche.ui",
            "La page memoire affiche les enfants directs.",
        ))
        .await
        .unwrap();

    let node = backend.read_node("projects.laruche").await.unwrap();
    assert_eq!(node["id"], "projects.laruche");
    assert_eq!(node["parent_id"], "projects");
    assert!(node["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|it| it["content"].as_str().unwrap_or("").contains("T2")));
    assert!(node["children"]
        .as_array()
        .unwrap()
        .iter()
        .any(|child| child["id"] == "projects.laruche.ui"));

    let root = backend.read_node("projects").await.unwrap();
    assert!(root["children"]
        .as_array()
        .unwrap()
        .iter()
        .any(|child| child["id"] == "projects.laruche"));

    cleanup_db(&dir);
}

#[tokio::test]
async fn sqlite_export_okf_writes_bundle() {
    let dir = temp_db("okf");
    cleanup_db(&dir);
    let backend = SqliteBackend::open(&dir).unwrap();
    backend
        .write(MemoryItem::new(
            "projects.laruche",
            "Cible = mono-binaire Rust.",
        ))
        .await
        .unwrap();

    let out = std::env::temp_dir().join(format!("okf_bundle_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let n = backend.export_okf(&out, None).await.unwrap();
    assert!(n >= 2, "must write at least the index + one node");

    let root = std::fs::read_to_string(out.join("index.md")).unwrap();
    assert!(root.contains("type: index"));

    // node projects.laruche -> projects/laruche/index.md directory, OKF frontmatter.
    let node = std::fs::read_to_string(out.join("projects/laruche/index.md")).unwrap();
    assert!(
        node.contains("type: memory-node"),
        "OKF frontmatter missing: {node}"
    );
    assert!(node.contains("mono-binaire Rust"));

    let _ = std::fs::remove_dir_all(&out);
    cleanup_db(&dir);
}

#[tokio::test]
async fn activation_cognitive_priorise_le_noeud_pertinent() {
    let dir = temp_db("activation");
    cleanup_db(&dir);
    let backend = SqliteBackend::open_with_embedder(&dir, Arc::new(FakeEmbedder)).unwrap();

    // Two items with equivalent embedding AND lexicon, under different nodes.
    backend
        .write(MemoryItem::new("projects.laruche", "il code"))
        .await
        .unwrap();
    backend
        .write(MemoryItem::new("misc.autre", "il code"))
        .await
        .unwrap();

    // The query mentions "laruche", so only the projects.laruche node activates and its item comes up first.
    let pack = backend
        .search("laruche code", SearchOpts::default())
        .await
        .unwrap();
    let first_node = pack.raw["items"][0]["node_id"].as_str().unwrap_or("");
    assert_eq!(
        first_node, "projects.laruche",
        "cognitive activation failed: {}",
        pack.raw
    );

    cleanup_db(&dir);
}

#[tokio::test]
async fn sqlite_okf_round_trip() {
    let dir_a = temp_db("okf_a");
    cleanup_db(&dir_a);
    let a = SqliteBackend::open(&dir_a).unwrap();
    a.write(MemoryItem::new(
        "projects.laruche",
        "Cible mono-binaire Rust unique.",
    ))
    .await
    .unwrap();

    let out = std::env::temp_dir().join(format!("okf_rt_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    a.export_okf(&out, None).await.unwrap();

    // Reimport into a FRESH backend, the knowledge must come back.
    let dir_b = temp_db("okf_b");
    cleanup_db(&dir_b);
    let b = SqliteBackend::open(&dir_b).unwrap();
    let n = b.import_okf(&out).await.unwrap();
    assert!(n >= 1, "must import at least 1 item");

    let node = b.read_node("projects.laruche").await.unwrap();
    assert!(
        node["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|it| it["content"]
                .as_str()
                .unwrap_or("")
                .contains("mono-binaire")),
        "OKF round-trip failed: {node}"
    );

    let _ = std::fs::remove_dir_all(&out);
    cleanup_db(&dir_a);
    cleanup_db(&dir_b);
}

#[tokio::test]
async fn sqlite_dream_reports_duplicate_suggestions() {
    let dir = temp_db("dream");
    cleanup_db(&dir);

    let backend = SqliteBackend::open(&dir).unwrap();
    backend
        .write(MemoryItem::new(
            "decisions.archi",
            "Garder un mono-binaire Rust.",
        ))
        .await
        .unwrap();
    backend
        .write(MemoryItem::new(
            "decisions.archi",
            "Garder un mono-binaire Rust.",
        ))
        .await
        .unwrap();

    let dream = backend.dream().await.unwrap();
    assert!(dream["duplicates"].as_i64().unwrap_or(0) >= 1);
    assert!(dream["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| { s["kind"] == "duplicate" && s["node_id"] == "decisions.archi" }));

    cleanup_db(&dir);
}
