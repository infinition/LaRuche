//! Hebbian level 2: a `sans_trace` search must NOT add weight (freshness only),
//! and `renforcer()` must add weight to exactly the given items. Together they
//! prove that mere recall no longer feeds the ranking, only actual use does.

use laruche_memoire::{MemoireCognitive, MemoryItem, SearchOpts, SqliteBackend};
use std::path::PathBuf;

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

/// Reads the hebbian weight straight from the database (the backend does not
/// expose it), via a second SQLite connection on the same file.
fn poids(chemin: &PathBuf, contenu: &str) -> (i64, Option<i64>) {
    let conn = rusqlite::Connection::open(chemin).unwrap();
    conn.query_row(
        "SELECT COALESCE(access_count,0), accessed_at FROM items WHERE content=?1",
        [contenu],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .unwrap()
}

#[tokio::test]
async fn recall_sans_trace_puis_renforcer_cible() {
    let chemin = temp_db("hebbien2");
    let mem = SqliteBackend::open(&chemin).unwrap();
    let contenu = "le mot ultraspecifique pour hebbien deux";

    let r = mem
        .write(MemoryItem::new("projets.test", contenu))
        .await
        .unwrap();
    let item_id = r["item_id"].as_str().unwrap().to_string();
    assert_eq!(poids(&chemin, contenu).0, 0, "fresh item starts unweighted");

    // 1) sans_trace search: recalled, freshness updated, but ZERO weight added.
    let pack = mem
        .search(
            "ultraspecifique hebbien",
            SearchOpts {
                depth: None,
                limit: Some(5),
                sans_trace: true,
            },
        )
        .await
        .unwrap();
    assert!(
        pack.raw["items"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "the item must be recalled"
    );
    let (compte, fraicheur) = poids(&chemin, contenu);
    assert_eq!(compte, 0, "mere recall must not add hebbian weight");
    assert!(fraicheur.is_some(), "freshness trace still updated");

    // 2) Legacy trace (default): a normal search DOES add weight.
    let _ = mem
        .search(
            "ultraspecifique hebbien",
            SearchOpts {
                depth: None,
                limit: Some(5),
                sans_trace: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(poids(&chemin, contenu).0, 1, "traced search keeps level 1");

    // 3) renforcer: weight lands on exactly this item; junk ids are ignored.
    assert_eq!(mem.renforcer(&[item_id]).await.unwrap(), 1);
    assert_eq!(poids(&chemin, contenu).0, 2);
    assert_eq!(
        mem.renforcer(&["itm_999999".to_string(), "garbage".to_string()])
            .await
            .unwrap(),
        0
    );

    cleanup_db(&chemin);
}
