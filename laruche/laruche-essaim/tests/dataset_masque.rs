/// A dataset file exists to be copied around and fed to a trainer. A key that reaches it
/// leaks everywhere, permanently. This pins the masking that stands between the two.
#[test]
fn le_masquage_couvre_les_champs_du_dataset() {
    let mut m = std::collections::HashMap::new();
    m.insert("OPENAI_KEY".to_string(), "sk-proj-abcdef1234567890".to_string());
    laruche_essaim::secrets::init(m);

    // What a draft looks like after an agent runs `env` or a verbose curl.
    let brouillon = "I ran the call with sk-proj-abcdef1234567890 and it returned 200.";
    let masque = laruche_essaim::secrets::masquer(brouillon);
    assert!(!masque.contains("sk-proj-abcdef1234567890"), "la cle a fuite: {masque}");
    assert!(masque.contains("[SECRET:OPENAI_KEY]"), "pas remplace: {masque}");

    // Ordinary text must survive untouched, otherwise the dataset is corrupted.
    let normal = "Explain the architecture of the cycle module.";
    assert_eq!(laruche_essaim::secrets::masquer(normal), normal);

    // A vault reference in a provider profile must resolve, and an unknown one must stay
    // visible rather than silently becoming an empty key.
    assert_eq!(laruche_essaim::secrets::substituer("${OPENAI_KEY}"), "sk-proj-abcdef1234567890");
    assert_eq!(laruche_essaim::secrets::substituer("${ABSENT}"), "${ABSENT}");
}
