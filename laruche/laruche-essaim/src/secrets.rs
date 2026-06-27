//! Le **coffre à secrets** (vue runtime, côté essaim).
//!
//! Principe : l'utilisateur enregistre des `NOM → valeur` (clés d'API, tokens, URLs de
//! webhook…). **Le LLM ne voit JAMAIS les valeurs** — seulement les NOMS, injectés au prompt.
//! Quand un outil/shell/script contient `${NOM}`, le node **substitue** la vraie valeur juste
//! avant l'exécution. Ainsi LaRuche peut utiliser un token sans jamais le connaître.
//!
//! Ce module est la **vue en mémoire** (jamais sérialisée, jamais loggée) accessible depuis
//! les outils (essaim) ET le node. Le chiffrement au repos + les endpoints sont côté node.
//! Accès global (comme [`crate::feed_journal`]) pour ne pas threader le coffre partout.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

static COFFRE: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();

fn coffre() -> &'static RwLock<HashMap<String, String>> {
    COFFRE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Remplace la table complète des secrets (appelé par le node au boot après déchiffrement).
pub fn init(map: HashMap<String, String>) {
    if let Ok(mut c) = coffre().write() {
        *c = map;
    }
}

/// Ajoute/met à jour un secret en mémoire (la persistance chiffrée est gérée par le node).
pub fn definir(nom: impl Into<String>, valeur: impl Into<String>) {
    if let Ok(mut c) = coffre().write() {
        c.insert(nom.into(), valeur.into());
    }
}

/// Retire un secret en mémoire.
pub fn retirer(nom: &str) {
    if let Ok(mut c) = coffre().write() {
        c.remove(nom);
    }
}

/// Liste des **NOMS** de secrets (jamais les valeurs) — pour le prompt système et l'UI.
pub fn noms() -> Vec<String> {
    let Ok(c) = coffre().read() else { return Vec::new() };
    let mut v: Vec<String> = c.keys().cloned().collect();
    v.sort();
    v
}

/// Indique si au moins un secret est défini.
pub fn non_vide() -> bool {
    coffre().read().map(|c| !c.is_empty()).unwrap_or(false)
}

/// **Substitution** : remplace toutes les occurrences de `${NOM}`, `{{NOM}}` ET `@@NOM` par la
/// valeur réelle du secret. Les références inconnues sont laissées telles quelles. C'est ici que
/// la valeur « entre » dans la commande, sans jamais transiter par le contexte du LLM.
///
/// `@@NOM` est la forme ergonomique tapée dans le chat/les formulaires (« envoie via @@webhook »).
/// Les noms sont traités du plus long au plus court pour éviter qu'un nom préfixe d'un autre
/// (`@@web` vs `@@webhook`) ne soit substitué en premier.
pub fn substituer(texte: &str) -> String {
    if !texte.contains("${") && !texte.contains("{{") && !texte.contains("@@") {
        return texte.to_string();
    }
    let Ok(c) = coffre().read() else { return texte.to_string() };
    let mut paires: Vec<(&String, &String)> = c.iter().collect();
    paires.sort_by(|a, b| b.0.len().cmp(&a.0.len())); // plus long d'abord
    let mut out = texte.to_string();
    for (nom, val) in paires {
        out = out.replace(&format!("${{{nom}}}"), val);
        out = out.replace(&format!("{{{{{nom}}}}}"), val);
        out = out.replace(&format!("@@{nom}"), val);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitue_les_references_connues_garde_les_autres() {
        let mut m = HashMap::new();
        m.insert("TOKEN_X".to_string(), "secret123".to_string());
        init(m);
        assert_eq!(substituer("curl -H ${TOKEN_X}"), "curl -H secret123");
        assert_eq!(substituer("voir {{TOKEN_X}}"), "voir secret123");
        assert_eq!(substituer("post @@TOKEN_X"), "post secret123");
        // référence inconnue laissée telle quelle
        assert_eq!(substituer("${INCONNU}"), "${INCONNU}");
        assert_eq!(substituer("@@INCONNU"), "@@INCONNU");
        // les noms sont exposés, pas les valeurs
        assert!(noms().contains(&"TOKEN_X".to_string()));
    }
}
