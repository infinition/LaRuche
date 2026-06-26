//! Le **cycle** — la boucle de butinage. Volontairement *bête* : elle orchestre,
//! la politique vit ailleurs ([`crate::cap`]), la classification d'erreurs aussi
//! ([`crate::meteo`]). Aucune heuristique de chaînes ici.
//!
//! Flux d'une passe : assembler le contexte → appeler le modèle (avec météo) →
//! `analyser` la réponse en [`Issue`] → `cap` décide → agir (poser / relancer /
//! récolter) → checkpoint.

use crate::cap::boussole::{cap, Decision};
use crate::cap::vigie::Vigie;
use crate::carnet::Carnet;
use crate::evenement::{Emetteur, Evenement};
use crate::fournisseur::{Fournisseur, ReponseModele};
use crate::issue::{Bilan, FinDeVol, Issue, StopReason, TexteSeul};
use crate::messagerie::Message;
use crate::meteo::{reagir, ClasseErreur, Reaction};
use crate::nectar::Source;
use crate::outils::Outils;
use crate::reglages::Reglages;
use std::time::Duration;

/// Lance un butinage jusqu'à son terme (mission accomplie, clarification, plafond,
/// erreur fatale ou boucle stérile). Le [`Carnet`] est muté à chaque passe et
/// peut être persisté (reprise après crash).
pub async fn butiner(
    carnet: &mut Carnet,
    reglages: &Reglages,
    fournisseur: &dyn Fournisseur,
    outils: &dyn Outils,
    emet: &dyn Emetteur,
    source: Option<&dyn Source>,
) -> anyhow::Result<Bilan> {
    if carnet.historique.is_empty() {
        let mission = carnet.mission.clone();
        carnet.historique.push(Message::utilisateur(mission));
    }
    let mut vigie = Vigie::nouvelle(reglages.profil.seuils_vigie());
    let mut jauge = crate::cap::jauge::Jauge::nouvelle(reglages.context_max_tokens, 0.70, 0.85);
    let schemas = outils.schemas();

    loop {
        if carnet.passe >= reglages.plafond_passes {
            let msg = format!("Plafond de {} passes atteint — peut être incomplet.", reglages.plafond_passes);
            emet.emettre(Evenement::Statut(msg.clone()));
            return Ok(Bilan::nouveau(msg, FinDeVol::Plafond, carnet.passe));
        }

        // Escale : compaction (extractive) ou, si la jauge est critique et qu'une mémoire
        // est branchée, consolidation cognitive (LLM → faits durables → contexte frais).
        jauge.estimer(&reglages.systeme, &carnet.historique);
        let consolide =
            matches!(jauge.besoin(), crate::cap::jauge::Besoin::Consolider) && source.is_some();
        if consolide {
            if let Some(ev) =
                crate::escale::consolider(carnet, fournisseur, source.unwrap(), emet).await
            {
                emet.emettre(ev);
                jauge.estimer(&reglages.systeme, &carnet.historique);
            }
        } else if let Some(ev) = crate::escale::peut_etre(carnet, &jauge, reglages.garder_recents) {
            emet.emettre(ev);
            jauge.estimer(&reglages.systeme, &carnet.historique);
        }

        let messages = assembler(carnet, reglages);
        let reponse = match appeler_modele(fournisseur, &messages, &schemas, reglages, emet).await {
            Ok(r) => r,
            Err(motif) => {
                return Ok(Bilan::nouveau(
                    format!("Erreur fatale du provider : {motif}"),
                    FinDeVol::Erreur(motif),
                    carnet.passe,
                ));
            }
        };

        if !reponse.texte.is_empty() {
            emet.emettre(Evenement::Texte(reponse.texte.clone()));
        }
        carnet.historique.push(Message::assistant(reponse.texte.clone()));

        let issue = analyser(&reponse, carnet);
        // Texte final candidat (avant que `cap` ne consomme l'issue).
        let texte_final = match &issue {
            Issue::MissionAccomplie { resume, .. } => resume.clone(),
            Issue::TexteSeul(t) => t.texte.clone(),
            _ => reponse.texte.clone(),
        };

        let ctx = carnet.contexte_cap(reglages.auto_continue_max, reglages.min_web_exploration);
        match cap(&ctx, issue) {
            Decision::Poser(fin) => {
                carnet.itineraire.finaliser();
                emet.emettre(Evenement::Fin(texte_final.clone()));
                return Ok(Bilan::nouveau(texte_final, fin, carnet.passe + 1));
            }
            Decision::Clarifier(q) => {
                emet.emettre(Evenement::Fin(q.clone()));
                return Ok(Bilan::nouveau(q.clone(), FinDeVol::Clarification(q), carnet.passe + 1));
            }
            Decision::Relancer(nudge) => {
                carnet.consommer_auto();
                emet.emettre(Evenement::Statut(format!(
                    "Auto-continuation ({}/{})",
                    carnet.auto_continue, reglages.auto_continue_max
                )));
                carnet.historique.push(Message::utilisateur(nudge));
            }
            Decision::Recolter(appels) => {
                carnet.rearmer_auto();
                if let Some(bilan) =
                    crate::recolte::recolter(&appels, carnet, reglages, outils, &mut vigie, emet).await
                {
                    return Ok(bilan); // arrêt propre : boucle stérile
                }
            }
        }

        carnet.passe += 1;
        if let Some(chemin) = &reglages.chemin_carnet {
            if let Err(e) = carnet.sauver(chemin, chrono::Utc::now()) {
                tracing::warn!(error = %e, "échec du checkpoint carnet");
            }
        }
    }
}

