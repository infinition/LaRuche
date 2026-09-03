//! Tester une vigie a la demande ne doit RIEN changer a ce qu'elle surveille.
//!
//! Un test qui consomme l'observation ferait manquer a la vigie le vrai
//! evenement suivant: on verifierait qu'elle marche en la cassant. Ces deux
//! tests fixent cette garantie, qui n'est pas evidente en lisant l'appel.

use chrono::Utc;
use laruche_watchers::{Action, Watcher, WatcherType, WatchersRegistry};
use uuid::Uuid;

fn vigie_fichier(cible: &std::path::Path) -> Watcher {
    Watcher {
        id: Uuid::new_v4(),
        name: "essai".into(),
        profile_id: None,
        model: None,
        watcher_type: WatcherType::File,
        target: cible.to_string_lossy().to_string(),
        condition: String::new(),
        prompt: String::new(),
        channel: None,
        active: true,
        created_at: Utc::now(),
        last_run: None,
        run_count: 0,
        last_state: None,
        interval_secs: None,
        cooldown_secs: None,
        sustained: false,
        action: Action::default(),
        dernier_verdict: None,
        verdict_depuis: None,
        echecs_consecutifs: 0,
        lignes_vues: None,
        regles: None,
    }
}

/// L'etat observable d'une vigie: tout ce qu'un test ne doit pas bouger.
fn empreinte(reg: &WatchersRegistry, id: &Uuid) -> Option<String> {
    reg.list().iter().find(|w| w.id == *id).map(|w| {
        format!(
            "{:?}|{}|{:?}|{:?}|{}",
            w.last_state, w.run_count, w.last_run, w.lignes_vues, w.echecs_consecutifs
        )
    })
}

#[tokio::test]
async fn tester_une_vigie_ne_touche_pas_a_son_etat() {
    let dir = std::env::temp_dir().join(format!("lr_vigie_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let cible = dir.join("surveille.log");
    std::fs::write(&cible, "premiere ligne\n").unwrap();

    let mut reg = WatchersRegistry::new(&dir.join("watchers.json"));
    let id = reg.add(vigie_fichier(&cible));

    // Un premier balayage pose la reference: sans lui on comparerait deux etats
    // vierges, ce qui ne prouverait rien.
    let _ = reg.check_triggered_watchers().await;
    let avant = empreinte(&reg, &id).expect("la vigie doit exister");

    // Le test, repete. Il observe, il ne consomme pas.
    for _ in 0..3 {
        assert!(
            reg.tester(&id).await.is_some(),
            "tester doit repondre pour une vigie connue"
        );
    }

    assert_eq!(
        avant,
        empreinte(&reg, &id).unwrap(),
        "tester a modifie l'etat de la vigie"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn tester_une_vigie_inconnue_ne_repond_rien() {
    let dir = std::env::temp_dir().join(format!("lr_vide_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let reg = WatchersRegistry::new(&dir.join("watchers.json"));
    assert!(
        reg.tester(&Uuid::new_v4()).await.is_none(),
        "une vigie inconnue ne doit pas produire de verdict"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
