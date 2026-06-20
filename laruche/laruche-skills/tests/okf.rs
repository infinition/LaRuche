use laruche_memoire::{MemoireCognitive, SqliteBackend};
use laruche_skills::{list_skills, read_skill, write_skill, Paradigm, Skill, Step};

fn sample_skill() -> Skill {
    let mut skill = Skill::new(
        "Edition Rust sure",
        "Modifier un fichier Rust sans casser le contexte",
    );
    skill.meta.allowed_tools = vec!["file_read".to_string(), "file_edit".to_string()];
    skill.meta.when_to_use = "Quand une modification ciblee de code est demandee".to_string();
    skill.paradigms.push(Paradigm {
        id: "p_1".to_string(),
        title: "Lire avant d'ecrire".to_string(),
        description: "Verifier le voisinage du code avant tout patch.".to_string(),
        rules: vec!["Preferer un remplacement minimal.".to_string()],
    });
    skill.steps.push(Step {
        id: "step_1".to_string(),
        name: "Patch cible".to_string(),
        instruction: "Appliquer le remplacement le plus petit possible.".to_string(),
        success_criteria: vec!["Le test de la crate passe.".to_string()],
        execution: Some("Direct".to_string()),
        artifacts: vec!["diff".to_string()],
        human_checkpoint: false,
    });
    skill
}

#[test]
fn parse_serialize_roundtrip_okf_skill() {
    let skill = sample_skill();
    let markdown = skill.to_markdown();
    assert!(markdown.starts_with("---\ntype: skill\n"));

    let parsed = Skill::parse(&markdown).unwrap();
    assert_eq!(parsed.meta.name, skill.meta.name);
    assert_eq!(parsed.meta.allowed_tools, skill.meta.allowed_tools);
    assert_eq!(parsed.paradigms[0].title, "Lire avant d'ecrire");
    assert_eq!(
        parsed.steps[0].success_criteria[0],
        "Le test de la crate passe."
    );
}

#[tokio::test]
async fn skill_created_then_read_from_cognitive_memory() {
    let db = std::env::temp_dir().join(format!(
        "laruche_skills_{}.db",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mem = SqliteBackend::open(&db).unwrap();
    let skill = sample_skill();

    write_skill(&mem, &skill).await.unwrap();
    let loaded = read_skill(&mem, "Edition Rust sure")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.meta.name, skill.meta.name);
    assert_eq!(loaded.steps.len(), 1);

    let listed = list_skills(&mem, Some(10)).await.unwrap();
    assert!(serde_json::to_string(&listed)
        .unwrap()
        .contains("tools.skills.edition_rust_sure"));

    assert!(mem
        .read_node("tools.skills.edition_rust_sure")
        .await
        .unwrap()["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["content"].as_str().unwrap().contains("type: skill")));

    let _ = std::fs::remove_file(db);
}