/// Assemble les messages envoyés au modèle : système (tier stable) + historique.
fn assembler(carnet: &Carnet, reglages: &Reglages) -> Vec<Message> {
    let mut v = Vec::with_capacity(carnet.historique.len() + 1);
    if !reglages.systeme.is_empty() {
        v.push(Message::systeme(reglages.systeme.clone()));
    }
    v.extend(carnet.historique.iter().cloned());
    v
}

/// Appel modèle avec politique météo (backoff/abandon). La rotation de clé et le
/// déroutement modèle sont gérés en interne par l'adaptateur `Fournisseur` ; le cœur
/// applique l'attente et l'abandon. Renvoie `Err(motif)` sur arrêt définitif.
async fn appeler_modele(
    fournisseur: &dyn Fournisseur,
    messages: &[Message],
    schemas: &[serde_json::Value],
    reglages: &Reglages,
    emet: &dyn Emetteur,
) -> Result<ReponseModele, String> {
    let mut tentative = 0usize;
    loop {
        match fournisseur.repondre(messages, schemas).await {
            Ok(r) => return Ok(r),
            Err(e) => {
                tentative += 1;
                let classe = ClasseErreur::classer(e.status, e.retry_after.as_deref(), &e.corps);
                let now = chrono::Utc::now().timestamp();
                match reagir(
                    &classe,
                    tentative,
                    reglages.max_rate_limit,
                    reglages.max_transitoire,
                    false, // rotation de clé : déjà tentée par l'adaptateur
                    false, // déroutement : déjà tenté par l'adaptateur
                    now,
                ) {
                    Reaction::Patienter(s) => {
                        emet.emettre(Evenement::Statut(format!(
                            "Erreur provider ({}) — reprise dans {s}s (essai {tentative}).",
                            e.status
                        )));
                        tokio::time::sleep(Duration::from_secs(s)).await;
                    }
                    Reaction::RotationCle | Reaction::Deroutement => {
                        emet.emettre(Evenement::Statut("Reprise après erreur provider.".into()));
                    }
                    Reaction::Stopper(motif) => return Err(motif),
                }
            }
        }
    }
}

