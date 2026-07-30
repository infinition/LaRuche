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
            { "id": "reponse", "nom": "Répondre", "acces": delib::Mission::Reponse.acces() },
            { "id": "code", "nom": "Coder", "acces": delib::Mission::Code.acces() },
            { "id": "recherche", "nom": "Chercher", "acces": delib::Mission::Recherche.acces() },
            { "id": "experimentation", "nom": "Expérimenter", "acces": delib::Mission::Experimentation.acces() },
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

/// POST /api/deliberation/run - fait deliberer la table et rend le debat complet.
///
/// Synchrone pour l'instant: une deliberation dure des minutes, ce qui depasse le
/// delai d'attente d'un navigateur. L'interface passera par la WebSocket - c'est la
/// meme mecanique que le chat, et elle permettra d'animer la table au fil des tours
/// plutot que de faire patienter devant un ecran fige.
pub(crate) async fn api_run(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let question = body["question"].as_str().unwrap_or("").trim().to_string();
    if question.is_empty() {
        return Json(serde_json::json!({ "status": "error", "error": "question vide" }));
    }
    let mission: delib::Mission = body
        .get("mission")
        .and_then(|m| serde_json::from_value(m.clone()).ok())
        .unwrap_or_default();

    // Les participants demandes, sinon l'equipe par defaut de la mission, filtree par
    // ce qui est reellement embauche: un specialiste mis en reserve ne doit pas
    // reapparaitre parce qu'une mission le prevoit.
    let pool = delib::pool();
    let embauches: Vec<String> = pool
        .iter()
        .filter(|s| s.embauche)
        .map(|s| s.id.clone())
        .collect();
    let demandes: Vec<String> = body["participants"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_else(|| {
            mission
                .equipe_par_defaut()
                .iter()
                .map(|s| s.to_string())
                .collect()
        });
    let participants: Vec<String> = demandes
        .into_iter()
        .filter(|id| embauches.contains(id))
        .collect();

    let plan = delib::Plan {
        mission,
        participants,
        tours_max: body["tours_max"].as_u64().unwrap_or(3) as u8,
        raison: body["raison"].as_str().unwrap_or("").to_string(),
    };

    let constitution =
        laruche_essaim::brain::charger_doc_systeme(&state.memoire, "system.constitution")
            .await
            .filter(|s| !s.trim().is_empty());
    let constitution = delib::constitution_effective(constitution.as_deref()).to_string();

    let profils = ProfilsNoeud::nouveau(&state).await;
    let mut d = delib::Deliberation::nouvelle(
        question,
        plan,
        delib::Reglages::default(),
        pool,
    );
    delib::deliberer(&mut d, &AppelNoeud, &profils, &constitution).await;

    // La repartition, jamais un pourcentage: un accord entre modeles mesure la
    // conformite et serait lu comme une confiance.
    let repartition: Vec<serde_json::Value> = d
        .repartition()
        .into_iter()
        .map(|(id, accord, confiance)| {
            serde_json::json!({
                "specialiste": id,
                "accord": accord,
                "symbole": accord.symbole(),
                "confiance": confiance,
            })
        })
        .collect();

    Json(serde_json::json!({
        "status": "ok",
        "question": d.question,
        "mission": d.plan.mission,
        "participants": d.plan.participants,
        "tours": d.tour_courant,
        "jetons": d.jetons_consommes(),
        "arret": match d.prochaine_etape() {
            delib::Etape::Fini(a) => serde_json::to_value(a).unwrap_or(serde_json::Value::Null),
            _ => serde_json::Value::Null,
        },
        "interventions": d.interventions,
        "repartition": repartition,
        // Le livrable qui compte: ce sur quoi la table n'est PAS d'accord.
        "dissidents": d.dissidents(),
    }))
}
