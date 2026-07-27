//! Dynamic plugin system for Abeilles.
//!
//! A plugin is a folder: `plugins/<name>/plugin.json` declares the tool, and the
//! files it runs sit beside it. The manifest and its body travel together, so
//! deleting the folder deletes the whole plugin. The flat layout that preceded
//! this one kept bodies in a shared `plugins/scripts/`, where every deletion left
//! an orphan behind.
//!
//! Each manifest defines a tool that executes a shell command template with
//! arguments from the LLM. `{{plugin_dir}}` in the template expands to the
//! plugin's own folder, so a command does not depend on the working directory.
//!
//! **Arguments passing**: arguments are injected into the command via the
//! `{{param}}` placeholder, BUT long, multi-line arguments (`message`,
//! `text`, `content`, `code`) are automatically passed via **stdin** to
//! avoid shell quoting issues (especially on Windows cmd.exe).

use crate::abeille::{
    Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille, ToolOrigin,
};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// File that declares a plugin inside its folder.
pub const MANIFESTE: &str = "plugin.json";

/// Folder holding a plugin: `plugins/<slug>/`.
pub fn dossier_plugin(racine: &Path, slug: &str) -> PathBuf {
    racine.join(slug)
}

/// Manifest of a plugin: `plugins/<slug>/plugin.json`.
pub fn chemin_manifeste(racine: &Path, slug: &str) -> PathBuf {
    dossier_plugin(racine, slug).join(MANIFESTE)
}

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
    /// Folder the manifest was read from. Filled at load, never serialised: it
    /// is where the plugin lives, not something an author declares.
    #[serde(skip)]
    pub dossier: PathBuf,
}

fn default_danger() -> String {
    "safe".to_string()
}

/// Long argument fields, passed via stdin instead of the shell.
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
        // The plugin's own folder, so `python {{plugin_dir}}/run.py` resolves the
        // same whatever directory the daemon was started from.
        let mut command = self.def.command.replace(
            "{{plugin_dir}}",
            &self.def.dossier.to_string_lossy().replace('\\', "/"),
        );
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
            dossier: PathBuf::new(),
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
            dossier: PathBuf::new(),
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

    fn ecrire_plugin(racine: &Path, slug: &str, commande: &str) {
        let dossier = dossier_plugin(racine, slug);
        std::fs::create_dir_all(&dossier).unwrap();
        let def = serde_json::json!({
            "name": slug,
            "description": "test",
            "parameters": {"type":"object","properties":{}},
            "command": commande,
        });
        std::fs::write(chemin_manifeste(racine, slug), def.to_string()).unwrap();
    }

    #[test]
    fn charge_un_plugin_par_dossier() {
        let base = std::env::temp_dir().join(format!("laruche-plugins-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        ecrire_plugin(&base, "meteo", "echo ok");
        // A folder without a manifest is not a plugin, it is just a folder.
        std::fs::create_dir_all(base.join("brouillon")).unwrap();

        let registry = AbeilleRegistry::new();
        assert_eq!(charger_plugins(&base, &registry), 1);
        assert_eq!(registry.origin("meteo"), Some(ToolOrigin::Custom));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn un_json_a_plat_est_ignore_et_signale() {
        let base = std::env::temp_dir().join(format!("laruche-plat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(
            base.join("ancien.json"),
            r#"{"name":"ancien","description":"d","parameters":{},"command":"echo x"}"#,
        )
        .unwrap();

        let registry = AbeilleRegistry::new();
        assert_eq!(charger_plugins(&base, &registry), 0);
        assert_eq!(registry.origin("ancien"), None);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn plugin_dir_pointe_sur_le_dossier_du_plugin() {
        let echo = if cfg!(windows) { "cmd /C echo" } else { "echo" };
        let plugin = PluginAbeille::new(PluginDefinition {
            name: "chemin".into(),
            description: "d".into(),
            parameters: serde_json::json!({}),
            command: format!("{echo} {{{{plugin_dir}}}}/run.py"),
            danger: "safe".into(),
            timeout_secs: Some(5),
            dossier: PathBuf::from("plugins").join("chemin"),
        });

        let result = plugin
            .executer(serde_json::json!({}), &ContextExecution::default())
            .await
            .unwrap();

        assert!(result.success);
        assert!(
            result.output.contains("plugins/chemin/run.py"),
            "sortie inattendue: {}",
            result.output
        );
    }
}

/// Registers every `plugins/<name>/plugin.json` found under `dir`.
///
/// A JSON file sitting loose at the root is the layout this replaced. It is not
/// loaded, and it is reported by name with the folder it should move to, because
/// a plugin that silently stops existing is the worst of the two failures.
pub fn charger_plugins(dir: &Path, registry: &AbeilleRegistry) -> usize {
    let mut count = 0;
    let _ = std::fs::create_dir_all(dir);

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to read plugins directory");
            return 0;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |e| e == "json") {
            if let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) {
                tracing::warn!(
                    file = %path.display(),
                    expected = %chemin_manifeste(dir, &stem).display(),
                    "Loose plugin JSON ignored: move it into its own folder"
                );
            }
            continue;
        }

        if !path.is_dir() {
            continue;
        }
        let manifeste = path.join(MANIFESTE);
        if !manifeste.exists() {
            continue;
        }

        match std::fs::read_to_string(&manifeste) {
            Ok(content) => match serde_json::from_str::<PluginDefinition>(&content) {
                Ok(mut def) => {
                    // plugin_delete resolves a plugin by folder name, so a manifest
                    // declaring something else registers a tool nobody can remove.
                    let dossier_nom = path.file_name().unwrap_or_default().to_string_lossy();
                    if dossier_nom != def.name {
                        tracing::warn!(
                            folder = %dossier_nom,
                            declared = %def.name,
                            "Plugin folder and name differ: plugin_delete will not find it"
                        );
                    }
                    def.dossier = path.clone();
                    tracing::info!(plugin = %def.name, file = %manifeste.display(), "Loaded plugin");
                    registry.enregistrer(Box::new(PluginAbeille::new(def)));
                    count += 1;
                }
                Err(e) => {
                    tracing::warn!(file = %manifeste.display(), error = %e, "Failed to parse plugin")
                }
            },
            Err(e) => {
                tracing::warn!(file = %manifeste.display(), error = %e, "Failed to read plugin")
            }
        }
    }
    if count > 0 {
        tracing::info!(count, dir = %dir.display(), "Plugins loaded");
    }
    count
}
