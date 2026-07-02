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
async fn ecriture_deduplique_a_la_source_et_dream_reste_propre() {
    // NEW behavior: an exact duplicate is refused AT WRITE TIME (dedup no-op),
    // so dream() has nothing to report - the map stays clean by construction.
    // (dream's duplicate report remains for LEGACY databases.)
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
    let deux = backend
        .write(MemoryItem::new(
            "decisions.archi",
            "Garder un mono-binaire Rust.",
        ))
        .await
        .unwrap();
    assert_eq!(deux["dedup"], true, "exact duplicate must be a no-op: {deux}");

    // Only ONE active item remains, and dream reports zero duplicates.
    let node = backend.read_node("decisions.archi").await.unwrap();
    assert_eq!(node["items"].as_array().unwrap().len(), 1);
    let dream = backend.dream().await.unwrap();
    assert_eq!(dream["duplicates"].as_i64().unwrap_or(-1), 0);

    cleanup_db(&dir);
}

#[tokio::test]
async fn recall_exclut_les_projections_skills_systeme() {
    // A GPU-ish question must NOT surface skill catalog bodies (capacities.*) nor
    // system.* projections, even if their tokens overlap. Regression: a recall used
    // to dump full skill guides into a hardware question.
    let dir = temp_db("skills_noise");
    cleanup_db(&dir);
    let backend = SqliteBackend::open_with_embedder(&dir, Arc::new(FakeEmbedder)).unwrap();
    backend
        .write(MemoryItem::new("capacities.skills.local_discovery", "Procedure: code jazz tongs guide"))
        .await
        .unwrap();
    backend
        .write(MemoryItem::new("people.fabien", "Il code en tongs en ecoutant du jazz"))
        .await
        .unwrap();
    let pack = backend.search("comment je code habituellement", SearchOpts::default()).await.unwrap();
    let text = pack.to_prompt_text();
    assert!(!text.contains("Procedure"), "skills projection must be excluded: {text}");
    cleanup_db(&dir);
}

#[tokio::test]
async fn supersede_traverse_les_noeuds_du_domaine() {
    // The same fact filed under SIBLING nodes of one domain (hardware.a, hardware.b)
    // must not both stay active: writing the second supersedes the first. Regression:
    // 4070 Ti in hardware.local_model_setup + 5080 in hardware.gpu both stayed active.
    let dir = temp_db("supersede_domain");
    cleanup_db(&dir);
    let backend = SqliteBackend::open_with_embedder(&dir, Arc::new(FakeEmbedder)).unwrap();
    backend
        .write(MemoryItem::new("hardware.local_model_setup", "code jazz tongs setup"))
        .await
        .unwrap();
    // Same embedding (shared keywords -> identical FakeEmbedder vector), sibling node.
    backend
        .write(MemoryItem::new("hardware.gpu", "code jazz tongs setup v2"))
        .await
        .unwrap();
    let a = backend.read_node("hardware.local_model_setup").await.unwrap();
    let b = backend.read_node("hardware.gpu").await.unwrap();
    let active_a = a["items"].as_array().map(|x| x.len()).unwrap_or(0);
    let active_b = b["items"].as_array().map(|x| x.len()).unwrap_or(0);
    assert_eq!(active_a + active_b, 1, "cross-node supersede: only the newest fact stays active");
    cleanup_db(&dir);
}
