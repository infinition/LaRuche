//! Le **cycle** — la boucle de butinage. Volontairement *bête* : elle orchestre,
//! la politique vit ailleurs ([`crate::cap`]), la classification d'erreurs aussi
//! ([`crate::meteo`]). Aucune heuristique de chaînes ici.
//!
//! Flux d'une passe : assembler le contexte → appeler le modèle (avec météo) →
//! `analyser` la réponse en [`Issue`] → `cap` décide → agir (poser / relancer /
//! récolter) → checkpoint.

use crate::cap::boussole::{cap, Decision};
use crate::cap::vigie::{Signal, Vigie};
use crate::carnet::Carnet;
use crate::evenement::{Emetteur, Evenement};
use crate::fournisseur::{Fournisseur, ReponseModele};
use crate::issue::{Bilan, FinDeVol, Issue, StopReason, TexteSeul};
use crate::messagerie::Message;
use crate::meteo::{reagir, ClasseErreur, Reaction};
use crate::outils::Outils;
use crate::reglages::Reglages;
use std::time::{Duration, Instant};

/// Lance un butinage jusqu'à son terme (mission accomplie, clarification, plafond,
/// erreur fatale ou boucle stérile). Le [`Carnet`] est muté à chaque passe et
/// peut être persisté (reprise après crash).
pub async fn butiner(
    carnet: &mut Carnet,
    reglages: &Reglages,
    fournisseur: &dyn Fournisseur,
    outils: &dyn Outils,
    emet: &dyn Emetteur,
) -> anyhow::Result<Bilan> {
    if carnet.historique.is_empty() {
        let mission = carnet.mission.clone();
        carnet.historique.push(Message::utilisateur(mission));
    }
    let mut vigie = Vigie::nouvelle(reglages.profil.seuils_vigie());
    let schemas = outils.schemas();

    loop {
        if carnet.passe >= reglages.plafond_passes {
            let msg = format!("Plafond de {} passes atteint — peut être incomplet.", reglages.plafond_passes);
            emet.emettre(Evenement::Statut(msg.clone()));
            return Ok(Bilan::nouveau(msg, FinDeVol::Plafond, carnet.passe));
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
                if let Some(bilan) = recolter(&appels, carnet, outils, &mut vigie, emet).await {
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
    if let Some(pos) = appels.iter().position(|a| a.nom == "plan") {
        let a = appels.remove(pos);
        if let Some(steps) = a.args.get("steps").and_then(|v| v.as_array()) {
            let titres: Vec<String> = steps
                .iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect();
            if !titres.is_empty() {
                carnet.itineraire.definir(titres);
            }
        }
    }

    if let Some(a) = appels.iter().find(|a| a.nom == "mission_accomplie") {
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

/// Récolte (exécution des outils) séquentielle, avec surveillance de la vigie.
/// Renvoie `Some(bilan)` si la vigie impose un arrêt (boucle stérile), sinon `None`.
///
/// NOTE : version séquentielle. La récolte **parallèle** des appels read-only
/// (partition) arrive dans un module dédié — voir ARCHI_BUTINAGE.md §recolte.
async fn recolter(
    appels: &[crate::issue::Appel],
    carnet: &mut Carnet,
    outils: &dyn Outils,
    vigie: &mut Vigie,
    emet: &dyn Emetteur,
) -> Option<Bilan> {
    for appel in appels {
        let sig = appel.signature();

        if let Signal::Bloquer(msg) = vigie.avant_appel(sig) {
            carnet
                .historique
                .push(Message::observation(&appel.nom, format!("Blocked: {msg}")));
            emet.emettre(Evenement::ResultatOutil { nom: appel.nom.clone(), ok: false, ms: 0 });
            continue;
        }

        emet.emettre(Evenement::AppelOutil { nom: appel.nom.clone() });
        let t0 = Instant::now();
        let resultat = outils.executer(appel).await;
        let ms = t0.elapsed().as_millis() as u64;

        if outils.est_web(appel) {
            carnet.recolte_web += 1;
        }

        let signal = vigie.apres_appel(
            &appel.nom,
            sig,
            resultat.ok,
            outils.idempotent(&appel.nom),
            resultat.empreinte(),
        );
        emet.emettre(Evenement::ResultatOutil { nom: appel.nom.clone(), ok: resultat.ok, ms });

        let mut observation = resultat.sortie.clone();
        if let Signal::Avertir(m) | Signal::Poser(m) = &signal {
            observation.push_str(&format!("\n\n[vigie: {m}]"));
        }
        carnet
            .historique
            .push(Message::observation(&appel.nom, observation));

        if let Signal::Poser(motif) = signal {
            carnet.itineraire.finaliser();
            return Some(Bilan::nouveau(
                "Arrêt : boucle stérile détectée par la vigie.",
                FinDeVol::BoucleSterile(motif),
                carnet.passe + 1,
            ));
        }
    }
    None
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
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux)
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
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux)
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
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux)
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
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux)
            .await
            .unwrap();
        assert_eq!(bilan.fin, FinDeVol::Accomplie);
        assert!(carnet.auto_continue >= 1 || carnet.passe >= 2, "l'auto-continuation a dû se déclencher");
    }

    #[tokio::test]
    async fn erreur_fatale_arrete_proprement() {
        let four = FournisseurScript::en_erreur(400); // requête invalide → fatal, pas de repli
        let mut carnet = Carnet::ouvrir("x", ModeMission::Standard, t0());
        let bilan = butiner(&mut carnet, &Reglages::default(), &four, &OutilsMock, &Silencieux)
            .await
            .unwrap();
        assert!(matches!(bilan.fin, FinDeVol::Erreur(_)));
    }
}
