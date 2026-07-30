//! Cote noeud: resoudre les profils, appeler les modeles, exposer le pool.
//!
//! Le moteur et l'executeur vivent dans `laruche-essaim` et ne connaissent ni les
//! profils de fournisseurs ni la memoire. Ce module fait la jonction: il implemente les
//! deux traits dont l'executeur a besoin et sert les routes.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use laruche_essaim::deliberation as delib;
use std::sync::Arc;

/// Resout un profil de specialiste en identifiants utilisables.
pub(crate) struct ProfilsNoeud {
    /// Copie du fichier de profils, prise au debut de la deliberation.
    ///
    /// Une copie plutot qu'un verrou tenu pendant tout le debat: une deliberation dure
    /// des minutes, et bloquer les reglages pendant ce temps rendrait l'interface
    /// inutilisable. Un profil modifie en cours de route ne prend effet qu'a la
    /// deliberation suivante, ce qui est le comportement le moins surprenant.
    config: profiles::ProfilesConfig,
    ollama_url: String,
}

impl ProfilsNoeud {
    pub(crate) async fn nouveau(state: &Arc<AppState>) -> Self {
        Self {
            config: state.profiles.read().await.clone(),
            ollama_url: state.essaim_config.read().await.ollama_url.clone(),
        }
    }

    fn creds_de(&self, id: &str, modele: Option<&str>) -> Option<delib::Creds> {
        let p = self.config.profiles.get(id)?;
        // Un profil sans modele connu n'est pas utilisable: mieux vaut retomber sur
        // l'actif que d'appeler un fournisseur avec un nom de modele vide.
        let model = modele
            .map(|m| m.to_string())
            .or_else(|| p.models.first().cloned())?;
        Some(delib::Creds {
            provider: p.provider.clone(),
            model,
            // La cle passe par le coffre: un profil peut stocker `${MA_CLE}` plutot que
            // le secret en clair, comme partout ailleurs.
            api_key: laruche_essaim::secrets::substituer(&p.api_key),
            api_base: Some(p.base_url.clone()).filter(|b| !b.is_empty()),
            ollama_url: self.ollama_url.clone(),
        })
    }
}

impl delib::Profils for ProfilsNoeud {
    fn resoudre(&self, profil: &str) -> delib::Creds {
        // 1. Le profil demande par le specialiste.
        if !profil.is_empty() {
            if let Some(c) = self.creds_de(profil, None) {
                return c;
            }
            // Repli silencieux, mais trace: un profil supprime ne doit pas interrompre
            // la deliberation, seulement la rendre moins diverse. L'interface le
            // signalera; le moteur, lui, continue.
            tracing::warn!(
                profil = %profil,
                "deliberation: profil introuvable, repli sur le profil actif"
            );
        }
        // 2. Le profil actif.
        let actif = &self.config.active_model;
        self.creds_de(&actif.profile_id, Some(&actif.model))
            .unwrap_or_else(|| {
                // 3. Rien d'exploitable: Ollama local, qui est le defaut de LaRuche.
                delib::Creds {
                    provider: "ollama".into(),
                    model: actif.model.clone(),
                    api_key: String::new(),
                    api_base: None,
                    ollama_url: self.ollama_url.clone(),
                }
            })
    }
}

/// Appelle un modele et rend son texte avec le cout.
pub(crate) struct AppelNoeud;

