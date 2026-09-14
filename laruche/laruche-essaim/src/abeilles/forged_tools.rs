//! Dynamic Forged Tools system for Abeilles.
//!
//! A forged tool is a folder: `forged_tools/<name>/tool.json` declares the tool, and the
//! files it runs sit beside it. The manifest and its body travel together, so
//! deleting the folder deletes the whole tool.
//!
//! Each manifest defines a tool that executes a shell command template with
//! arguments from the LLM. `{{forged_tool_dir}}` in the template expands to the
//! forged tool's own folder, so a command does not depend on the working directory.
//! `{{plugin_dir}}` remains accepted for manifests created before the rename.
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

/// File that declares a forged tool inside its folder.
pub const MANIFESTE: &str = "tool.json";
pub const MANIFESTE_HERITE: &str = "plugin.json";

/// Folder holding a forged tool: `forged_tools/<slug>/`.
pub fn dossier_outil_forge(racine: &Path, slug: &str) -> PathBuf {
    racine.join(slug)
}

/// Manifest of a forged tool: `forged_tools/<slug>/tool.json`.
pub fn chemin_manifeste(racine: &Path, slug: &str) -> PathBuf {
    dossier_outil_forge(racine, slug).join(MANIFESTE)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgedToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub command: String,
    #[serde(default = "default_danger")]
    pub danger: String,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    /// Folder the manifest was read from. Filled at load, never serialised: it
    /// is where the forged tool lives, not something an author declares.
    #[serde(skip)]
    pub dossier: PathBuf,
}

fn default_danger() -> String {
    "safe".to_string()
}

/// Long argument fields, passed via stdin instead of the shell.
const STDIN_ARGS: &[&str] = &["message", "text", "content", "code", "body"];

pub struct ForgedToolAbeille {
    def: ForgedToolDefinition,
}

impl ForgedToolAbeille {
    pub fn new(def: ForgedToolDefinition) -> Self {
        Self { def }
    }
}

#[async_trait]
impl Abeille for ForgedToolAbeille {
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
        ToolOrigin::Forged
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        // The forged tool's own folder, so `python {{forged_tool_dir}}/run.py` resolves the
        // same whatever directory the daemon was started from.
        let dossier = self.def.dossier.to_string_lossy().replace('\\', "/");
        let mut command = self
            .def
            .command
            .replace("{{forged_tool_dir}}", &dossier)
            .replace("{{plugin_dir}}", &dossier);
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
            Ok(Err(e)) => Ok(ResultatAbeille::err(format!(
                "Forged tool exec error: {}",
                e
            ))),
            Err(_) => Ok(ResultatAbeille::err(format!(
                "Forged tool timed out ({}s)",
                timeout_secs
            ))),
        }
    }
}

/// Registers every `forged_tools/<name>/tool.json` found under `dir`.
pub fn charger_outils_forges(dir: &Path, registry: &AbeilleRegistry) -> usize {
    charger_manifestes(dir, MANIFESTE, registry, false)
}

/// Reads the previous `plugins/<name>/plugin.json` layout without creating new
/// legacy data. Canonical manifests are loaded afterwards and win on conflicts.
pub fn charger_outils_herites(dir: &Path, registry: &AbeilleRegistry) -> usize {
    charger_manifestes(dir, MANIFESTE_HERITE, registry, true)
}

