use async_trait::async_trait;
use laruche_butinage::*;
use serde_json::json;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::Duration;

#[cfg(test)]
mod provider_witnesses;

struct TextModel(&'static str);
#[async_trait]
impl Fournisseur for TextModel {
    async fn repondre(&self, _: &[Message], _: &[serde_json::Value]) -> Result<ReponseModele, ErreurFournisseur> {
        Ok(ReponseModele { texte: self.0.into(), ..Default::default() })
    }
}
struct NoTools;
#[async_trait]
impl Outils for NoTools {
    async fn executer(&self, _: &Appel) -> ResultatOutil { panic!("unexpected call") }
}
fn notebook() -> Carnet { Carnet::ouvrir("CURRENT_MISSION_DO_NOT_LOSE", ModeMission::Standard, chrono::Utc::now()) }

#[tokio::test]
async fn witness_open_plan_becomes_success_and_skipped() {
    let mut c = notebook();
    c.itineraire.definir(vec!["Create deliverable".into(), "Verify deliverable".into()]);
    let b = butiner(&mut c, &Reglages::default(), &TextModel("I will now create it."), &NoTools, &Silencieux, None, None, None).await.unwrap();
    assert_eq!(b.fin, FinDeVol::Accomplie);
    assert!(c.itineraire.etapes.iter().all(|e| e.statut == StatutEtape::NonApplicable));
}

#[tokio::test]
async fn witness_exhausted_empty_replies_are_success() {
    let mut c = notebook();
    let b = butiner(&mut c, &Reglages::default(), &TextModel(""), &NoTools, &Silencieux, None, None, None).await.unwrap();
    assert_eq!(b.fin, FinDeVol::Accomplie);
    assert!(b.texte.is_empty());
}

struct HungModel;
#[async_trait]
impl Fournisseur for HungModel {
    async fn repondre(&self, _: &[Message], _: &[serde_json::Value]) -> Result<ReponseModele, ErreurFournisseur> {
        std::future::pending().await
    }
}
#[tokio::test(start_paused = true)]
async fn witness_compaction_bypasses_model_timeout_and_cancel() {
    let mut c = notebook();
    c.historique.push(Message::utilisateur(c.mission.clone()));
    for _ in 0..20 { c.historique.push(Message::assistant("x".repeat(1000))); }
    let r = Reglages { context_max_tokens: 1000, timeout_modele_secs: 1, max_transitoire: 0, ..Default::default() };
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let other = cancel.clone();
    tokio::spawn(async move { tokio::time::sleep(Duration::from_secs(2)).await; other.store(true, Ordering::SeqCst); });
    let result = tokio::time::timeout(Duration::from_secs(5), butiner(&mut c, &r, &HungModel, &NoTools, &Silencieux, None, None, Some(cancel.as_ref()))).await;
    assert!(cancel.load(Ordering::SeqCst));
    assert!(result.is_err(), "compaction still awaiting provider after cancellation and model deadline");
}

#[test]
fn witness_compaction_keeps_old_question_and_loses_current_mission() {
    let mut c = notebook();
    c.historique.push(Message::utilisateur("OLD_UNRELATED_QUESTION"));
    c.historique.push(Message::assistant("Old answer"));
    c.historique.push(Message::utilisateur(c.mission.clone()));
    for _ in 0..20 { c.historique.push(Message::assistant("intermediate reasoning")); }
    laruche_butinage::escale::compacter(&mut c.historique, 4).unwrap();
    assert!(c.historique.iter().any(|m| m.contenu.contains("OLD_UNRELATED_QUESTION")));
    assert!(!c.historique.iter().any(|m| m.contenu.contains(&c.mission)));
}

struct TwoCalls;
#[async_trait]
impl Fournisseur for TwoCalls {
    async fn repondre(&self, _: &[Message], _: &[serde_json::Value]) -> Result<ReponseModele, ErreurFournisseur> {
        Ok(ReponseModele { appels: vec![Appel::nouveau("mutate", json!({})), Appel::nouveau("hang", json!({}))], stop: StopReason::Outils, ..Default::default() })
    }
}
struct Effects(AtomicUsize);
#[async_trait]
impl Outils for Effects {
    async fn executer(&self, a: &Appel) -> ResultatOutil {
        if a.nom == "mutate" { self.0.fetch_add(1, Ordering::SeqCst); ResultatOutil::ok("applied") }
        else { std::future::pending().await }
    }
}
#[tokio::test(start_paused = true)]
async fn witness_abort_then_resume_repeats_completed_side_effect() {
    let mut c = notebook();
    let path = std::env::temp_dir().join(format!("audit-checkpoint-{}.json", c.id));
    c.sauver(&path, chrono::Utc::now()).unwrap();
    let r = Reglages { chemin_carnet: Some(path.clone()), ..Default::default() };
    let effects = Effects(AtomicUsize::new(0));
    assert!(tokio::time::timeout(Duration::from_secs(1), butiner(&mut c, &r, &TwoCalls, &effects, &Silencieux, None, None, None)).await.is_err());
    assert_eq!(effects.0.load(Ordering::SeqCst), 1);
    let mut resumed = Carnet::charger(&path).unwrap();
    assert_eq!(resumed.passe, 0);
    assert!(tokio::time::timeout(Duration::from_secs(1), butiner(&mut resumed, &r, &TwoCalls, &effects, &Silencieux, None, None, None)).await.is_err());
    assert_eq!(effects.0.load(Ordering::SeqCst), 2);
    std::fs::remove_file(path).unwrap();
}
