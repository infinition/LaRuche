//! Purge des episodes.
//!
//! Un episode est le compte rendu d'une mission, ecrit sous
//! `episodes.<date>.<slug>` a chaque fois que l'agent termine un travail un peu
//! long. Rien ne les efface jamais.
//!
//! Apres quelques mois d'usage quotidien, la carte cognitive est surtout faite
//! de comptes rendus: le rappel remonte alors ce que l'agent faisait en juillet
//! plutot que le fait qu'on cherchait. Deux facons d'y remedier, et les deux
//! sont ici: tout effacer d'un geste, ou fixer un age au-dela duquel un episode
//! s'en va tout seul.
//!
//! Ce qui n'est PAS fait, volontairement: aucune purge par defaut. Effacer la
//! memoire de quelqu'un sans le lui demander ne se fait pas, donc le reglage
//! part a zero et ne s'active que si l'utilisateur le decide.

use crate::*;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;

/// La date portee par un noeud d'episode.
///
/// Deux formats coexistent dans l'historique: `2026_07_21` du cote du moteur de
/// butinage, `2026-07-21` du cote des taches de fond. Les accepter tous les deux
/// coute deux lignes; ne pas le faire laisserait la moitie des episodes hors de
/// portee de la purge, sans que rien ne l'explique.
fn date_du_noeud(node_id: &str) -> Option<chrono::NaiveDate> {
    let dernier = node_id.rsplit('.').next()?;
    for forme in ["%Y_%m_%d", "%Y-%m-%d"] {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(dernier, forme) {
            return Some(d);
        }
    }
    None
}

/// Ce qu'une purge a emporte.
pub(crate) struct Bilan {
    pub jours: usize,
    pub gardes: usize,
    /// Noeuds dont la date est illisible: gardes, et signales.
    pub illisibles: usize,
}

/// Efface les episodes anterieurs a `avant`, ou tous si `avant` vaut `None`.
///
/// La suppression se fait en deux temps parce que la memoire ne detruit rien
/// d'emblee: elle deplace le sous-arbre sous `orphans.<nom>_<horodatage>` pour
/// qu'un geste malheureux reste rattrapable. Ici l'intention est bien d'effacer,
/// donc on reprend l'orphelin et on le supprime pour de bon. Sans cette seconde
/// passe, "vider les episodes" les deplacerait simplement ailleurs, en laissant
/// croire au menage.
pub(crate) async fn purger(
    memoire: &Arc<dyn laruche_memoire::MemoireCognitive>,
    avant: Option<chrono::NaiveDate>,
) -> Bilan {
    let mut bilan = Bilan {
        jours: 0,
        gardes: 0,
        illisibles: 0,
    };
    let Ok(racine) = memoire.read_node("episodes").await else {
        return bilan; // Aucun episode: rien a faire, et ce n'est pas une erreur.
    };
    let Some(enfants) = racine["children"].as_array() else {
        return bilan;
    };

    for enfant in enfants {
        let Some(id) = enfant["id"]
            .as_str()
            .or_else(|| enfant["node_id"].as_str())
        else {
            continue;
        };
        match (date_du_noeud(id), avant) {
            // Une date illisible n'est jamais effacee par la purge par age: on ne
            // sait pas ce qu'on detruirait. Elle part avec un effacement total,
            // qui lui est explicite.
            (None, Some(_)) => {
                bilan.illisibles += 1;
                continue;
            }
            (Some(d), Some(limite)) if d >= limite => {
                bilan.gardes += 1;
                continue;
            }
            _ => {}
        }
        if let Ok(r) = memoire.delete_node(id).await {
            bilan.jours += 1;
            if let Some(orphelin) = r.get("relocated_to").and_then(|v| v.as_str()) {
                let _ = memoire.delete_node(orphelin).await;
            }
        }
    }
    if bilan.jours > 0 {
        tracing::info!(
            jours = bilan.jours,
            gardes = bilan.gardes,
            "Episodes purges"
        );
    }
    bilan
}

