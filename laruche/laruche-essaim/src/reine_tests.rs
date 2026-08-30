//! What LaReine is actually shown. These lock the fix for the loop where she sent
//! well-researched work back to be redone because the workshop view listed tool
//! NAMES and no evidence at all.

#[cfg(test)]
mod tests {
    use crate::reine_live::{construire_atelier_pour_test, construire_contexte_pour_test};
    use crate::session::{Message, Session};
    use crate::AbeilleRegistry;

    fn session_recherche() -> Session {
        let mut s = Session::new("test");
        s.messages
            .push(Message::User("les RPG les plus hype".into()));
        s.messages.push(Message::Assistant("premiere liste".into()));
        s.messages
            .push(Message::User("recentre sur Skyrim, Witcher, Fable".into()));
        s.messages.push(Message::ToolCall {
            name: "delegate".into(),
            args: serde_json::json!({"role":"eclaireuse","task":"Recherche Witcher 4"}),
        });
        s.messages.push(Message::Observation {
            tool: "delegate".into(),
            result: "CD Projekt confirmed Witcher 4 is in full production, no date. \
                     Source https://www.cdprojekt.com/en/investors/ and \
                     https://gamesradar.com/witcher-4-news"
                .into(),
            images: vec![],
        });
        s.messages.push(Message::ToolCall {
            name: "web_deep_search".into(),
            args: serde_json::json!({"query":"Fable reboot 2026 release date confirmed"}),
        });
        s.messages.push(Message::Observation {
            tool: "web_deep_search".into(),
            result: "Fable is slated for 2026 per https://xbox.com/fable and a trailer.".into(),
            images: vec![],
        });
        s
    }

    #[test]
    fn latelier_montre_les_preuves_pas_seulement_les_noms_doutils() {
        let s = session_recherche();
        let atelier = construire_atelier_pour_test(&s, &AbeilleRegistry::new());

        // The counts survive, they were the only useful part of the old view.
        assert!(atelier.contains("2 call(s)"), "{atelier}");

        // WHAT was searched. Without this the judge cannot tell a real search from a
        // claimed one, which is exactly how the redo loop started.
        assert!(
            atelier.contains("Recherche Witcher 4"),
            "scout task missing:\n{atelier}"
        );
        assert!(
            atelier.contains("Fable reboot 2026 release date confirmed"),
            "query missing:\n{atelier}"
        );

        // WHAT came back, including the scout's own synthesis.
        assert!(
            atelier.contains("full production"),
            "scout report missing:\n{atelier}"
        );

        // HOW MANY real sources it rests on, the number the verdict should turn on.
        assert!(
            atelier.contains("Distinct sources actually fetched or returned: 3"),
            "{atelier}"
        );
        assert!(atelier.contains("cdprojekt.com"), "{atelier}");
        assert!(atelier.contains("xbox.com/fable"), "{atelier}");

        // And she is told not to demand a search that is already in front of her.
        assert!(
            atelier.contains("do not ask for a search that already appears here"),
            "{atelier}"
        );
    }

    #[test]
    fn un_brouillon_sans_aucun_outil_est_annonce_comme_non_verifie() {
        let mut s = Session::new("test");
        s.messages.push(Message::User("question".into()));
        let atelier = construire_atelier_pour_test(&s, &AbeilleRegistry::new());
        assert!(atelier.contains("NO tool was called"), "{atelier}");
        assert!(atelier.contains("unverified by construction"), "{atelier}");
    }

    #[test]
    fn un_echec_doutil_est_signale_sans_faire_disparaitre_la_preuve() {
        let mut s = Session::new("test");
        s.messages.push(Message::User("q".into()));
        s.messages.push(Message::ToolCall {
            name: "web_fetch".into(),
            args: serde_json::json!({"url":"https://example.com/blocked"}),
        });
        s.messages.push(Message::Observation {
            tool: "web_fetch".into(),
            result: "Error 403 Forbidden".into(),
            images: vec![],
        });
        let atelier = construire_atelier_pour_test(&s, &AbeilleRegistry::new());
        assert!(atelier.contains("(1 failed)"), "{atelier}");
        assert!(atelier.contains("FAILED"), "{atelier}");
        assert!(atelier.contains("example.com/blocked"), "{atelier}");
    }

    #[test]
    fn le_contexte_garde_la_demande_initiale_quand_la_fenetre_ne_latteint_plus() {
        let mut s = Session::new("test");
        s.messages
            .push(Message::User("compare ces trois moteurs de jeu".into()));
        for i in 0..6 {
            s.messages.push(Message::Assistant(format!("reponse {i}")));
            s.messages.push(Message::User(format!("suite {i}")));
        }
        s.messages
            .push(Message::Assistant("brouillon en cours".into()));

        // With a window of 2, the opening request falls outside it. Losing it makes the
        // judge review the answer against the latest follow-up rather than the goal.
        let ctx = construire_contexte_pour_test(&s, 2);
        assert!(ctx.contains("[opening request]"), "{ctx}");
        assert!(ctx.contains("compare ces trois moteurs de jeu"), "{ctx}");
        assert!(ctx.contains("earlier turn(s) omitted"), "{ctx}");
        // The draft under review and the current request are supplied separately.
        assert!(!ctx.contains("brouillon en cours"), "{ctx}");

        // Short conversation: no banner, nothing is missing to begin with.
        let mut court = Session::new("test");
        court.messages.push(Message::User("salut".into()));
        court.messages.push(Message::Assistant("brouillon".into()));
        let ctx2 = construire_contexte_pour_test(&court, 4);
        assert!(!ctx2.contains("[opening request]"), "{ctx2}");
    }
}

#[cfg(test)]
mod tests_outils {
    use crate::abeille::{Abeille, ContextExecution};
    use crate::abeilles::fichiers::FileEdit;

    /// The trace that started the six-round loop: the model wanted to READ a file and
    /// emitted file_edit instead, eight times in a row. Neither error said which tool
    /// it should have used, so nothing broke the cycle.
    #[tokio::test]
    async fn file_edit_oriente_vers_file_read_et_file_list() {
        let ctx = ContextExecution::default();

        // A directory. The OS answered "Access denied", which reads like a rights
        // problem and sends the model hunting for permissions it already has.
        let sur_dossier = FileEdit
            .executer(
                serde_json::json!({
                    "path": ".",
                    "old_string": "a",
                    "new_string": "b"
                }),
                &ctx,
            )
            .await
            .unwrap();
        let txt = format!("{sur_dossier:?}");
        assert!(txt.contains("DIRECTORY"), "{txt}");
        assert!(txt.contains("file_list"), "{txt}");
        assert!(txt.contains("file_read"), "{txt}");
    }
}

#[cfg(test)]
mod tests_fuite_revue {
    use crate::reine_live::est_consigne_revue;

    #[test]
    fn une_consigne_de_revue_nest_pas_une_mission_utilisateur() {
        // Recalled verbatim in a live prompt as a past mission the agent had run:
        // "Mission: [Your supervisor LaReine reviewed your previous ANSWER and is
        // sending it back. SCOPE: ..."
        assert!(est_consigne_revue(
            "[Your supervisor LaReine reviewed your previous ANSWER and is sending it back.\nSCOPE: ..."
        ));
        assert!(est_consigne_revue(
            "  [Your supervisor LaReine reviewed your previous answer and sends you back"
        ));
        // A real question, even one that mentions her, still gets its episode.
        assert!(!est_consigne_revue("Explique-moi l'architecture du projet"));
        assert!(!est_consigne_revue(
            "pourquoi LaReine reviewed ma reponse ?"
        ));
    }
}