#[async_trait::async_trait]
impl delib::Appel for AppelNoeud {
    async fn demander(
        &self,
        creds: &delib::Creds,
        systeme: &str,
        utilisateur: &str,
    ) -> anyhow::Result<(String, u32)> {
        let messages = vec![
            serde_json::json!({ "role": "system", "content": systeme }),
            serde_json::json!({ "role": "user", "content": utilisateur }),
        ];
        let mut flux = laruche_essaim::providers::provider_chat_stream(
            &creds.provider,
            &creds.model,
            &messages,
            0.6,
            2048,
            &creds.api_key,
            creds.api_base.as_deref(),
            &creds.ollama_url,
            None,
        )
        .await?;

        use futures_util::StreamExt;
        let mut texte = String::new();
        let mut jetons = 0u32;
        while let Some(chunk) = flux.next().await {
            texte.push_str(&chunk.text);
            // Le decompte arrive sur le dernier morceau. A defaut, on estime: sans
            // estimation, un fournisseur muet sur ses jetons rendrait le plafond
            // inoperant, ce qui est exactement le cas ou il sert.
            if let Some(n) = chunk.eval_count {
                jetons += n as u32;
            }
            if let Some(n) = chunk.prompt_eval_count {
                jetons += n as u32;
            }
        }
        if jetons == 0 {
            jetons = ((systeme.len() + utilisateur.len() + texte.len()) / 4) as u32;
        }
        Ok((texte, jetons))
    }
}

/// GET /api/deliberation/pool - le pool complet, embauches et reserve.
pub(crate) async fn api_pool() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "specialistes": delib::pool(),
        "missions": [
            { "id": "reponse", "nom": "Répondre", "acces": delib::Mission::Reponse.acces(),
              "livrable": delib::Mission::Reponse.livrable(), "outils": delib::Mission::Reponse.outils_disponibles() },
            { "id": "code", "nom": "Coder", "acces": delib::Mission::Code.acces(),
              "livrable": delib::Mission::Code.livrable(), "outils": delib::Mission::Code.outils_disponibles() },
            { "id": "recherche", "nom": "Chercher", "acces": delib::Mission::Recherche.acces(),
              "livrable": delib::Mission::Recherche.livrable(), "outils": delib::Mission::Recherche.outils_disponibles() },
            { "id": "experimentation", "nom": "Expérimenter", "acces": delib::Mission::Experimentation.acces(),
              "livrable": delib::Mission::Experimentation.livrable(), "outils": delib::Mission::Experimentation.outils_disponibles() },
        ],
    }))
}

/// POST /api/deliberation/pool - remplace la liste des specialistes personnalises.
pub(crate) async fn api_pool_set(
    Json(body): Json<Vec<delib::Specialiste>>,
) -> Json<serde_json::Value> {
    match delib::specialiste::sauver_personnalises(&body) {
        Ok(()) => Json(serde_json::json!({ "status": "ok", "count": body.len() })),
        Err(e) => Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
    }
}

/// GET /api/deliberation/constitution - le socle effectif.
pub(crate) async fn api_constitution(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // Surcharge depuis la memoire, comme la charte de LaReine.
    let surcharge =
        laruche_essaim::brain::charger_doc_systeme(&state.memoire, "system.constitution")
            .await
            .filter(|s| !s.trim().is_empty());
    Json(serde_json::json!({
        "constitution": delib::constitution_effective(surcharge.as_deref()),
        "personnalisee": surcharge.is_some(),
        "socle": delib::CONSTITUTION,
    }))
}

