//! `task_complete` tool: explicit completion signal for the LLM.
//!
//! The model calls this tool when the task is fully finished and validated.
//! In the ReAct loop, this call is detected BEFORE executing tools
//! and we exit immediately with the structured summary.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;

pub struct TaskComplete;

#[async_trait]
impl Abeille for TaskComplete {
    fn nom(&self) -> &str {
        "task_complete"
    }

    fn description(&self) -> &str {
        "Call THIS tool ONLY when the task is fully finished \
         and validated. Provide a structured summary of accomplishments. \
         DO NOT call it for intermediate steps."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "summary": {
                    "type": "string",
                    "description": "What was accomplished (2-3 sentences max)"
                },
                "confidence": {
                    "type": "number",
                    "description": "0.0 to 1.0 - confidence in the result"
                },
                "artifacts": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Files/results produced (paths)"
                }
            },
            "required": ["summary", "confidence"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let summary = args["summary"].as_str().unwrap_or("Task complete");
        let confidence = args["confidence"].as_f64().unwrap_or(1.0);
        let artifacts: Vec<String> = args["artifacts"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let mut out = format!(
            "[TASK_COMPLETE] Summary: {summary}\nConfidence: {:.0}%",
            confidence * 100.0
        );
        if !artifacts.is_empty() {
            out.push_str(&format!("\nArtifacts: {}", artifacts.join(", ")));
        }

        tracing::info!(
            summary_len = summary.len(),
            confidence,
            artifacts = artifacts.len(),
            "TaskComplete called"
        );

        Ok(ResultatAbeille::ok(out))
    }
}
