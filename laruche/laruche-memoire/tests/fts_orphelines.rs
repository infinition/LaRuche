use laruche_memoire::{MemoireCognitive, MemoryItem, SqliteBackend};

/// Reproduces the state found in a live base: 372 FTS rows for 160 items, because hard
/// deletes left FTS rows behind. Since `items.id` is INTEGER PRIMARY KEY *without*
/// autoincrement, SQLite reuses freed rowids, so every orphan poisons one future write
/// with a bare "constraint failed" — which is why it failed one time in two.
#[tokio::test]
async fn un_rowid_reutilise_avec_une_ligne_fts_orpheline_nempeche_plus_lecriture() {
    let dir = std::env::temp_dir().join(format!("lr_fts_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("m.db");

    // Plant the exact hazard: an FTS row on a rowid that no item occupies.
    {
        let c = rusqlite::Connection::open(&db).unwrap();
        c.execute_batch(
            "CREATE TABLE items(id INTEGER PRIMARY KEY, node_id TEXT NOT NULL,
               content TEXT NOT NULL, source TEXT, status TEXT NOT NULL DEFAULT 'active',
               embedding BLOB, created_at INTEGER NOT NULL);
             CREATE TABLE nodes(id TEXT PRIMARY KEY, parent_id TEXT, label TEXT NOT NULL,
               one_liner TEXT NOT NULL DEFAULT '', importance REAL NOT NULL DEFAULT 0.5,
               source TEXT, created_at INTEGER NOT NULL);
             CREATE VIRTUAL TABLE items_fts USING fts5(content, node_id);
             CREATE TABLE mutations(id INTEGER PRIMARY KEY, op TEXT NOT NULL,
               node_id TEXT, content TEXT, ts INTEGER NOT NULL);",
        )
        .unwrap();
        // rowid 1 is free in `items` but taken in the index.
        c.execute("INSERT INTO items_fts(rowid,content,node_id) VALUES(1,'fantome','episodes')", [])
            .unwrap();
    }

    let mem = SqliteBackend::open(&db).unwrap();
    let ecrire = |contenu: &str| {
        mem.write(MemoryItem::new("episodes", contenu))
    };

    // The very first write lands on the poisoned rowid 1.
    ecrire("ceci est un test").await.expect("premiere ecriture refusee");
    ecrire("deuxieme").await.expect("deuxieme ecriture refusee");
    ecrire("troisieme").await.expect("troisieme ecriture refusee");

    // And the index is consistent again: no row without an item behind it.
    let c = rusqlite::Connection::open(&db).unwrap();
    let orphelines: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM items_fts f WHERE NOT EXISTS(SELECT 1 FROM items i WHERE i.id=f.rowid)",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(orphelines, 0, "des lignes FTS orphelines subsistent");
    let _ = std::fs::remove_dir_all(&dir);
}
