//! **User hooks** (Claude Code / third-party style): Gap D.
//!
//! The user defines shell commands in `hooks.json` to run AROUND tool calls:
//! `pre_tool` (before) and `post_tool` (after). A `pre_tool` hook that fails (exit
//! != 0) with `block: true` **blocks** the tool, useful for custom guardrails (linter,
//! validation, audit, refusing certain paths). Without touching the engine core.
//!
//! **Global** access (like [`crate::feed_journal`]/[`crate::secrets`]) to avoid threading the
//! config everywhere: the node loads `hooks.json` at boot ([`init`]), the engine calls
//! [`run_pre`]/[`run_post`] in the harvest loop. The tool name and its JSON arguments
//! are passed to the hook via the `LARUCHE_TOOL` and `LARUCHE_ARGS` environment variables.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// A user hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    /// `"pre_tool"` or `"post_tool"`.
    pub event: String,
    /// Simple glob on the tool name: `"*"` (all), `"shell_exec"`, or `"file_*"` prefix.
    #[serde(default = "etoile")]
    pub matcher: String,
    /// Shell command to run (receives `LARUCHE_TOOL` + `LARUCHE_ARGS` in env).
    pub command: String,
    /// If `true` and the `pre_tool` hook fails, the tool is BLOCKED.
    #[serde(default)]
    pub block: bool,
}

fn etoile() -> String {
    "*".to_string()
}

static HOOKS: OnceLock<Vec<Hook>> = OnceLock::new();

/// Initializes the hooks (called by the node at boot). Idempotent.
pub fn init(hooks: Vec<Hook>) {
    let _ = HOOKS.set(hooks);
}

fn correspond(matcher: &str, outil: &str) -> bool {
    if matcher == "*" || matcher == outil {
        return true;
    }
    // glob prefix "file_*"
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
    // Hard bound: a hook must not hang the loop.
    let fut = c.status();
    match tokio::time::timeout(std::time::Duration::from_secs(20), fut).await {
        Ok(Ok(st)) => Ok(st.success()),
        Ok(Err(e)) => Err(e),
        Err(_) => Ok(false), // timeout: treated as failure
    }
}

/// Runs the matching `pre_tool` hooks. Returns `Some(reason)` if a blocking hook
/// fails, in which case the tool must be refused. `None` means continue.
pub async fn run_pre(outil: &str, args: &serde_json::Value) -> Option<String> {
    for h in actifs("pre_tool", outil) {
        let ok = lancer(&h.command, outil, args).await.unwrap_or(false);
        if !ok && h.block {
            return Some(format!(
                "Blocked by a user pre_tool hook (command: {})",
                h.command
            ));
        }
    }
    None
}

/// Runs the matching `post_tool` hooks (best-effort, non-blocking).
pub async fn run_post(outil: &str, args: &serde_json::Value) {
    for h in actifs("post_tool", outil) {
        let _ = lancer(&h.command, outil, args).await;
    }
}

/// Is there at least one hook loaded?
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
