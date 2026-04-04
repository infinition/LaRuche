use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Danger level for a tool — determines approval gating behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NiveauDanger {
    /// Safe to execute without confirmation (e.g., file_read, math)
    Safe,
    /// Requires user approval before execution (e.g., file_write, shell)
    NeedsApproval,
    /// Blocked by default — must be explicitly allowlisted (e.g., rm -rf)
    Dangerous,
}

/// Result returned by an Abeille after execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultatAbeille {
    /// Whether the tool executed successfully
    pub success: bool,
    /// The output/result text to show the LLM
    pub output: String,
    /// Optional error message
    pub error: Option<String>,
}

impl ResultatAbeille {
    pub fn ok(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        let error = error.into();
        Self {
            success: false,
            output: String::new(),
            error: Some(error),
        }
    }
}

/// Execution context passed to each Abeille — contains sandbox limits and config.
#[derive(Debug, Clone)]
pub struct ContextExecution {
    /// Allowed base directories for file operations
    pub allowed_dirs: Vec<PathBuf>,
    /// Allowed shell commands (if empty, all are blocked)
    pub shell_allowlist: Vec<String>,
    /// Working directory for the current session
    pub working_dir: PathBuf,
}

impl Default for ContextExecution {
    fn default() -> Self {
        Self {
            allowed_dirs: vec![],
            shell_allowlist: vec![],
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

/// The core tool trait. Each tool ("Abeille") implements this.
#[async_trait]
pub trait Abeille: Send + Sync {
    /// Unique tool name (e.g., "file_read", "web_search")
    fn nom(&self) -> &str;

    /// Human-readable description for the LLM
    fn description(&self) -> &str;

    /// JSON Schema describing the tool's parameters.
    /// This is injected into the system prompt so the LLM knows how to call the tool.
    fn schema(&self) -> serde_json::Value;

    /// Danger level — determines if user approval is needed
    fn niveau_danger(&self) -> NiveauDanger;

    /// Execute the tool with parsed JSON arguments.
    async fn executer(
        &self,
        args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille>;
}

/// Registry holding all available Abeilles (tools).
pub struct AbeilleRegistry {
    abeilles: HashMap<String, Box<dyn Abeille>>,
}

impl AbeilleRegistry {
    pub fn new() -> Self {
        Self {
            abeilles: HashMap::new(),
        }
    }

    /// Register a new Abeille.
    pub fn enregistrer(&mut self, abeille: Box<dyn Abeille>) {
        let nom = abeille.nom().to_string();
        tracing::info!(tool = %nom, "Abeille registered");
        self.abeilles.insert(nom, abeille);
    }

    /// Get a reference to an Abeille by name.
    pub fn get(&self, nom: &str) -> Option<&dyn Abeille> {
        self.abeilles.get(nom).map(|a| a.as_ref())
    }

    /// Get all tool names.
    pub fn noms(&self) -> Vec<&str> {
        self.abeilles.keys().map(|s| s.as_str()).collect()
    }

    /// Generate the complete JSON schema for all tools — injected into the system prompt.
    pub fn schema_complet(&self) -> serde_json::Value {
        let tools: Vec<serde_json::Value> = self
            .abeilles
            .values()
            .map(|a| {
                serde_json::json!({
                    "name": a.nom(),
                    "description": a.description(),
                    "parameters": a.schema(),
                })
            })
            .collect();
        serde_json::Value::Array(tools)
    }

    /// Execute an Abeille by name.
    pub async fn executer(
        &self,
        nom: &str,
        args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        match self.abeilles.get(nom) {
            Some(abeille) => abeille.executer(args, ctx).await,
            None => Ok(ResultatAbeille::err(format!(
                "Unknown tool: '{}'. Available tools: {}",
                nom,
                self.noms().join(", ")
            ))),
        }
    }
}

impl Default for AbeilleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
