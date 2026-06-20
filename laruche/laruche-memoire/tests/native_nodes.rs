use laruche_memoire::{MemoireCognitive, MemoryItem, NativeBackend};

#[tokio::test]
async fn native_read_node_derives_children_and_filters_proposals() {
    let backend = NativeBackend::new();
    backend
        .write(MemoryItem::new(
            "projects.laruche",
            "La carte cognitive native expose les items actifs.",
        ))
        .await
        .unwrap();
    backend
        .write(MemoryItem::new(
            "projects.laruche.ui",
            "Un enfant direct existe pour l'interface memoire.",
        ))
        .await
        .unwrap();
    backend
        .propose_write(MemoryItem::new(
            "projects.laruche",
            "Cette proposition reste hors lecture active.",
        ))
        .await
        .unwrap();

    let node = backend.read_node("projects.laruche").await.unwrap();
    assert_eq!(node["id"], "projects.laruche");
    assert!(node["children"]
        .as_array()
        .unwrap()
        .iter()
        .any(|child| child["id"] == "projects.laruche.ui"));
    assert_eq!(node["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn native_dream_reports_duplicates() {
    let backend = NativeBackend::new();
    backend
        .write(MemoryItem::new(
            "decisions.archi",
            "Garder le trait stable.",
        ))
        .await
        .unwrap();
    backend
        .write(MemoryItem::new(
            "decisions.archi",
            "Garder le trait stable.",
        ))
        .await
        .unwrap();

    let dream = backend.dream().await.unwrap();
    assert!(dream["duplicates"].as_u64().unwrap_or(0) >= 1);
    assert!(dream["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| { s["kind"] == "duplicate" && s["node_id"] == "decisions.archi" }));
}
