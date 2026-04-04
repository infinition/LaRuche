//! Sub-agent delegation — allows the main agent to spawn sub-tasks.
//!
//! Inspired by OpenClaw's multi-agent routing. The main agent can delegate
//! a sub-task to a fresh agent context that runs independently and returns
//! a result. This enables complex task decomposition.

use crate::abeille::{Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille};
use crate::brain::{boucle_react, ChatEvent, EssaimConfig};
use crate::session::Session;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Delegate a sub-task to a fresh agent context.
/// The sub-agent runs the full ReAct loop independently and returns the result.
pub struct Delegate {
    pub registry: Arc<AbeilleRegistry>,
    pub config: EssaimConfig,
}

#[async_trait]
impl Abeille for Delegate {
    fn nom(&self) -> &str {
        "delegate"
    }

    fn description(&self) -> &str {
        "Delegate a sub-task to a fresh agent. The sub-agent will execute the task \
         independently using all available tools and return the result. \
         Use this for complex tasks that can be broken into independent sub-tasks, \
         or when you need to run something in a separate context."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "The task description for the sub-agent to execute"
                },
                "context": {
                    "type": "string",
                    "description": "Optional context or instructions for the sub-agent"
                }
            },
            "required": ["task"]
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
        let task = args["task"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'task' argument"))?;
        let context = args["context"].as_str().unwrap_or("");

        let full_prompt = if context.is_empty() {
            task.to_string()
        } else {
            format!("{}\n\nContext: {}", task, context)
        };

        tracing::info!(task = %task, "Spawning sub-agent");

        // Create a fresh session for the sub-agent
        let mut sub_session = Session::new(&self.config.model);

        // Create a broadcast channel (we won't forward these events to the main UI)
        let (tx, _rx) = broadcast::channel::<ChatEvent>(64);

        // Run the sub-agent with a limited iteration count
        let mut sub_config = self.config.clone();
        sub_config.max_iterations = 8; // Sub-agents are more limited

        let result = boucle_react(
            &full_prompt,
            &mut sub_session,
            &self.registry,
            &sub_config,
            &tx,
        )
        .await;

        match result {
            Ok(response) => {
                tracing::info!(
                    task = %task,
                    response_len = response.len(),
                    "Sub-agent completed"
                );
                Ok(ResultatAbeille::ok(format!(
                    "Sub-agent result:\n{}",
                    response
                )))
            }
            Err(e) => {
                tracing::warn!(task = %task, error = %e, "Sub-agent failed");
                Ok(ResultatAbeille::err(format!(
                    "Sub-agent failed: {}",
                    e
                )))
            }
        }
    }
}
