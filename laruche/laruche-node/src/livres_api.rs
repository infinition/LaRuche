//! Ce que la mise a jour n'a PAS ose toucher, rendu visible.
//!
//! Le registre de `main.rs` decide seul dans deux cas sur quatre: une capacite
//! que l'utilisateur a effacee reste morte, une capacite qu'il a modifiee reste
//! la sienne. C'est le bon choix par defaut, mais pris en silence il devient un
//! secret: on ne sait ni qu'une nouveaute existe, ni que la version livree a
//! change sous une modification qu'on avait faite il y a trois mois.
//!
//! Ces routes ne decident rien. Elles montrent, et laissent choisir. Un indicateur
//! qu'on ignore reste affiche: ne rien decider est une reponse valable, ce n'est
//! pas une raison de faire disparaitre la question.

use crate::AppState;
use axum::{extract::State, Json};
use std::sync::Arc;

/// Le nom du skill porte par un chemin livre (`arxiv/SKILL.md` -> `arxiv`).
fn skill_de(chemin: &str) -> String {
    chemin.split('/').next().unwrap_or(chemin).to_string()
}

fn parcourir(d: &include_dir::Dir<'_>, sortie: &mut Vec<(String, Vec<u8>)>) {
    for f in d.files() {
        sortie.push((
            f.path().to_string_lossy().replace('\\', "/"),
            f.contents().to_vec(),
        ));
    }
    for sd in d.dirs() {
        parcourir(sd, sortie);
    }
}

/// GET /api/skills/livres - l'etat de chaque fichier livre face au foyer.
///
/// `identique` n'est pas renvoye: seul ce qui demande une decision remonte, sinon
/// la liste serait un inventaire de quarante skills ou l'on chercherait les deux
/// qui comptent.
pub(crate) async fn api_livres_etat() -> Json<serde_json::Value> {
    let racine = std::path::Path::new("skills");
    let reg = crate::registre_livre(racine);
    let mut livres = Vec::new();
    parcourir(&crate::SKILLS_LIVRES, &mut livres);

    let mut sortie = Vec::new();
    for (chemin, contenu) in livres {
        let livre_h = crate::empreinte(&contenu);
        let sur_disque = std::fs::read(racine.join(&chemin)).ok();
        let actuel_h = sur_disque.as_deref().map(crate::empreinte);
        let etat = match (&actuel_h, reg.get(&chemin)) {
            // Sur le disque et identique au livre: rien a dire.
            (Some(a), _) if *a == livre_h => continue,
            // Absent, et nous l'avions depose: efface volontairement.
            (None, Some(_)) => "manquant",
            // Absent et jamais depose: `tenir_a_jour` s'en charge au demarrage,
            // il ne reste donc rien a decider.
            (None, None) => continue,
            // Present et different: soit le sien, soit une version livree plus
            // recente qu'il n'a pas encore vue.
            (Some(_), _) => "different",
        };
        sortie.push(serde_json::json!({
            "chemin": chemin, "skill": skill_de(&chemin), "etat": etat,
        }));
    }
    Json(serde_json::json!({ "entrees": sortie }))
}

/// GET /api/skills/livres/contenu?chemin=... - les deux versions, pour comparer.
pub(crate) async fn api_livres_contenu(
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let chemin = q.get("chemin").map(String::as_str).unwrap_or("");
    // Le chemin vient du client et sert a lire un fichier: on refuse tout ce qui
    // pourrait remonter l'arborescence plutot que de tenter de le nettoyer.
    if chemin.is_empty() || chemin.contains("..") || chemin.starts_with('/') {
        return Json(serde_json::json!({ "error": "chemin invalide" }));
    }
    let mut livres = Vec::new();
    parcourir(&crate::SKILLS_LIVRES, &mut livres);
    let livre = livres
        .into_iter()
        .find(|(c, _)| c == chemin)
        .map(|(_, v)| String::from_utf8_lossy(&v).to_string());
    let Some(livre) = livre else {
        return Json(serde_json::json!({ "error": "inconnu" }));
    };
    let actuel = std::fs::read_to_string(std::path::Path::new("skills").join(chemin)).ok();
    Json(serde_json::json!({ "chemin": chemin, "livre": livre, "actuel": actuel }))
}

/// POST /api/skills/livres/appliquer {chemin} - ecrit la version livree.
///
/// C'est le SEUL endroit qui ecrase, et il ne s'execute que sur demande
/// explicite. Le registre est mis a jour dans la foulee, sans quoi l'indicateur
/// reviendrait au redemarrage suivant en accusant l'utilisateur d'une
/// modification qu'il vient justement d'accepter d'abandonner.
pub(crate) async fn api_livres_appliquer(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let chemin = body["chemin"].as_str().unwrap_or("");
    if chemin.is_empty() || chemin.contains("..") || chemin.starts_with('/') {
        return Json(serde_json::json!({ "status": "error", "error": "chemin invalide" }));
    }
    let mut livres = Vec::new();
    parcourir(&crate::SKILLS_LIVRES, &mut livres);
    let Some((_, contenu)) = livres.into_iter().find(|(c, _)| c == chemin) else {
        return Json(serde_json::json!({ "status": "error", "error": "inconnu" }));
    };
    let racine = std::path::Path::new("skills");
    let cible = racine.join(chemin);
    if let Some(parent) = cible.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&cible, &contenu) {
        Ok(()) => {
            let mut reg = crate::registre_livre(racine);
            reg.insert(chemin.to_string(), crate::empreinte(&contenu));
            crate::ecrire_registre(racine, &reg);
            Json(serde_json::json!({ "status": "ok" }))
        }
        Err(e) => Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
    }
}

/// POST /api/skills/livres/ignorer {chemin} - garder SA version, sans y revenir.
///
/// L'indicateur reste tant qu'on n'a rien decide, c'est voulu. Mais decider de
/// garder sa version EST une decision: on note l'empreinte actuelle comme etant
/// la reference, et la question ne se repose qu'a la prochaine divergence.
pub(crate) async fn api_livres_ignorer(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let chemin = body["chemin"].as_str().unwrap_or("");
    if chemin.is_empty() || chemin.contains("..") || chemin.starts_with('/') {
        return Json(serde_json::json!({ "status": "error", "error": "chemin invalide" }));
    }
    let racine = std::path::Path::new("skills");
    let actuel = std::fs::read(racine.join(chemin)).ok();
    let mut reg = crate::registre_livre(racine);
    match actuel {
        Some(v) => {
            reg.insert(chemin.to_string(), crate::empreinte(&v));
        }
        // Absent: on note une empreinte impossible, ce qui vaut « efface, et je le
        // sais ». Le fichier ne reviendra pas et l'indicateur s'eteint.
        None => {
            reg.insert(chemin.to_string(), "efface".to_string());
        }
    }
    crate::ecrire_registre(racine, &reg);
    Json(serde_json::json!({ "status": "ok" }))
}

#[cfg(test)]
mod tests_livres {
    use super::skill_de;

    #[test]
    fn le_nom_du_skill_est_le_premier_segment() {
        assert_eq!(skill_de("arxiv/SKILL.md"), "arxiv");
        assert_eq!(skill_de("laruche/wiki/concepts/LaReine.md"), "laruche");
        assert_eq!(skill_de("AUTHORING.md"), "AUTHORING.md");
    }
}
