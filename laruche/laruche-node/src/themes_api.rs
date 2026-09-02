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

/// Taille maximale d'un logo, et d'une image de fond.
///
/// Un theme se partage en un fichier; il doit rester transportable. Ces deux
/// bornes sont aussi ce qui empeche un fichier de theme de faire grossir le
/// foyer sans limite.
const LOGO_MAX: usize = 512 * 1024;
const FOND_MAX: usize = 4 * 1024 * 1024;

/// Un SVG accepte, ou la raison du refus.
///
/// On REFUSE, on ne reecrit pas. Un SVG expurge a la main se casse en silence,
/// et l'utilisateur ne comprend pas pourquoi son logo s'affiche de travers; un
/// refus nomme le motif et lui laisse corriger son fichier.
///
/// Ce qui est refuse, et pourquoi. Un SVG est du code, et un fichier de theme se
/// partage: celui qui l'ouvre execute ce qu'il contient, dans la page, avec la
/// meme origine et les memes cookies que le reste de l'application.
/// - `on...=`: un gestionnaire d'evenement s'execute immediatement. `<svg
///   onload=...>` est le vecteur classique, et `<animate onbegin=...>` le meme
///   en moins connu.
/// - `<script>`: inerte quand il arrive par `innerHTML`, mais pas partout, et
///   surtout: aucun logo n'en a besoin.
/// - `<foreignObject>`: rouvre tout le HTML a l'interieur du SVG.
/// - `javascript:` et `data:text/html`: la meme execution par une URL.
/// - une reference EXTERNE (`href` qui ne commence pas par `#`) fait une requete
///   sortante a l'affichage, ce qui trahit la promesse hors ligne et signale le
///   lecteur a un tiers.
fn laver_svg(brut: &str) -> Result<String, &'static str> {
    let t = brut.trim();
    if t.len() > LOGO_MAX {
        return Err("logo trop lourd");
    }
    if !t.to_lowercase().contains("<svg") {
        return Err("ce n'est pas un SVG");
    }
    let bas = t.to_lowercase();
    for motif in ["<script", "</script", "<foreignobject", "javascript:", "data:text/html"] {
        if bas.contains(motif) {
            return Err("le SVG contient du code executable");
        }
    }
    // Un gestionnaire d'evenement: `on` colle a un blanc ou a un chevron, suivi de
    // lettres puis d'un `=`. Chercher le seul mot "on" attraperait `font-size`.
    let o: Vec<char> = bas.chars().collect();
    for i in 0..o.len().saturating_sub(3) {
        if o[i] != 'o' || o[i + 1] != 'n' {
            continue;
        }
        if i > 0 && !o[i - 1].is_whitespace() && o[i - 1] != '<' && o[i - 1] != '"' && o[i - 1] != '\'' {
            continue;
        }
        let mut j = i + 2;
        while j < o.len() && o[j].is_ascii_alphabetic() {
            j += 1;
        }
        if j == i + 2 {
            continue; // "on" tout seul, pas un attribut
        }
        let mut k = j;
        while k < o.len() && o[k].is_whitespace() {
            k += 1;
        }
        if k < o.len() && o[k] == '=' {
            return Err("le SVG contient un gestionnaire d'evenement");
        }
    }
    // Reference sortante: tout `href` dont la valeur ne commence pas par `#`.
    let mut reste = bas.as_str();
    while let Some(p) = reste.find("href") {
        let apres = &reste[p + 4..];
        let apres = apres.trim_start_matches([' ', '\t', '\n', '\r']);
        if let Some(v) = apres.strip_prefix('=') {
            let v = v.trim_start().trim_start_matches(['"', '\'']);
            if !v.starts_with('#') {
                return Err("le SVG pointe vers une ressource externe");
            }
        }
        reste = &reste[p + 4..];
    }
    Ok(t.to_string())
}

