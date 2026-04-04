//! Dynamic plugin system for Abeilles.
//!
//! Plugins are JSON files in a `plugins/` directory. Each file defines a tool
//! that executes a shell command template with arguments from the LLM.
//!
//! Example plugin file (`plugins/docker-status.json`):
//! ```json
//! {
//!   "name": "docker_status",
//!   "description": "Get Docker container status",
//!   "parameters": {
//!     "type": "object",
//!     "properties": {
//!       "container": { "type": "string", "description": "Container name or ID" }
//!     },
//!     "required": []
//!   },
//!   "command": "docker ps --filter name={{container}} --format '{{.Names}} {{.Status}}'",
//!   "danger": "safe"
//! }
//! ```

use crate::abeille::{Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
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

/// A dynamically loaded plugin abeille.
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

    async fn executer(
        &self,
        args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        // Template substitution: replace {{param}} with actual values
        let mut command = self.def.command.clone();
        if let Some(obj) = args.as_object() {
            for (key, value) in obj {
                let placeholder = format!("{{{{{}}}}}", key);
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                command = command.replace(&placeholder, &replacement);
            }
        }

        let shell = if cfg!(windows) { "cmd" } else { "sh" };
        let flag = if cfg!(windows) { "/C" } else { "-c" };
        let timeout = self.def.timeout_secs.unwrap_or(30);

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout),
            Command::new(shell)
                .arg(flag)
                .arg(&command)
                .current_dir(&ctx.working_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
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
                // Truncate
                if combined.len() > 4000 {
                    combined.truncate(4000);
                    combined.push_str("\n...(truncated)");
                }
                Ok(ResultatAbeille::ok(combined))
            }
            Ok(Err(e)) => Ok(ResultatAbeille::err(format!("Plugin exec error: {}", e))),
            Err(_) => Ok(ResultatAbeille::err(format!("Plugin timed out ({}s)", timeout))),
        }
    }
}

/// Scan a directory for plugin JSON files and register them.
pub fn charger_plugins(dir: &Path, registry: &mut AbeilleRegistry) -> usize {
    let mut count = 0;

    if !dir.exists() {
        // Create plugins directory with an example
        if let Err(e) = std::fs::create_dir_all(dir) {
            tracing::warn!(error = %e, "Failed to create plugins directory");
            return 0;
        }
        // Write example plugin
        let example = PluginDefinition {
            name: "example_hello".to_string(),
            description: "Example plugin: says hello (delete this file to remove)".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Name to greet" }
                },
                "required": ["name"]
            }),
            command: if cfg!(windows) {
                "echo Hello, {{name}}!".to_string()
            } else {
                "echo 'Hello, {{name}}!'".to_string()
            },
            danger: "safe".to_string(),
            timeout_secs: Some(5),
        };
        let example_path = dir.join("example_hello.json");
        if let Ok(json) = serde_json::to_string_pretty(&example) {
            let _ = std::fs::write(&example_path, json);
        }
    }

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
                        tracing::info!(
                            plugin = %def.name,
                            file = %path.display(),
                            "Loaded plugin"
                        );
                        registry.enregistrer(Box::new(PluginAbeille::new(def)));
                        count += 1;
                    }
                    Err(e) => {
                        tracing::warn!(
                            file = %path.display(),
                            error = %e,
                            "Failed to parse plugin"
                        );
                    }
                },
                Err(e) => {
                    tracing::warn!(file = %path.display(), error = %e, "Failed to read plugin");
                }
            }
        }
    }

    if count > 0 {
        tracing::info!(count, dir = %dir.display(), "Plugins loaded");
    }

    count
}