/// Convertit une réponse modèle en [`Issue`] exploitable par la boussole. Applique
/// au passage les effets de bord « plan » (mise à jour de l'itinéraire).
fn analyser(reponse: &ReponseModele, carnet: &mut Carnet) -> Issue {
    let mut appels = reponse.appels.clone();

    // `plan` : effet de bord (pose/maj l'itinéraire), retiré de la liste d'appels.
    // Deux formats acceptés : `items:[{task,status}]` (statuts préservés, le modèle
    // ré-émet son plan à jour chaque tour) ou `steps:[titres]` (tout à faire).
    if let Some(pos) = appels.iter().position(|a| a.nom == "plan") {
        let a = appels.remove(pos);
        if let Some(items) = a.args.get("items").and_then(|v| v.as_array()) {
            let etapes: Vec<crate::itineraire::Etape> = items
                .iter()
                .filter_map(|it| {
                    let titre = it
                        .get("task")
                        .or_else(|| it.get("titre"))
                        .and_then(|v| v.as_str())?;
                    let statut = it
                        .get("status")
                        .or_else(|| it.get("statut"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("pending");
                    Some(crate::itineraire::Etape {
                        titre: titre.to_string(),
                        statut: crate::itineraire::StatutEtape::depuis(statut),
                    })
                })
                .collect();
            if !etapes.is_empty() {
                carnet.itineraire.etapes = etapes;
            }
        } else if let Some(steps) = a.args.get("steps").and_then(|v| v.as_array()) {
            let titres: Vec<String> = steps
                .iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect();
            if !titres.is_empty() {
                carnet.itineraire.definir(titres);
            }
        }
    }

    // Outils de fin explicite : `mission_accomplie` (butinage natif) ou `task_complete`
    // (déjà enregistré dans LaRuche). On reconnaît les deux.
    if let Some(a) = appels
        .iter()
        .find(|a| a.nom == "mission_accomplie" || a.nom == "task_complete")
    {
        let resume = a
            .args
            .get("resume")
            .or_else(|| a.args.get("summary"))
            .and_then(|v| v.as_str())
            .unwrap_or("Mission accomplie.")
            .to_string();
        let confiance = a
            .args
            .get("confiance")
            .or_else(|| a.args.get("confidence"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        return Issue::MissionAccomplie { resume, confiance };
    }

    if let Some(a) = appels.iter().find(|a| a.nom == "clarify") {
        let q = a
            .args
            .get("question")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        return Issue::Clarification(q);
    }

    if !appels.is_empty() {
        return Issue::Outils(appels);
    }

    let tronquee = matches!(reponse.stop, StopReason::Longueur) || bloc_outil_non_ferme(&reponse.texte);
    Issue::TexteSeul(TexteSeul {
        texte: reponse.texte.clone(),
        fin_native: Some(reponse.stop),
        plan_inacheve: carnet.itineraire.a_des_ouvertes(),
        malforme: ressemble_a_un_outil(&reponse.texte),
        tronquee,
    })
}

/// Le texte ressemble-t-il à un tool_call (mais n'a pas été parsé) ? Rail pour modèles faibles.
fn ressemble_a_un_outil(t: &str) -> bool {
    t.contains("<tool_call>") || (t.contains("\"name\"") && t.contains("\"arguments\""))
}

/// Bloc d'outil ouvert mais jamais fermé → sortie probablement tronquée.
fn bloc_outil_non_ferme(t: &str) -> bool {
    let ouverts = t.matches("<tool_call>").count();
    let fermes = t.matches("</tool_call>").count();
    ouverts > fermes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carnet::ModeMission;
    use crate::evenement::Silencieux;
    use crate::fournisseur::ErreurFournisseur;
    use crate::issue::Appel;
    use crate::outils::ResultatOutil;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Mutex;

    fn t0() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    /// Fournisseur mock : débite des réponses pré-enregistrées, ou une erreur fixe.
    struct FournisseurScript {
        reponses: Mutex<std::collections::VecDeque<ReponseModele>>,
        erreur: Option<ErreurFournisseur>,
    }
    impl FournisseurScript {
        fn scenario(reponses: Vec<ReponseModele>) -> Self {
            Self { reponses: Mutex::new(reponses.into()), erreur: None }
        }
        fn en_erreur(status: u16) -> Self {
            Self {
                reponses: Mutex::new(Default::default()),
                erreur: Some(ErreurFournisseur { status, retry_after: None, corps: "boom".into() }),
            }
        }
    }
    #[async_trait]
    impl Fournisseur for FournisseurScript {
        async fn repondre(
            &self,
            _m: &[Message],
            _s: &[serde_json::Value],
        ) -> Result<ReponseModele, ErreurFournisseur> {
            if let Some(e) = &self.erreur {
                return Err(e.clone());
            }
            Ok(self.reponses.lock().unwrap().pop_front().unwrap_or(ReponseModele {
                texte: "(plus de script)".into(),
                stop: StopReason::FinTour,
                appels: vec![],
                usage: None,
            }))
        }
    }

    struct OutilsMock;
    #[async_trait]
    impl Outils for OutilsMock {
        async fn executer(&self, appel: &Appel) -> ResultatOutil {
            ResultatOutil::ok(format!("résultat de {}", appel.nom))
        }
        fn idempotent(&self, nom: &str) -> bool {
            nom.starts_with("web_")
        }
    }

    fn rep_texte(t: &str) -> ReponseModele {
        ReponseModele { texte: t.into(), stop: StopReason::FinTour, appels: vec![], usage: None }
    }
    fn rep_appel(nom: &str, args: serde_json::Value) -> ReponseModele {
        ReponseModele { texte: String::new(), stop: StopReason::Outils, appels: vec![Appel::nouveau(nom, args)], usage: None }
    }

    #[tokio::test]
    async fn termine_sur_mission_accomplie() {
        let four = FournisseurScript::scenario(vec![rep_appel(
            "mission_accomplie",
            json!({"resume": "tout est fait", "confiance": 0.9}),
        )]);
        let mut carnet = Carnet::ouvrir("fais X", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Accomplie);
        assert_eq!(bilan.texte, "tout est fait");
        assert_eq!(bilan.passes, 1);
    }

    #[tokio::test]
    async fn execute_un_outil_puis_termine() {
        let four = FournisseurScript::scenario(vec![
            rep_appel("web_search", json!({"q": "rust"})),
            rep_appel("mission_accomplie", json!({"resume": "trouvé"})),
        ]);
        let mut carnet = Carnet::ouvrir("cherche", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Accomplie);
        assert_eq!(carnet.recolte_web, 1, "l'appel web doit être compté");
        // une observation d'outil a été réinjectée dans l'historique
        assert!(carnet
            .historique
            .iter()
            .any(|m| m.outil.as_deref() == Some("web_search")));
        assert_eq!(bilan.passes, 2);
    }

    #[tokio::test]
    async fn texte_seul_standard_termine_tout_de_suite() {
        let four = FournisseurScript::scenario(vec![rep_texte("voici la réponse directe")]);
        let mut carnet = Carnet::ouvrir("salut", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Accomplie);
        assert_eq!(bilan.texte, "voici la réponse directe");
    }

    #[tokio::test]
    async fn plan_force_l_auto_continuation() {
        // 1) pose un plan (1 étape) ; 2) texte seul (étape encore ouverte) → doit relancer ;
        // 3) mission_accomplie → fin. La passe 2 ne doit PAS conclure.
        let four = FournisseurScript::scenario(vec![
            rep_appel("plan", json!({"steps": ["chercher la source"]})),
            rep_texte("je réfléchis à voix haute mais je n'agis pas"),
            rep_appel("mission_accomplie", json!({"resume": "ok"})),
        ]);
        let mut carnet = Carnet::ouvrir("mission", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Accomplie);
        assert!(carnet.auto_continue >= 1 || carnet.passe >= 2, "l'auto-continuation a dû se déclencher");
    }

    #[tokio::test]
    async fn erreur_fatale_arrete_proprement() {
        let four = FournisseurScript::en_erreur(400); // requête invalide → fatal, pas de repli
        let mut carnet = Carnet::ouvrir("x", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux, None)
            .await
            .unwrap();
        assert!(matches!(bilan.fin, FinDeVol::Erreur(_)));
    }
}