/// Le logo tel qu'il sera stocke: un SVG lave, ou une image matricielle encodee.
fn logo_sur(brut: &str) -> Result<String, &'static str> {
    let t = brut.trim();
    if t.is_empty() {
        return Ok(String::new());
    }
    if t.starts_with('<') {
        return laver_svg(t);
    }
    if t.starts_with("data:image/") {
        if t.len() > LOGO_MAX {
            return Err("logo trop lourd");
        }
        // `data:image/svg+xml` rentrerait par cette porte sans passer par le
        // laveur, encode et donc illisible pour lui. On l'exclut explicitement.
        if t.to_lowercase().starts_with("data:image/svg") {
            return Err("un SVG doit etre fourni en texte, pas encode");
        }
        return Ok(t.to_string());
    }
    Err("format de logo non reconnu")
}

/// L'habillage: la marque et le fond, valides avant d'etre ecrits sur le disque.
///
/// La validation est ici et non dans le navigateur, parce que c'est ici qu'est la
/// frontiere: un fichier de theme peut arriver d'ailleurs, ou etre modifie a la
/// main dans le foyer.
/// Les icones de remplacement, lavees une par une comme le logo.
///
/// Meme raison, meme severite: chacune est un SVG, donc du code, et un fichier de
/// theme se partage. Une seule icone refusee fait refuser l'enregistrement
/// entier, avec son nom: accepter les onze autres en taisant la douzieme
/// donnerait un theme dont on croirait qu'il est complet.
fn icones_sur(v: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut out = serde_json::Map::new();
    if let Some(obj) = v.as_object() {
        for (cle, val) in obj {
            let brut = val.as_str().unwrap_or("").trim();
            if brut.is_empty() {
                continue;
            }
            match logo_sur(brut) {
                Ok(propre) => {
                    out.insert(cle.clone(), serde_json::Value::String(propre));
                }
                Err(e) => return Err(format!("icone {cle}: {e}")),
            }
        }
    }
    Ok(serde_json::Value::Object(out))
}

/// Valide l'habillage, en gardant de l'ANCIEN ce que le nouveau ne mentionne pas.
///
/// Un theme porte son image de fond encodee dans son JSON, souvent plusieurs
/// centaines de kilooctets. L'enregistrement automatique la renvoyait en entier a
/// chaque frappe, ce qui rendait chaque sauvegarde longue pour ne changer, la
/// plupart du temps, qu'un chiffre de couleur.
///
/// La regle distingue donc trois cas, et c'est la distinction qui compte:
/// - champ ABSENT: on garde ce qui est sur le disque, rien n'a change;
/// - champ VIDE: on efface, l'utilisateur a retire l'image;
/// - champ RENSEIGNE: on valide et on remplace.
/// Les polices importees, validees avant d'entrer dans un fichier de theme.
///
/// Elles voyagent encodees dans le theme, donc dans un fichier qui se partage. Le
/// nom devient une famille CSS ecrite dans une feuille de style: on n'y laisse
/// donc que des caracteres ordinaires, sinon un nom bien choisi fermerait la
/// declaration et ecrirait la suite a sa guise.
fn polices_sur(v: &serde_json::Value) -> Result<serde_json::Value, String> {
    const POLICE_MAX: usize = 2 * 1024 * 1024;
    const NOMBRE_MAX: usize = 8;
    let mut out: Vec<serde_json::Value> = Vec::new();
    for p in v.as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
        if out.len() >= NOMBRE_MAX {
            return Err("trop de polices importees (8 au maximum)".into());
        }
        let nom: String = p["nom"]
            .as_str()
            .unwrap_or("")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
            .take(48)
            .collect();
        let nom = nom.trim().to_string();
        let data = p["data"].as_str().unwrap_or("").trim();
        if nom.is_empty() || data.is_empty() {
            continue;
        }
        if data.len() > POLICE_MAX {
            return Err(format!("police {nom}: fichier trop lourd"));
        }
        // Une police, et rien d'autre. `data:` accepte n'importe quel type, et une
        // feuille de style qui charge autre chose n'a aucune raison d'exister ici.
        let bas = data.to_lowercase();
        let ok = bas.starts_with("data:font/")
            || bas.starts_with("data:application/font")
            || bas.starts_with("data:application/x-font")
            || bas.starts_with("data:application/octet-stream");
        if !ok {
            return Err(format!("police {nom}: ce n'est pas un fichier de police"));
        }
        out.push(serde_json::json!({ "nom": nom, "data": data }));
    }
    Ok(serde_json::Value::Array(out))
}

