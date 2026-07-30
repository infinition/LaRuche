//! Quels outils chaque mission ouvre, et jusqu'ou.
//!
//! Une liste BLANCHE, jamais une liste noire. Une ruche compte pres de cent outils, et
//! parmi eux de quoi supprimer une memoire, poster sur Telegram ou modifier un cron:
//! partir de « tout sauf » garantit qu'un outil ajoute demain sera ouvert par defaut a
//! sept agents. Partir de « rien sauf » garantit l'inverse.
//!
//! Le decoupage suit une gradation qu'on peut expliquer en une phrase:
//!
//! | Mission | Ce qu'ils peuvent faire |
//! |---|---|
//! | Reponse | lire le web et la memoire |
//! | Recherche | + fouiller en profondeur et lire des fichiers |
//! | Code | + ecrire des fichiers |
//! | Experimentation | + executer du code |
//!
//! Les deux dernieres touchent la machine: l'interface doit demander confirmation
//! AVANT de lancer, pas apres. Un debat qui ecrit des fichiers n'est plus une
//! conversation.

use super::moteur::Mission;

/// Lire le monde, sans rien y changer. Socle commun a toutes les missions.
const LECTURE: &[&str] = &[
    "web_search",
    "web_fetch",
    "read_extract",
    "memory_search",
    "memory_read_node",
    "memory_tree",
    "math_eval",
];

/// Fouiller: recherche profonde et lecture de fichiers locaux.
const FOUILLE: &[&str] = &["web_deep_search", "image_search", "file_read", "file_list", "file_search"];

/// Ecrire sur le disque. A partir d'ici, la table touche la machine.
const ECRITURE: &[&str] = &["file_write", "file_edit", "git_status", "git_diff"];

/// Executer. Le niveau le plus engageant.
const EXECUTION: &[&str] = &["execute_code", "run_script", "shell_exec"];

/// Les outils ouverts pour cette mission.
pub fn permis(mission: Mission) -> Vec<String> {
    let mut v: Vec<&str> = LECTURE.to_vec();
    match mission {
        Mission::Reponse => {}
        Mission::Recherche => v.extend_from_slice(FOUILLE),
        Mission::Code => {
            v.extend_from_slice(FOUILLE);
            v.extend_from_slice(ECRITURE);
        }
        Mission::Experimentation => {
            v.extend_from_slice(FOUILLE);
            v.extend_from_slice(ECRITURE);
            v.extend_from_slice(EXECUTION);
        }
    }
    v.into_iter().map(String::from).collect()
}

/// Cette mission touche-t-elle la machine ?
///
/// Sert a exiger une confirmation avant de lancer. Lire le web n'engage rien;
/// ecrire des fichiers ou lancer du code, si - et a sept agents.
pub fn touche_la_machine(mission: Mission) -> bool {
    matches!(mission, Mission::Code | Mission::Experimentation)
}

/// Nombre d'appels d'outils autorises par intervention.
///
/// Borne basse volontairement: un specialiste qui enchaine quinze recherches n'a pas
/// compris sa question, et chaque appel s'ajoute au contexte de TOUS les tours
/// suivants. Trois suffisent a verifier un fait ou lire un fichier.
pub const OUTILS_PAR_TOUR: usize = 3;