/// Le balayage automatique, quand l'utilisateur a fixe une duree de vie.
///
/// Toutes les six heures plutot qu'une fois par jour: un noeud qui tourne en
/// permanence n'a pas de "demarrage quotidien", et un poste eteint la nuit ne
/// verrait jamais passer une tache calee a trois heures du matin. La granularite
/// du reglage etant le jour, six heures est deja bien plus fin que necessaire.
///
/// Premiere passe differee d'un quart d'heure: le demarrage a mieux a faire, et
/// rien ne presse pour effacer ce qui attend depuis des semaines.
pub(crate) fn spawn_balayage_episodes(state: &Arc<AppState>) {
    let etat = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(900)).await;
        let mut horloge = tokio::time::interval(std::time::Duration::from_secs(6 * 3600));
        loop {
            horloge.tick().await;
            let jours = etat.essaim_config.read().await.episodes_retention_jours;
            if jours == 0 {
                continue; // Reglage a zero: on ne touche a rien, c'est le defaut.
            }
            let limite = chrono::Local::now().date_naive() - chrono::Duration::days(jours as i64);
            purger(&etat.memoire, Some(limite)).await;
        }
    });
}

/// POST /api/memory/episodes/purge
///
/// `{ "older_than_days": 30 }` n'efface que ce qui a plus de trente jours.
/// `{ "older_than_days": 0 }` ou un corps vide efface TOUT, ce qui est le geste
/// explicite du bouton "tout effacer".
pub(crate) async fn api_purger_episodes(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Destructif et irreversible: on exige une session, comme pour les reglages.
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let jours = body["older_than_days"].as_u64().unwrap_or(0);
    let avant = if jours == 0 {
        None
    } else {
        Some(chrono::Local::now().date_naive() - chrono::Duration::days(jours as i64))
    };
    let bilan = purger(&state.memoire, avant).await;
    Ok(Json(serde_json::json!({
        "status": "ok",
        "deleted_days": bilan.jours,
        "kept_days": bilan.gardes,
        "unreadable_days": bilan.illisibles,
    })))
}

/// GET /api/memory/episodes - de quoi remplir l'ecran de reglages sans deviner.
pub(crate) async fn api_etat_episodes(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let retention = state.essaim_config.read().await.episodes_retention_jours;
    let (mut jours, mut plus_ancien, mut plus_recent) = (0usize, None, None);
    if let Ok(racine) = state.memoire.read_node("episodes").await {
        if let Some(enfants) = racine["children"].as_array() {
            jours = enfants.len();
            for e in enfants {
                let Some(id) = e["id"].as_str().or_else(|| e["node_id"].as_str()) else {
                    continue;
                };
                if let Some(d) = date_du_noeud(id) {
                    if plus_ancien.is_none_or(|a| d < a) {
                        plus_ancien = Some(d);
                    }
                    if plus_recent.is_none_or(|a| d > a) {
                        plus_recent = Some(d);
                    }
                }
            }
        }
    }
    Json(serde_json::json!({
        "days": jours,
        "oldest": plus_ancien.map(|d| d.to_string()),
        "newest": plus_recent.map(|d| d.to_string()),
        "retention_days": retention,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_deux_formats_de_date_sont_lus() {
        // Le moteur de butinage ecrit avec des soulignes, les taches de fond avec
        // des tirets. Les deux existent dans les memoires deja sur disque.
        assert_eq!(
            date_du_noeud("episodes.2026_07_21"),
            chrono::NaiveDate::from_ymd_opt(2026, 7, 21)
        );
        assert_eq!(
            date_du_noeud("episodes.2026-07-21"),
            chrono::NaiveDate::from_ymd_opt(2026, 7, 21)
        );
        // Et on lit bien le DERNIER segment, pas le premier venu.
        assert_eq!(
            date_du_noeud("episodes.2026-07-21.ma_mission"),
            None,
            "un episode feuille n'est pas un jour"
        );
        assert_eq!(date_du_noeud("episodes"), None);
        assert_eq!(date_du_noeud("episodes.divers"), None);
    }
}