fn habillage_sur(
    body: &serde_json::Value,
    ancien: Option<&serde_json::Value>,
) -> Result<
    (
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
    ),
    String,
> {
    let m = &body["marque"];
    let nom: String = m["nom"].as_str().unwrap_or("").trim().chars().take(60).collect();
    let logo = match m.get("logo") {
        Some(v) => logo_sur(v.as_str().unwrap_or("")).map_err(|e| e.to_string())?,
        None => ancien
            .and_then(|a| a["marque"]["logo"].as_str())
            .unwrap_or("")
            .to_string(),
    };
    // Zero vaut « pas de taille choisie »: le logo reprend alors celle de la
    // ruche qu'il remplace, et la barre ne bouge pas.
    let taille = m["taille"].as_i64().unwrap_or(0);
    let taille = if taille == 0 { 0 } else { taille.clamp(18, 96) };
    let masquer = m["masquerNom"].as_bool().unwrap_or(false);
    let marque =
        serde_json::json!({ "nom": nom, "logo": logo, "taille": taille, "masquerNom": masquer });

    let f = &body["fond"];
    let image = match f.get("image") {
        Some(v) => v.as_str().unwrap_or("").trim().to_string(),
        None => ancien
            .and_then(|a| a["fond"]["image"].as_str())
            .unwrap_or("")
            .to_string(),
    };
    if !image.is_empty() {
        if !image.starts_with("data:image/") {
            return Err("l'image de fond doit etre encodee".into());
        }
        if image.len() > FOND_MAX {
            return Err("image de fond trop lourde".into());
        }
    }
    let opacite = f["opacite"].as_f64().unwrap_or(0.35).clamp(0.0, 1.0);
    let cadrage = match f["cadrage"].as_str().unwrap_or("cover") {
        c @ ("cover" | "contain" | "auto" | "100% 100%") => c,
        _ => "cover",
    };
    let mut zones = serde_json::Map::new();
    for z in ["app", "gauche", "droite", "haut", "bas"] {
        if f["zones"][z].as_bool().unwrap_or(false) {
            zones.insert(z.to_string(), serde_json::Value::Bool(true));
        }
    }
    let fond = serde_json::json!({
        "image": image, "opacite": opacite, "cadrage": cadrage, "zones": zones
    });
    let icones = match body.get("icones") {
        Some(v) => icones_sur(v)?,
        None => ancien
            .and_then(|a| a.get("icones").cloned())
            .unwrap_or_else(|| serde_json::Value::Object(Default::default())),
    };
    let polices = match body.get("polices") {
        Some(v) => polices_sur(v)?,
        None => ancien
            .and_then(|a| a.get("polices").cloned())
            .unwrap_or_else(|| serde_json::Value::Array(Vec::new())),
    };
    // Les tailles d'icones: un nombre de pixels par emplacement, borne. Une valeur
    // absente vaut « taille d'origine », c'est ce qui distingue un curseur au repos
    // d'un curseur pose sur sa valeur par defaut.
    let mut tailles = serde_json::Map::new();
    if let Some(obj) = body.get("taillesIcones").and_then(|v| v.as_object()) {
        for (cle, val) in obj {
            if let Some(px) = val.as_i64() {
                if px > 0 {
                    tailles.insert(cle.clone(), serde_json::Value::from(px.clamp(8, 96)));
                }
            }
        }
    } else if let Some(a) = ancien.and_then(|a| a.get("taillesIcones")).and_then(|v| v.as_object()) {
        tailles = a.clone();
    }
    Ok((marque, fond, icones, polices, serde_json::Value::Object(tailles)))
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
    let ancien = std::fs::read_to_string(dossier().join(format!("{id}.json")))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
    let (marque, fond, icones, polices, tailles_icones) = match habillage_sur(&body, ancien.as_ref()) {
        Ok(v) => v,
        Err(e) => return Json(serde_json::json!({ "status": "error", "error": e })),
    };
    // `parent` et `base` servent au retour a la valeur d'origine, jeton par jeton.
    // La base est capturee a la creation de la copie: la recalculer plus tard
    // obligerait a repeindre le theme parent pour le lire, donc a faire clignoter
    // l'interface. Elles ne sont ecrites qu'a la creation, jamais ecrasees ensuite,
    // sinon le premier enregistrement d'une modification effacerait la reference
    // meme a laquelle on veut pouvoir revenir.
    let parent = ancien
        .as_ref()
        .map(|a| a["parent"].clone())
        .filter(|v| !v.is_null())
        .unwrap_or_else(|| body["parent"].clone());
    let base = ancien
        .as_ref()
        .map(|a| a["base"].clone())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| body["base"].clone());
    let theme = serde_json::json!({
        "id": id, "nom": nom, "jetons": jetons, "marque": marque, "fond": fond,
        "icones": icones, "polices": polices, "taillesIcones": tailles_icones,
        "parent": parent, "base": base
    });
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