fn charger_manifestes(
    dir: &Path,
    manifeste_nom: &str,
    registry: &AbeilleRegistry,
    heritage: bool,
) -> usize {
    let mut count = 0;
    let _ = std::fs::create_dir_all(dir);

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(error = %e, directory = %dir.display(), "Failed to read forged tools directory");
            return 0;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|e| e == "json") {
            if let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) {
                tracing::warn!(
                    file = %path.display(),
                    expected = %dossier_outil_forge(dir, &stem).join(manifeste_nom).display(),
                    "Loose forged tool JSON ignored: move it into its own folder"
                );
            }
            continue;
        }

        if !path.is_dir() {
            continue;
        }
        let manifeste = path.join(manifeste_nom);
        if !manifeste.exists() {
            continue;
        }

        match std::fs::read_to_string(&manifeste) {
            Ok(content) => match serde_json::from_str::<ForgedToolDefinition>(&content) {
                Ok(mut def) => {
                    // forged_tool_delete resolves a forged tool by folder name, so a manifest
                    // declaring something else registers a tool nobody can remove.
                    let dossier_nom = path.file_name().unwrap_or_default().to_string_lossy();
                    if dossier_nom != def.name {
                        tracing::warn!(
                            folder = %dossier_nom,
                            declared = %def.name,
                            "Forged tool folder and name differ: forged_tool_delete will not find it"
                        );
                    }
                    def.dossier = path.clone();
                    tracing::info!(forged_tool = %def.name, file = %manifeste.display(), legacy = heritage, "Loaded forged tool");
                    registry.enregistrer(Box::new(ForgedToolAbeille::new(def)));
                    count += 1;
                }
                Err(e) => {
                    tracing::warn!(file = %manifeste.display(), error = %e, "Failed to parse forged tool")
                }
            },
            Err(e) => {
                tracing::warn!(file = %manifeste.display(), error = %e, "Failed to read forged tool")
            }
        }
    }
    if count > 0 {
        tracing::info!(count, dir = %dir.display(), legacy = heritage, "Forged tools loaded");
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forged_tool_origin_is_forged() {
        let forged_tool = ForgedToolAbeille::new(ForgedToolDefinition {
            name: "custom_test".into(),
            description: "Custom test tool".into(),
            parameters: serde_json::json!({}),
            command: "echo ok".into(),
            danger: "safe".into(),
            timeout_secs: None,
            dossier: PathBuf::new(),
        });
        assert_eq!(forged_tool.origin(), ToolOrigin::Forged);
        let registry = AbeilleRegistry::new();
        registry.enregistrer(Box::new(forged_tool));
        assert_eq!(registry.origin("custom_test"), Some(ToolOrigin::Forged));
        assert_eq!(registry.schema_complet()[0]["origin"], "forged");
    }

    #[tokio::test]
    async fn forged_tool_passes_long_arg_to_stdin_without_placeholder() {
        let command = if cfg!(windows) {
            "powershell -NoProfile -Command \"$input | Write-Output\""
        } else {
            "cat"
        };
        let forged_tool = ForgedToolAbeille::new(ForgedToolDefinition {
            name: "stdin_test".into(),
            description: "stdin test".into(),
            parameters: serde_json::json!({}),
            command: command.into(),
            danger: "safe".into(),
            timeout_secs: Some(60),
            dossier: PathBuf::new(),
        });

        let result = forged_tool
            .executer(
                serde_json::json!({"message": "hello from stdin"}),
                &ContextExecution::default(),
            )
            .await
            .unwrap();

        // Le motif de l'echec, et pas seulement le fait qu'il y en ait un: sans
        // lui, un echec sur une machine qu'on n'a pas sous la main ne dit rien.
        assert!(
            result.success,
            "le forged_tool a echoue: {:?} / sortie: {}",
            result.error, result.output
        );
        assert!(result.output.contains("hello from stdin"));
    }

    fn ecrire_forged_tool(racine: &Path, slug: &str, commande: &str) {
        let dossier = dossier_outil_forge(racine, slug);
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
    fn charge_un_forged_tool_par_dossier() {
        let base =
            std::env::temp_dir().join(format!("laruche-forged_tools-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        ecrire_forged_tool(&base, "meteo", "echo ok");
        // A folder without a manifest is not a forged_tool, it is just a folder.
        std::fs::create_dir_all(base.join("brouillon")).unwrap();

        let registry = AbeilleRegistry::new();
        assert_eq!(charger_outils_forges(&base, &registry), 1);
        assert_eq!(registry.origin("meteo"), Some(ToolOrigin::Forged));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn charge_un_ancien_manifeste_comme_outil_forge() {
        let base =
            std::env::temp_dir().join(format!("laruche-legacy-tools-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dossier = dossier_outil_forge(&base, "meteo_legacy");
        std::fs::create_dir_all(&dossier).unwrap();
        std::fs::write(
            dossier.join(MANIFESTE_HERITE),
            r#"{"name":"meteo_legacy","description":"test","parameters":{},"command":"echo ok"}"#,
        )
        .unwrap();

        let registry = AbeilleRegistry::new();
        assert_eq!(charger_outils_herites(&base, &registry), 1);
        assert_eq!(registry.origin("meteo_legacy"), Some(ToolOrigin::Forged));
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
        assert_eq!(charger_outils_forges(&base, &registry), 0);
        assert_eq!(registry.origin("ancien"), None);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn forged_tool_dir_pointe_sur_le_dossier_du_forged_tool() {
        let echo = if cfg!(windows) { "cmd /C echo" } else { "echo" };
        let forged_tool = ForgedToolAbeille::new(ForgedToolDefinition {
            name: "chemin".into(),
            description: "d".into(),
            parameters: serde_json::json!({}),
            command: format!("{echo} {{{{forged_tool_dir}}}}/run.py"),
            danger: "safe".into(),
            timeout_secs: Some(5),
            dossier: PathBuf::from("forged_tools").join("chemin"),
        });

        let result = forged_tool
            .executer(serde_json::json!({}), &ContextExecution::default())
            .await
            .unwrap();

        assert!(result.success);
        assert!(
            result.output.contains("forged_tools/chemin/run.py"),
            "sortie inattendue: {}",
            result.output
        );
    }

    #[tokio::test]
    async fn ancien_placeholder_reste_un_alias_du_dossier() {
        let echo = if cfg!(windows) { "cmd /C echo" } else { "echo" };
        let forged_tool = ForgedToolAbeille::new(ForgedToolDefinition {
            name: "chemin_legacy".into(),
            description: "d".into(),
            parameters: serde_json::json!({}),
            command: format!("{echo} {{{{plugin_dir}}}}/run.py"),
            danger: "safe".into(),
            timeout_secs: Some(5),
            dossier: PathBuf::from("plugins").join("chemin_legacy"),
        });

        let result = forged_tool
            .executer(serde_json::json!({}), &ContextExecution::default())
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("plugins/chemin_legacy/run.py"));
    }
}
