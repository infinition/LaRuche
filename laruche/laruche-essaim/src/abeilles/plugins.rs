//! Dynamic plugin system for Abeilles.
//!
//! Plugins are JSON files in a `plugins/` directory. Each file defines a tool
//! that executes a shell command template with arguments from the LLM.
//!
//! **Arguments passing** : les arguments sont injectés dans la commande via
//! `{{param}}` placeholder, MAIS les arguments longs et multi-lignes (`message`,
//! `text`, `content`, `code`) sont automatiquement passés via **stdin** pour
//! éviter les problèmes de quoting shell (surtout sur Windows cmd.exe).

use crate::abeille::{
    Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille, ToolOrigin,
};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub command: String,
    #[serde(default = "default_danger")]
    pub danger: String,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

fn default_danger() -> String {
    "safe".to_string()
}

/// Champs d'argument longs → passés via stdin au lieu du shell.
const STDIN_ARGS: &[&str] = &["message", "text", "content", "code", "body"];

pub struct PluginAbeille {
    def: PluginDefinition,
}

impl PluginAbeille {
    pub fn new(def: PluginDefinition) -> Self {
        Self { def }
    }
}

#[async_trait]
impl Abeille for PluginAbeille {
    fn nom(&self) -> &str {
        &self.def.name
    }
    fn description(&self) -> &str {
        &self.def.description
    }
    fn schema(&self) -> serde_json::Value {
        self.def.parameters.clone()
    }

    fn niveau_danger(&self) -> NiveauDanger {
        match self.def.danger.as_str() {
            "needs_approval" => NiveauDanger::NeedsApproval,
            "dangerous" => NiveauDanger::Dangerous,
            _ => NiveauDanger::Safe,
        }
    }

    fn origin(&self) -> ToolOrigin {
        ToolOrigin::Custom
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let mut command = self.def.command.clone();
        let mut stdin_data: Option<String> = None;

        if let Some(obj) = args.as_object() {
            for (key, value) in obj {
                let placeholder = format!("{{{{{}}}}}", key);
                let is_stdin_candidate = STDIN_ARGS.contains(&key.as_str());
                let command_has_placeholder = command.contains(&placeholder);

                let replacement = match value {
                    serde_json::Value::String(s) => {
                        if is_stdin_candidate && !command_has_placeholder {
                            match &mut stdin_data {
                                Some(existing) => {
                                    existing.push('\n');
                                    existing.push_str(s);
                                }
                                None => stdin_data = Some(s.clone()),
                            }
                        }
                        s.clone()
                    }
                    other => other.to_string(),
                };
                if command_has_placeholder {
                    command = command.replace(&placeholder, &replacement);
                }
            }
        }

        let shell = if cfg!(windows) { "cmd" } else { "sh" };
        let flag = if cfg!(windows) { "/C" } else { "-c" };
        let timeout_secs = self.def.timeout_secs.unwrap_or(30);

        let mut child = Command::new(shell)
            .arg(flag)
            .arg(&command)
            .current_dir(&ctx.working_dir)
            .stdin(if stdin_data.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(data) = stdin_data {
            if let Some(mut stdin_handle) = child.stdin.take() {
                let _ = stdin_handle.write_all(data.as_bytes()).await;
                let _ = stdin_handle.shutdown().await;
            }
        }

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            child.wait_with_output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let mut combined = stdout.to_string();
                if !stderr.is_empty() {
                    combined.push_str(&format!("\n--- stderr ---\n{}", stderr));
                }
                if combined.len() > 4000 {
                    combined.truncate(4000);
                    combined.push_str("\n...(truncated)");
                }
                Ok(ResultatAbeille::ok(combined))
            }
            Ok(Err(e)) => Ok(ResultatAbeille::err(format!("Plugin exec error: {}", e))),
            Err(_) => Ok(ResultatAbeille::err(format!(
                "Plugin timed out ({}s)",
                timeout_secs
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plugin_origin_is_custom() {
        let plugin = PluginAbeille::new(PluginDefinition {
            name: "custom_test".into(),
            description: "Custom test tool".into(),
            parameters: serde_json::json!({}),
            command: "echo ok".into(),
            danger: "safe".into(),
            timeout_secs: None,
        });
        assert_eq!(plugin.origin(), ToolOrigin::Custom);
        let registry = AbeilleRegistry::new();
        registry.enregistrer(Box::new(plugin));
        assert_eq!(registry.origin("custom_test"), Some(ToolOrigin::Custom));
        assert_eq!(registry.schema_complet()[0]["origin"], "custom");
    }

    #[tokio::test]
    async fn plugin_passes_long_arg_to_stdin_without_placeholder() {
        let command = if cfg!(windows) {
            "powershell -NoProfile -Command \"$input | Write-Output\""
        } else {
            "cat"
        };
        let plugin = PluginAbeille::new(PluginDefinition {
            name: "stdin_test".into(),
            description: "stdin test".into(),
            parameters: serde_json::json!({}),
            command: command.into(),
            danger: "safe".into(),
            timeout_secs: Some(5),
        });

        let result = plugin
            .executer(
                serde_json::json!({"message": "hello from stdin"}),
                &ContextExecution::default(),
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("hello from stdin"));
    }
}

pub fn charger_plugins(dir: &Path, registry: &AbeilleRegistry) -> usize {
    let mut count = 0;
    if !dir.exists() {
        let scripts_dir = dir.join("scripts");
        let _ = std::fs::create_dir_all(&scripts_dir);
    }
    let _ = std::fs::create_dir_all(dir.join("scripts"));

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to read plugins directory");
            return 0;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "json") {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<PluginDefinition>(&content) {
                    Ok(def) => {
                        tracing::info!(plugin = %def.name, file = %path.display(), "Loaded plugin");
                        registry.enregistrer(Box::new(PluginAbeille::new(def)));
                        count += 1;
                    }
                    Err(e) => {
                        tracing::warn!(file = %path.display(), error = %e, "Failed to parse plugin")
                    }
                },
                Err(e) => {
                    tracing::warn!(file = %path.display(), error = %e, "Failed to read plugin")
                }
            }
        }
    }
    if count > 0 {
        tracing::info!(count, dir = %dir.display(), "Plugins loaded");
    }
    count
}
