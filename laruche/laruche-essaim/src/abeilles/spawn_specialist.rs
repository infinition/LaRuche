//! Outil `spawn_specialist` — agent spécialisé par rôle.
//!
//! Permet à l'orchestrateur de déléguer une tâche à un agent spécialisé
//! avec son propre system prompt, ses itérations et ses outils restreints.
//! Contrairement à `delegate` (générique), `spawn_specialist` adapte
//! la configuration selon le rôle. Supporte l'override de provider.

use crate::abeille::{Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille};
use crate::brain::EssaimConfig;
use crate::subagent::{config_agent_specialise, lancer_sous_agent, AgentRole, ProviderConfig};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub struct SpawnSpecialist {
    pub registry: Arc<AbeilleRegistry>,
    pub config: EssaimConfig,
}

#[async_trait]
impl Abeille for SpawnSpecialist {
    fn nom(&self) -> &str {
        "spawn_specialist"
    }

    fn description(&self) -> &str {
        "Spawn a role-specific sub-agent for a complex task. \
         Roles: 'research' (web, 25 iter), 'experiment' (code, 15 iter), \
         'critique' (validation, 8 iter), 'synthesis' (report, 5 iter). \
         Optional: 'provider' and 'model' to override the target LLM."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "role": {
                    "type": "string",
                    "enum": ["research", "experiment", "critique", "synthesis"],
                    "description": "Type of specialized agent"
                },
                "task": { "type": "string" },
                "context": { "type": "string" },
                "provider": {
                    "type": "string",
                    "description": "Provider override (ollama, openai, anthropic)"
                },
                "model": {
                    "type": "string",
                    "description": "Model override (e.g. deepseek-chat, gemma4:e4b)"
                }
            },
            "required": ["role", "task"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let role = args["role"]
            .as_str()
            .map(AgentRole::from_str)
            .unwrap_or(AgentRole::Recherche);
        let task = args["task"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required argument 'task'"))?;
        let context = args["context"].as_str();

        let provider_override = match (args["provider"].as_str(), args["model"].as_str()) {
            (Some(p), Some(m)) => Some(ProviderConfig {
                provider: p.to_string(),
                model: m.to_string(),
                api_key: std::env::var(format!("{}_API_KEY", p.to_uppercase())).unwrap_or_default(),
                api_base: None,
            }),
            _ => None,
        };

        tracing::info!(
            role = ?role,
            task_len = task.len(),
            provider = ?provider_override.as_ref().map(|p| &p.provider),
            "Spawning specialist agent"
        );

        let cfg = config_agent_specialise(&self.config, role, provider_override);
        let result = lancer_sous_agent(task, context, self.registry.clone(), &cfg).await?;

        tracing::info!(
            role = ?role,
            summary_len = result.summary.len(),
            iterations = result.iterations_limit,
            "Specialist agent completed"
        );

        Ok(ResultatAbeille::ok(format!(
            "Specialist agent '{}' done (max {} iterations):\n{}",
            args["role"].as_str().unwrap_or("?"),
            result.iterations_limit,
            result.summary
        )))
    }
}
