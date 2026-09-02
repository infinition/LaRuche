//! Les themes de l'interface: catalogue, lecture, ecriture, suppression.
//!
//! Un theme est un jeu de valeurs pour les jetons CSS de `:root`, rien de plus.
//! L'interface en porte trois integres, ecrits en dur dans la feuille de style; ce
//! module ne s'occupe que de ceux que l'utilisateur fabrique.
//!
//! Ils vivent dans `<foyer>/themes/<id>.json`, a cote des skills et des plugins,
//! pour la meme raison qu'eux: ce sont des choses que l'on cree, que l'on veut
//! retrouver au redemarrage et que l'on peut vouloir copier d'une machine a
//! l'autre. Un fichier par theme, lisible et modifiable a la main.

use crate::AppState;
use axum::extract::{Path as AxPath, State};
use axum::response::Json;
use std::path::PathBuf;
use std::sync::Arc;

/// Le dossier des themes, sous le foyer. Cree a la demande.
fn dossier() -> PathBuf {
    let d = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("themes");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Un identifiant de theme sur qui on peut construire un nom de fichier.
///
/// Le nom vient de l'utilisateur et sert de chemin: sans ce filtre, un theme
/// nomme `../../memoire` ecrirait ailleurs que dans le dossier prevu. On ne garde
/// que ce qui ne peut pas voyager.
fn identifiant_sur(brut: &str) -> Option<String> {
    let id: String = brut
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let id = id.trim_matches('-').to_string();
    (!id.is_empty() && id.len() <= 64).then_some(id)
}

/// GET /api/themes - les themes personnalises, tries par nom.
pub(crate) async fn api_themes_list() -> Json<serde_json::Value> {
    let mut themes: Vec<serde_json::Value> = Vec::new();
    if let Ok(entrees) = std::fs::read_dir(dossier()) {
        for e in entrees.flatten() {
            let chemin = e.path();
            if chemin.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let Ok(texte) = std::fs::read_to_string(&chemin) else {
                continue;
            };
            // Un fichier illisible est ignore, jamais fatal: on edite ces fichiers a
            // la main, une virgule en trop ne doit pas priver de tous les autres.
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&texte) {
                themes.push(v);
            }
        }
    }
    themes.sort_by(|a, b| {
        a["nom"]
            .as_str()
            .unwrap_or("")
            .to_lowercase()
            .cmp(&b["nom"].as_str().unwrap_or("").to_lowercase())
    });
    Json(serde_json::json!({ "themes": themes }))
}

/// POST /api/themes {id?, nom, jetons} - cree ou remplace un theme.
pub(crate) async fn api_themes_save(
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let nom = body["nom"].as_str().unwrap_or("").trim().to_string();
    if nom.is_empty() {
        return Json(serde_json::json!({ "status": "error", "error": "nom requis" }));
    }
    // L'identifiant suit le nom quand il n'est pas donne, mais ne le suit PLUS
    // ensuite: renommer un theme ne doit pas en fabriquer un second a cote.
    let Some(id) = body["id"]
        .as_str()
        .and_then(identifiant_sur)
        .or_else(|| identifiant_sur(&nom))
    else {
        return Json(serde_json::json!({ "status": "error", "error": "identifiant invalide" }));
    };
    let jetons = body["jetons"].clone();
    if !jetons.is_object() {
        return Json(serde_json::json!({ "status": "error", "error": "jetons doit etre un objet" }));
    }
    let theme = serde_json::json!({ "id": id, "nom": nom, "jetons": jetons });
    let chemin = dossier().join(format!("{id}.json"));
    match serde_json::to_string_pretty(&theme)
        .map_err(|e| e.to_string())
        .and_then(|t| std::fs::write(&chemin, t).map_err(|e| e.to_string()))
    {
        Ok(()) => Json(serde_json::json!({ "status": "ok", "theme": theme })),
        Err(e) => Json(serde_json::json!({ "status": "error", "error": e })),
    }
}

/// DELETE /api/themes/:id
pub(crate) async fn api_themes_delete(AxPath(id): AxPath<String>) -> Json<serde_json::Value> {
    let Some(id) = identifiant_sur(&id) else {
        return Json(serde_json::json!({ "status": "error", "error": "identifiant invalide" }));
    };
    let chemin = dossier().join(format!("{id}.json"));
    match std::fs::remove_file(&chemin) {
        Ok(()) => Json(serde_json::json!({ "status": "ok" })),
        // Supprimer ce qui n'est plus la n'est pas une erreur: c'est le resultat voulu.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Json(serde_json::json!({ "status": "ok" }))
        }
        Err(e) => Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
    }
}

/// GET /api/themes/actif - le theme choisi, pour qu'un nouvel appareil le retrouve.
pub(crate) async fn api_theme_actif_get(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let actif = state.theme_actif.read().await.clone();
    Json(serde_json::json!({ "actif": actif }))
}

/// POST /api/themes/actif {actif}
///
/// Le choix vit AUSSI dans le stockage local du navigateur, et ce n'est pas un
/// doublon: c'est lui qui permet de peindre le theme avant le premier rendu, sans
/// attendre une reponse du serveur, donc sans le clignotement blanc. Le serveur
/// garde la reference, pour qu'un navigateur qui n'a jamais vu cette ruche ouvre
/// directement sur le bon theme.
pub(crate) async fn api_theme_actif_set(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let actif = body["actif"].as_str().unwrap_or("defaut").trim().to_string();
    *state.theme_actif.write().await = actif.clone();
    let _ = std::fs::write(dossier().join("actif.txt"), &actif);
    Json(serde_json::json!({ "status": "ok", "actif": actif }))
}

/// Le theme choisi au dernier arret, relu au demarrage.
pub(crate) fn theme_actif_au_demarrage() -> String {
    std::fs::read_to_string(dossier().join("actif.txt"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "defaut".to_string())
}

#[cfg(test)]
mod tests {
    use super::identifiant_sur;

    /// Le nom vient de l'utilisateur et devient un chemin de fichier.
    #[test]
    fn un_identifiant_ne_peut_pas_voyager() {
        assert_eq!(identifiant_sur("Mon Theme").as_deref(), Some("mon-theme"));
        assert_eq!(identifiant_sur("  Nuit  ").as_deref(), Some("nuit"));
        // Le point-point et les separateurs sont neutralises, pas rejetes: le nom
        // reste utilisable, il ne sort simplement plus du dossier.
        assert_eq!(identifiant_sur("../../memoire").as_deref(), Some("memoire"));
        assert_eq!(identifiant_sur("a/b\\c").as_deref(), Some("a-b-c"));
        // Rien d'exploitable: on refuse plutot que d'inventer un nom.
        assert!(identifiant_sur("").is_none());
        assert!(identifiant_sur("...").is_none());
        assert!(identifiant_sur(&"x".repeat(200)).is_none());
    }
}
