use laruche_essaim::{Message, Session};

/// The agent's reaction used to exist only as a transient event: correct on screen,
/// gone on the next reload. It has to survive a save/load round trip.
#[test]
fn la_reaction_agent_survit_a_un_aller_retour_disque() {
    let dir = std::env::temp_dir().join(format!("lr_react_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();

    let mut s = Session::new_with_path("m", &dir);
    s.messages.push(Message::User("ceci est un test".into()));
    s.messages.push(Message::Assistant("reponse".into()));
    assert!(
        s.definir_reaction_agent("joie"),
        "aucun message utilisateur trouve"
    );
    // Anchored on the USER message (index 0), not on the answer.
    assert_eq!(s.reactions_agent.get(&0).map(String::as_str), Some("joie"));
    s.sauvegarder().unwrap();

    let chemin = dir.join(format!("{}.json", s.id));
    let charge = Session::charger(&chemin).unwrap();
    assert_eq!(
        charge.reactions_agent.get(&0).map(String::as_str),
        Some("joie"),
        "la reaction agent n'a pas survecu au rechargement"
    );
    // The user's own reactions stay a separate map, untouched.
    assert!(charge.reactions.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