/// POST /api/deliberation/run - fait deliberer la table, en FLUX.
///
/// NDJSON plutot que WebSocket: la WS du chat est couplee a une session, et une
/// deliberation n'en est pas une. Un flux de reponse suffit, il donne le temps reel
/// sans plomberie - et il regle au passage le delai d'attente du navigateur, qui
/// abandonnait avant la fin d'un debat de plusieurs minutes.
///
/// Une ligne JSON par evenement: `debut`, `etape` (qui reflechit maintenant),
/// `intervention` (des qu'elle arrive), `fin`.
pub(crate) async fn api_run(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let question = body["question"].as_str().unwrap_or("").trim().to_string();
    let mission: delib::Mission = body
        .get("mission")
        .and_then(|m| serde_json::from_value(m.clone()).ok())
        .unwrap_or_default();
    let tours_max = body["tours_max"].as_u64().unwrap_or(3) as u8;
    let demandes: Option<Vec<String>> = body["participants"].as_array().map(|a| {
        a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()
    });

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<String, std::io::Error>>(64);

    tokio::spawn(async move {
        let envoyer = |v: serde_json::Value| {
            let tx = tx.clone();
            async move {
                let _ = tx.send(Ok(format!("{v}
"))).await;
            }
        };

        if question.is_empty() {
            envoyer(serde_json::json!({ "type": "fin", "erreur": "question vide" })).await;
            return;
        }

        let pool = delib::pool();
        let embauches: Vec<String> =
            pool.iter().filter(|s| s.embauche).map(|s| s.id.clone()).collect();
        // Les EMBAUCHES d'abord. L'equipe par defaut de la mission n'est qu'un repli
        // pour une installation ou personne n'a encore choisi: elle prenait le pas sur
        // les embauches, si bien qu'embaucher l'ingenieur et l'attaquant ne changeait
        // rien - seuls le scientifique et le contradicteur parlaient.
        //
        // L'orchestrateur et l'arbitre sont structurels: le premier ne debat pas, le
        // second conclut. Ils ne comptent pas comme participants.
        let a_la_table: Vec<String> = pool
            .iter()
            .filter(|s| {
                s.embauche
                    && s.role != delib::Role::Orchestrateur
                    && s.role != delib::Role::Arbitre
            })
            .map(|s| s.id.clone())
            .collect();
        let participants: Vec<String> = demandes
            .unwrap_or_else(|| {
                if a_la_table.is_empty() {
                    mission.equipe_par_defaut().iter().map(|s| s.to_string()).collect()
                } else {
                    a_la_table.clone()
                }
            })
            .into_iter()
            .filter(|id| embauches.contains(id))
            .collect();

        let plan = delib::Plan {
            mission,
            participants: participants.clone(),
            tours_max,
            raison: String::new(),
        };
        let constitution =
            laruche_essaim::brain::charger_doc_systeme(&state.memoire, "system.constitution")
                .await
                .filter(|s| !s.trim().is_empty());
        let constitution = delib::constitution_effective(constitution.as_deref()).to_string();
        let profils = ProfilsNoeud::nouveau(&state).await;

        envoyer(serde_json::json!({
            "type": "debut",
            "question": question,
            "mission": mission,
            "participants": participants,
        }))
        .await;

        let mut d = delib::Deliberation::nouvelle(
            question,
            plan,
            delib::Reglages::default(),
            pool,
        );

        // On pilote la boucle a la main plutot que d'appeler `deliberer`: c'est ce qui
        // permet d'annoncer QUI reflechit avant de l'attendre. Sans cet evenement, la
        // table s'anime en bloc et on ne voit pas la delegation se faire.
        let mut garde = 0;
        loop {
            garde += 1;
            if garde > 32 {
                break;
            }
            let etape = d.prochaine_etape();
            let (nom, acteurs) = match &etape {
                delib::Etape::Solo(v) => ("solo", v.clone()),
                delib::Etape::Relecture(v) => ("relecture", v.clone()),
                delib::Etape::Contradiction(i) => ("contradiction", vec![i.clone()]),
                delib::Etape::Reponse(v) => ("reponse", v.clone()),
                delib::Etape::Synthese(i) => ("synthese", vec![i.clone()]),
                delib::Etape::Fini(_) => break,
            };
            envoyer(serde_json::json!({
                "type": "etape", "nom": nom, "acteurs": acteurs, "tour": d.tour_courant + 1,
            }))
            .await;

            let lot = delib::executeur::jouer_etape(
                &d, &etape, &AppelNoeud, &profils, &constitution,
            )
            .await;
            if lot.is_empty() {
                break;
            }
            d.deposer(lot.clone());
            for iv in &d.interventions[d.interventions.len() - lot.len()..] {
                envoyer(serde_json::json!({ "type": "intervention", "intervention": iv })).await;
            }
        }

        let repartition: Vec<serde_json::Value> = d
            .repartition()
            .into_iter()
            .map(|(id, accord, confiance)| {
                serde_json::json!({
                    "specialiste": id, "accord": accord,
                    "symbole": accord.symbole(), "confiance": confiance,
                })
            })
            .collect();
        // On enregistre AVANT d'annoncer la fin: si l'ecriture echoue, l'interface
        // recevra quand meme son evenement et le debat ne sera pas perdu a l'ecran.
        let id = format!(
            "{}-{}",
            chrono::Utc::now().format("%Y%m%d-%H%M%S"),
            &uuid::Uuid::new_v4().to_string()[..8]
        );
        let enregistre = serde_json::json!({
            "id": id,
            "quand": chrono::Utc::now().timestamp(),
            "question": d.question,
            "mission": d.plan.mission,
            "participants": d.plan.participants,
            "tours": d.tour_courant,
            "jetons": d.jetons_consommes(),
            "interventions": d.interventions,
            "repartition": repartition,
            "dissidents": d.dissidents(),
        });
        if let Err(e) = std::fs::write(
            dossier_tours().join(format!("{id}.json")),
            serde_json::to_string_pretty(&enregistre).unwrap_or_default(),
        ) {
            tracing::warn!(error = %e, "deliberation: tour de table non enregistre");
        }

        envoyer(serde_json::json!({
            "type": "fin",
            "id": id,
            "tours": d.tour_courant,
            "jetons": d.jetons_consommes(),
            "arret": match d.prochaine_etape() {
                delib::Etape::Fini(a) => serde_json::to_value(a).unwrap_or(serde_json::Value::Null),
                _ => serde_json::Value::Null,
            },
            "repartition": repartition,
            "dissidents": d.dissidents(),
        }))
        .await;
    });

    // On transforme le recepteur en flux avec futures_util, deja dependance du
    // noeud, plutot que d'ajouter tokio-stream pour une seule ligne.
    let flux = futures_util::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|v| (v, rx))
    });
    (
        [(axum::http::header::CONTENT_TYPE, "application/x-ndjson")],
        axum::body::Body::from_stream(flux),
    )
        .into_response()
}