/// La consigne d'outillage ajoutee au prompt du specialiste.
///
/// Le protocole est celui de LaRuche (`<tool_call>`), et non les outils natifs du
/// fournisseur: il marche avec tous les modeles, y compris ceux qui n'ont pas d'API
/// d'outils - et la diversite des modeles est le coeur du dispositif.
pub fn consigne_outils(noms: &[String], schemas: &serde_json::Value) -> String {
    if noms.is_empty() {
        return String::new();
    }
    let liste = schemas
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|t| {
                    t.get("name")
                        .and_then(|n| n.as_str())
                        .is_some_and(|n| noms.iter().any(|p| p == n))
                })
                .map(|t| {
                    format!(
                        "- `{}`: {}\n  arguments: {}",
                        t["name"].as_str().unwrap_or(""),
                        t["description"].as_str().unwrap_or(""),
                        t["parameters"]
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    format!(
        "\n\n# Outils\n\n\
         Tu peux verifier au lieu de supposer. Pour appeler un outil, ecris EXACTEMENT:\n\n\
         <tool_call>{{\"name\": \"...\", \"arguments\": {{...}}}}</tool_call>\n\n\
         Le resultat te sera donne et tu pourras continuer. {OUTILS_PAR_TOUR} appels au \
         maximum par intervention.\n\n\
         N'appelle un outil que si la reponse en depend. La constitution t'interdit \
         d'inventer: un fait verifiable doit etre verifie, mais chercher pour le plaisir \
         de chercher coute du temps a tout le monde.\n\n\
         Quand tu as fini d'utiliser les outils, rends ta reponse au format demande.\n\n\
         Outils disponibles:\n{liste}\n"
    )
}

/// Extrait les appels d'outils d'une reponse de modele.
///
/// Tolerant: un modele qui oublie la fermeture, ecrit du JSON approximatif ou entoure
/// le bloc de texte reste exploitable. Un appel illisible est ignore plutot que de
/// faire echouer l'intervention entiere.
pub fn extraire_appels(texte: &str) -> Vec<(String, serde_json::Value)> {
    let mut out = Vec::new();
    let mut reste = texte;
    while let Some(i) = reste.find("<tool_call>") {
        let apres = &reste[i + "<tool_call>".len()..];
        let (corps, suite) = match apres.find("</tool_call>") {
            Some(j) => (&apres[..j], &apres[j + "</tool_call>".len()..]),
            // Balise ouvrante sans fermeture: on prend jusqu'a la fin. Une reponse
            // tronquee en plein appel reste souvent lisible.
            None => (apres, ""),
        };
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(corps.trim()) {
            if let Some(nom) = v.get("name").and_then(|n| n.as_str()) {
                let args = v
                    .get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                out.push((nom.to_string(), args));
            }
        }
        if suite.is_empty() {
            break;
        }
        reste = suite;
    }
    out
}

/// Retire les blocs d'appel du texte rendu a l'utilisateur.
pub fn nettoyer(texte: &str) -> String {
    let mut out = texte.to_string();
    while let Some(i) = out.find("<tool_call>") {
        match out[i..].find("</tool_call>") {
            Some(j) => {
                let fin = i + j + "</tool_call>".len();
                out.replace_range(i..fin, "");
            }
            None => {
                out.truncate(i);
                break;
            }
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_gradation_est_croissante() {
        let r = permis(Mission::Reponse);
        let rech = permis(Mission::Recherche);
        let code = permis(Mission::Code);
        let exp = permis(Mission::Experimentation);
        // Chaque niveau contient le precedent: on ne retire jamais un droit en
        // montant, sinon la promesse faite a l'utilisateur devient illisible.
        for o in &r {
            assert!(rech.contains(o), "{o} perdu en Recherche");
        }
        for o in &rech {
            assert!(code.contains(o), "{o} perdu en Code");
        }
        for o in &code {
            assert!(exp.contains(o), "{o} perdu en Experimentation");
        }
        assert!(r.len() < rech.len() && rech.len() < code.len() && code.len() < exp.len());
    }

    #[test]
    fn repondre_n_ecrit_rien() {
        let r = permis(Mission::Reponse);
        for interdit in ["file_write", "file_edit", "shell_exec", "execute_code", "run_script"] {
            assert!(!r.contains(&interdit.to_string()), "{interdit} ouvert en lecture seule");
        }
    }

    #[test]
    fn aucune_mission_n_ouvre_les_outils_destructeurs() {
        // La liste blanche doit rester une liste blanche: ces outils existent dans la
        // ruche et ne doivent jamais tomber dans une deliberation par inadvertance.
        let dangereux = [
            "memory_delete", "memory_delete_node", "cron_delete", "mission_delete",
            "skill_delete", "plugin_delete", "watcher_delete", "send_telegram",
            "mesh_send", "delegate", "spawn_specialist",
        ];
        for m in [Mission::Reponse, Mission::Recherche, Mission::Code, Mission::Experimentation] {
            let p = permis(m);
            for d in dangereux {
                assert!(!p.contains(&d.to_string()), "{d} ouvert en {m:?}");
            }
        }
    }

    #[test]
    fn seules_les_missions_engageantes_touchent_la_machine() {
        assert!(!touche_la_machine(Mission::Reponse));
        assert!(!touche_la_machine(Mission::Recherche));
        assert!(touche_la_machine(Mission::Code));
        assert!(touche_la_machine(Mission::Experimentation));
    }

    #[test]
    fn lit_un_appel_simple() {
        let t = "Je verifie.\n<tool_call>{\"name\": \"web_search\", \"arguments\": {\"query\": \"rust\"}}</tool_call>";
        let a = extraire_appels(t);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].0, "web_search");
        assert_eq!(a[0].1["query"], "rust");
    }

    #[test]
    fn lit_plusieurs_appels() {
        let t = "<tool_call>{\"name\":\"a\",\"arguments\":{}}</tool_call> texte \
                 <tool_call>{\"name\":\"b\",\"arguments\":{}}</tool_call>";
        assert_eq!(extraire_appels(t).len(), 2);
    }

    #[test]
    fn un_appel_tronque_reste_lisible() {
        let t = "<tool_call>{\"name\":\"web_search\",\"arguments\":{\"query\":\"x\"}}";
        let a = extraire_appels(t);
        assert_eq!(a.len(), 1, "une balise sans fermeture doit rester exploitable");
    }

    #[test]
    fn un_appel_illisible_est_ignore_sans_tout_perdre() {
        let t = "<tool_call>ceci n'est pas du json</tool_call>\
                 <tool_call>{\"name\":\"ok\",\"arguments\":{}}</tool_call>";
        let a = extraire_appels(t);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].0, "ok");
    }

    #[test]
    fn le_nettoyage_retire_les_blocs() {
        let t = "Avant <tool_call>{\"name\":\"x\",\"arguments\":{}}</tool_call> apres";
        let n = nettoyer(t);
        assert!(!n.contains("tool_call"));
        assert!(n.contains("Avant") && n.contains("apres"));
    }

    #[test]
    fn sans_outil_la_consigne_est_vide() {
        assert!(consigne_outils(&[], &serde_json::json!([])).is_empty());
    }

    #[test]
    fn la_consigne_ne_liste_que_les_outils_permis() {
        let schemas = serde_json::json!([
            {"name":"web_search","description":"cherche","parameters":{}},
            {"name":"shell_exec","description":"execute","parameters":{}}
        ]);
        let c = consigne_outils(&["web_search".to_string()], &schemas);
        assert!(c.contains("web_search"));
        assert!(!c.contains("shell_exec"), "un outil non permis ne doit pas etre annonce");
    }
}