#[cfg(test)]
mod tests_laveur_svg {
    use super::{laver_svg, logo_sur};

    const PROPRE: &str = "<svg viewBox=\"0 0 24 24\"><path d=\"M2 2h20v20H2z\" fill=\"currentColor\"/></svg>";

    #[test]
    fn un_logo_honnete_passe() {
        assert!(laver_svg(PROPRE).is_ok());
        // Une reference INTERNE est legitime: un degrade se reference ainsi.
        assert!(laver_svg("<svg><use href=\"#a\"/><rect fill=\"url(#a)\"/></svg>").is_ok());
    }

    #[test]
    fn les_gestionnaires_d_evenement_sont_refuses() {
        assert!(laver_svg("<svg onload=\"alert(1)\"><path/></svg>").is_err());
        // Le meme, en moins connu, et avec des blancs autour du signe.
        assert!(laver_svg("<svg><animate onbegin = 'x()'/></svg>").is_err());
        assert!(laver_svg("<svg><a onclick=\"x()\"/></svg>").is_err());
    }

    #[test]
    fn font_size_n_est_pas_un_gestionnaire() {
        // Le piege du filtre naif: `on` apparait dans beaucoup de mots.
        assert!(laver_svg("<svg font-size=\"12\"><text>bonjour</text></svg>").is_ok());
        assert!(laver_svg("<svg><path id=\"lion\" d=\"M0 0\"/></svg>").is_ok());
    }

    #[test]
    fn le_code_et_les_ressources_externes_sont_refuses() {
        assert!(laver_svg("<svg><script>alert(1)</script></svg>").is_err());
        assert!(laver_svg("<svg><foreignObject><body/></foreignObject></svg>").is_err());
        assert!(laver_svg("<svg><a href=\"javascript:alert(1)\"/></svg>").is_err());
        assert!(laver_svg("<svg><image href=\"https://ailleurs/x.png\"/></svg>").is_err());
    }

    #[test]
    fn ce_qui_n_est_pas_un_svg_est_refuse() {
        assert!(laver_svg("bonjour").is_err());
        assert!(laver_svg("<html><body/></html>").is_err());
    }

    #[test]
    fn le_logo_accepte_le_texte_svg_et_les_images_encodees() {
        assert!(logo_sur("").unwrap().is_empty());
        assert!(logo_sur(PROPRE).is_ok());
        assert!(logo_sur("data:image/png;base64,AAAA").is_ok());
        // Un SVG encode contournerait le laveur: il doit venir en texte.
        assert!(logo_sur("data:image/svg+xml;base64,AAAA").is_err());
        assert!(logo_sur("https://ailleurs/logo.svg").is_err());
    }
}
