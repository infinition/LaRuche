//! Sub-agent delegation: allows the main agent to spawn sub-tasks.
//!
//! Inspired by third-party's multi-agent routing. The main agent can delegate
//! a sub-task to a fresh agent context that runs independently and returns
//! a result. This enables complex task decomposition.

use crate::abeille::{Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille};
use crate::brain::EssaimConfig;
use crate::subagent::lancer_sous_agent;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

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
        "Delegate a sub-task to a fresh sub-agent with an isolated context. \
         Use this for complex tasks that break into independent sub-tasks — especially \
         RESEARCH: dispatch SEVERAL delegate calls in the SAME message (one per angle), \
         they run in PARALLEL and each returns a compact report."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "Precise, self-contained brief for the sub-agent"
                },
                "context": {
                    "type": "string",
                    "description": "Optional context or instructions for the sub-agent"
                },
                "role": {
                    "type": "string",
                    "enum": ["eclaireuse", "ouvriere", "gardienne", "architecte"],
                    "description": "eclaireuse=broad research scout (default), ouvriere=execute a focused sub-task, gardienne=critically verify a claim/result, architecte=synthesize provided material"
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

        tracing::info!(task = %task, "Spawning sub-agent");

        let result = lancer_sous_agent(task, Some(context), self.registry.clone(), &self.config)
            .await
            .map(|result| result.summary);

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
                Ok(ResultatAbeille::err(format!("Sub-agent failed: {}", e)))
            }
        }
    }
}
