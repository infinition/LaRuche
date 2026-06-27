//! **Hooks utilisateur** (façon Claude Code / third-party) — Gap D.
//!
//! L'utilisateur définit dans `hooks.json` des commandes shell à exécuter AUTOUR des appels
//! d'outils : `pre_tool` (avant) et `post_tool` (après). Un hook `pre_tool` qui échoue (exit
//! ≠ 0) avec `block: true` **bloque** l'outil — utile pour des garde-fous custom (linter,
//! validation, audit, refus de certains chemins…). Sans toucher au cœur du moteur.
//!
//! Accès **global** (comme [`crate::feed_journal`]/[`crate::secrets`]) pour ne pas threader la
//! config partout : le node charge `hooks.json` au boot ([`init`]), le moteur appelle
//! [`run_pre`]/[`run_post`] dans la boucle de récolte. Le nom de l'outil et ses arguments JSON
//! sont passés au hook via les variables d'environnement `LARUCHE_TOOL` et `LARUCHE_ARGS`.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Un hook utilisateur.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    /// `"pre_tool"` ou `"post_tool"`.
    pub event: String,
    /// Glob simple sur le nom d'outil : `"*"` (tous), `"shell_exec"`, ou préfixe `"file_*"`.
    #[serde(default = "etoile")]
    pub matcher: String,
    /// Commande shell à exécuter (reçoit `LARUCHE_TOOL` + `LARUCHE_ARGS` en env).
    pub command: String,
    /// Si `true` et que le hook `pre_tool` échoue → l'outil est BLOQUÉ.
    #[serde(default)]
    pub block: bool,
}

fn etoile() -> String {
    "*".to_string()
}

static HOOKS: OnceLock<Vec<Hook>> = OnceLock::new();

/// Initialise les hooks (appelé par le node au boot). Idempotent.
pub fn init(hooks: Vec<Hook>) {
    let _ = HOOKS.set(hooks);
}

fn correspond(matcher: &str, outil: &str) -> bool {
    if matcher == "*" || matcher == outil {
        return true;
    }
    // préfixe glob « file_* »
    matcher
        .strip_suffix('*')
        .map(|p| outil.starts_with(p))
        .unwrap_or(false)
}

fn actifs(event: &str, outil: &str) -> Vec<Hook> {
    let Some(hooks) = HOOKS.get() else {
        return Vec::new();
    };
    hooks
        .iter()
        .filter(|h| h.event == event && correspond(&h.matcher, outil))
        .cloned()
        .collect()
}

async fn lancer(cmd: &str, outil: &str, args: &serde_json::Value) -> std::io::Result<bool> {
    use tokio::process::Command;
    let args_str = args.to_string();
    let mut c = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", cmd]);
        c
    } else {
        let mut c = Command::new("sh");
        c.arg("-c").arg(cmd);
        c
    };
    c.env("LARUCHE_TOOL", outil).env("LARUCHE_ARGS", args_str);
    // Borne dure : un hook ne doit pas pendre la boucle.
    let fut = c.status();
    match tokio::time::timeout(std::time::Duration::from_secs(20), fut).await {
        Ok(Ok(st)) => Ok(st.success()),
        Ok(Err(e)) => Err(e),
        Err(_) => Ok(false), // timeout → considéré échec
    }
}

/// Exécute les hooks `pre_tool` correspondants. Renvoie `Some(raison)` si un hook bloquant
/// échoue → l'outil doit être refusé. `None` = on continue.
pub async fn run_pre(outil: &str, args: &serde_json::Value) -> Option<String> {
    for h in actifs("pre_tool", outil) {
        let ok = lancer(&h.command, outil, args).await.unwrap_or(false);
        if !ok && h.block {
            return Some(format!(
                "Bloqué par un hook pre_tool de l'utilisateur (commande : {})",
                h.command
            ));
        }
    }
    None
}

/// Exécute les hooks `post_tool` correspondants (best-effort, non bloquant).
pub async fn run_post(outil: &str, args: &serde_json::Value) {
    for h in actifs("post_tool", outil) {
        let _ = lancer(&h.command, outil, args).await;
    }
}

/// Y a-t-il au moins un hook chargé ?
pub fn non_vide() -> bool {
    HOOKS.get().map(|h| !h.is_empty()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matcher_glob() {
        assert!(correspond("*", "shell_exec"));
        assert!(correspond("shell_exec", "shell_exec"));
        assert!(correspond("file_*", "file_write"));
        assert!(!correspond("file_*", "shell_exec"));
        assert!(!correspond("web_search", "shell_exec"));
    }
}
