//! The **secrets vault** (runtime view, swarm side).
//!
//! Principle: the user registers `NAME -> value` pairs (API keys, tokens, webhook
//! URLs). **The LLM NEVER sees the values**, only the NAMES, injected into the prompt.
//! When a tool/shell/script contains `${NAME}`, the node **substitutes** the real value just
//! before execution. This lets LaRuche use a token without ever knowing it.
//!
//! This module is the **in-memory view** (never serialized, never logged) accessible from
//! the tools (swarm) AND the node. Encryption at rest and the endpoints live on the node side.
//! Global access (like [`crate::feed_journal`]) to avoid threading the vault everywhere.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

static COFFRE: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();

fn coffre() -> &'static RwLock<HashMap<String, String>> {
    COFFRE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Replaces the entire secrets table (called by the node at boot after decryption).
pub fn init(map: HashMap<String, String>) {
    if let Ok(mut c) = coffre().write() {
        *c = map;
    }
}

/// Adds/updates a secret in memory (encrypted persistence is handled by the node).
pub fn definir(nom: impl Into<String>, valeur: impl Into<String>) {
    if let Ok(mut c) = coffre().write() {
        c.insert(nom.into(), valeur.into());
    }
}

/// Removes a secret from memory.
pub fn retirer(nom: &str) {
    if let Ok(mut c) = coffre().write() {
        c.remove(nom);
    }
}

/// List of secret **NAMES** (never the values), for the system prompt and the UI.
pub fn noms() -> Vec<String> {
    let Ok(c) = coffre().read() else { return Vec::new() };
    let mut v: Vec<String> = c.keys().cloned().collect();
    v.sort();
    v
}

/// Reports whether at least one secret is defined.
pub fn non_vide() -> bool {
    coffre().read().map(|c| !c.is_empty()).unwrap_or(false)
}

/// **Substitution**: replaces every occurrence of `${NAME}`, `{{NAME}}` AND `@@NAME` with the
/// secret's real value. Unknown references are left as-is. This is where the value "enters"
/// the command, without ever passing through the LLM context.
///
/// `@@NAME` is the ergonomic form typed in the chat/forms ("send via @@webhook").
/// Names are processed from longest to shortest so that a name which is a prefix of another
/// (`@@web` vs `@@webhook`) is not substituted first.
pub fn substituer(texte: &str) -> String {
    if !texte.contains("${") && !texte.contains("{{") && !texte.contains("@@") {
        return texte.to_string();
    }
    let Ok(c) = coffre().read() else { return texte.to_string() };
    let mut paires: Vec<(&String, &String)> = c.iter().collect();
    paires.sort_by(|a, b| b.0.len().cmp(&a.0.len())); // longest first
    let mut out = texte.to_string();
    for (nom, val) in paires {
        out = out.replace(&format!("${{{nom}}}"), val);
        out = out.replace(&format!("{{{{{nom}}}}}"), val);
        out = out.replace(&format!("@@{nom}"), val);
    }
    out
}

/// **Masking** (the return trip): replaces every occurrence of a vault VALUE with
/// `[SECRET:NAME]`. Applied to tool observations before they reach the LLM context
/// and the persisted session, so a command that echoes a token (`env`, a verbose
/// curl, a config dump) no longer leaks it. Deterministic exact match, longest
/// value first; values shorter than 6 chars are skipped (collision-prone).
pub fn masquer(texte: &str) -> String {
    let Ok(c) = coffre().read() else {
        return texte.to_string();
    };
    if c.is_empty() {
        return texte.to_string();
    }
    let mut paires: Vec<(&String, &String)> = c.iter().filter(|(_, v)| v.len() >= 6).collect();
    paires.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    let mut out = texte.to_string();
    for (nom, val) in paires {
        if out.contains(val.as_str()) {
            out = out.replace(val.as_str(), &format!("[SECRET:{nom}]"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masque_les_valeurs_du_coffre_dans_les_sorties() {
        // definir() (not init) so the parallel test's map is not wiped.
        definir("MASK_TOKEN", "sk-abcdef123456");
        definir("MASK_COURT", "abc"); // < 6 chars: never masked (collisions)
        let sortie = "header Authorization: Bearer sk-abcdef123456 fin abc";
        assert_eq!(
            masquer(sortie),
            "header Authorization: Bearer [SECRET:MASK_TOKEN] fin abc"
        );
        // No secret in the text: untouched.
        assert_eq!(masquer("rien ici"), "rien ici");
    }

    #[test]
    fn substitue_les_references_connues_garde_les_autres() {
        let mut m = HashMap::new();
        m.insert("TOKEN_X".to_string(), "secret123".to_string());
        init(m);
        assert_eq!(substituer("curl -H ${TOKEN_X}"), "curl -H secret123");
        assert_eq!(substituer("voir {{TOKEN_X}}"), "voir secret123");
        assert_eq!(substituer("post @@TOKEN_X"), "post secret123");
        // unknown reference left as-is
        assert_eq!(substituer("${INCONNU}"), "${INCONNU}");
        assert_eq!(substituer("@@INCONNU"), "@@INCONNU");
        // names are exposed, not the values
        assert!(noms().contains(&"TOKEN_X".to_string()));
    }
}
