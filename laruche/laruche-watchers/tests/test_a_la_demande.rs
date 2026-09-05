//! Tester une vigie a la demande est un VRAI passage.
//!
//! Le bouton doit prouver ce qui se passera vraiment, notification comprise. Il
//! consomme donc l'observation, exactement comme le balayage automatique. Ces
//! tests fixent les deux consequences de ce choix, qui ne se devinent pas en
//! lisant l'appel: un evenement n'est annonce qu'une fois, et une vigie dont la
//! condition n'est pas remplie ne declenche rien meme sur demande.

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

/// Un evenement ne doit etre annonce qu'UNE fois.
///
/// C'est la raison d'etre de la consommation: si le test laissait l'etat
/// intact, il annoncerait le changement, puis le balayage suivant l'annoncerait
/// une seconde fois. On recevrait deux notifications pour un seul fait.
#[tokio::test]
async fn un_changement_ne_se_declenche_qu_une_fois() {
    let dir = std::env::temp_dir().join(format!("lr_vigie_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let cible = dir.join("surveille.log");
    std::fs::write(&cible, "avant\n").unwrap();

    let mut reg = WatchersRegistry::new(&dir.join("watchers.json"));
    let id = reg.add(vigie_fichier(&cible));

    // Premier balayage: il pose la reference, sans rien annoncer.
    let _ = reg.check_triggered_watchers().await;

    // Un vrai changement, puis le bouton.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(&cible, "apres\n").unwrap();
    assert!(
        reg.tester(&id).await.is_some(),
        "le changement doit declencher la vigie"
    );

    // Le meme bouton, tout de suite apres: plus rien a annoncer.
    assert!(
        reg.tester(&id).await.is_none(),
        "le changement a ete annonce deux fois"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Une condition non remplie ne declenche rien, meme sur demande.
///
/// Le bouton leve l'intervalle et le delai de garde, qui sont des cadences. Il
/// ne force pas la regle: sinon il repondrait toujours oui et ne prouverait rien.
#[tokio::test]
async fn sans_changement_le_bouton_ne_declenche_rien() {
    let dir = std::env::temp_dir().join(format!("lr_calme_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let cible = dir.join("surveille.log");
    std::fs::write(&cible, "stable\n").unwrap();

    let mut reg = WatchersRegistry::new(&dir.join("watchers.json"));
    let id = reg.add(vigie_fichier(&cible));
    let _ = reg.check_triggered_watchers().await;

    assert!(
        reg.tester(&id).await.is_none(),
        "rien n'a change: la vigie ne doit pas se declencher"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn tester_une_vigie_inconnue_ne_repond_rien() {
    let dir = std::env::temp_dir().join(format!("lr_vide_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut reg = WatchersRegistry::new(&dir.join("watchers.json"));
    assert!(
        reg.tester(&Uuid::new_v4()).await.is_none(),
        "une vigie inconnue ne doit pas produire de declenchement"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
