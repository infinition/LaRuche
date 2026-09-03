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

/// Le message assistant ne doit PAS garder son marqueur `/haha` une fois stocke.
/// Avant, le texte diffuse etait nettoye mais la copie poussee dans la session
/// gardait la commande brute, relue telle quelle au rechargement.
#[test]
fn le_message_assistant_stocke_perd_son_marqueur() {
    let dir = std::env::temp_dir().join(format!("lr_clean_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();

    let mut s = Session::new_with_path("m", &dir);
    s.messages.push(Message::User("un test".into()));
    s.messages.push(Message::Assistant("/haha\n\nCoucou, ca marche.".into()));

    // Ce que fait le pont: on nettoie la copie stockee avec le texte depouille.
    let (propre, cle) = laruche_essaim::reactions::extraire_reaction("/haha\n\nCoucou, ca marche.");
    assert_eq!(cle.as_deref(), Some("haha"));
    assert!(s.nettoyer_dernier_assistant(&propre));

    s.sauvegarder().unwrap();
    let chemin = dir.join(format!("{}.json", s.id));
    let charge = Session::charger(&chemin).unwrap();
    let dernier = charge
        .messages
        .iter()
        .rev()
        .find_map(|m| match m {
            Message::Assistant(t) => Some(t.clone()),
            _ => None,
        })
        .unwrap();
    assert!(!dernier.contains("/haha"), "le marqueur est reste: {dernier:?}");
    assert_eq!(dernier, "Coucou, ca marche.");
    let _ = std::fs::remove_dir_all(&dir);
}