/// Ou vivent les tours de table passes.
///
/// Un dossier de fichiers plutot que la memoire cognitive: un debat est une TRACE,
/// pas un fait appris. L'y verser polluerait le rappel avec sept opinions dont six
/// ont ete ecartees - et c'est exactement ce que la memoire ne doit pas contenir.
fn dossier_tours() -> std::path::PathBuf {
    let d = std::path::PathBuf::from("deliberations");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// GET /api/deliberation/tours - la liste, du plus recent au plus ancien.
pub(crate) async fn api_tours() -> Json<serde_json::Value> {
    let mut tours: Vec<serde_json::Value> = std::fs::read_dir(dossier_tours())
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                .filter_map(|e| {
                    let v: serde_json::Value =
                        serde_json::from_str(&std::fs::read_to_string(e.path()).ok()?).ok()?;
                    // On ne rend que l'entete: la liste n'a pas besoin des
                    // interventions, et les charger toutes rendrait le volet lent
                    // des la dixieme deliberation.
                    Some(serde_json::json!({
                        "id": v["id"],
                        "question": v["question"],
                        "quand": v["quand"],
                        "tours": v["tours"],
                        "dissidents": v["dissidents"],
                    }))
                })
                .collect()
        })
        .unwrap_or_default();
    tours.sort_by(|a, b| b["quand"].as_i64().cmp(&a["quand"].as_i64()));
    Json(serde_json::json!({ "tours": tours }))
}

/// GET /api/deliberation/tour/:id - un debat complet.
pub(crate) async fn api_tour(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    // Un identifiant venant de l'URL ne doit jamais devenir un chemin: on ne garde
    // que ce qu'on a nous-memes produit, des caracteres de nom de fichier surs.
    let sur: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(64)
        .collect();
    match std::fs::read_to_string(dossier_tours().join(format!("{sur}.json"))) {
        Ok(t) => Json(serde_json::from_str(&t).unwrap_or(serde_json::json!({}))),
        Err(_) => Json(serde_json::json!({ "error": "introuvable" })),
    }
}

/// DELETE /api/deliberation/tour/:id
pub(crate) async fn api_tour_supprimer(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let sur: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(64)
        .collect();
    let _ = std::fs::remove_file(dossier_tours().join(format!("{sur}.json")));
    Json(serde_json::json!({ "status": "ok" }))
}
