use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct PlanModeTool;

#[async_trait]
impl Abeille for PlanModeTool {
    fn nom(&self) -> &str {
        "plan_mode"
    }

    fn description(&self) -> &str {
        "Enter plan mode. Creates plan.md where you must write a detailed technical plan before making any changes. Use it to request user approval for major modifications."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "titre": {
                    "type": "string",
                    "description": "Implementation plan title"
                }
            },
            "required": ["titre"],
            "additionalProperties": false
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let titre = args
            .get("titre")
            .and_then(|t| t.as_str())
            .unwrap_or("Implementation Plan");
        let path = ctx.working_dir.join("plan.md");
        let initial_content = format!("# {}\n\n## Context\n\n## Proposed Steps\n\n## Approval Required\n(Write your questions here)\n", titre);

        match std::fs::write(&path, initial_content) {
            Ok(_) => Ok(ResultatAbeille::ok(format!("Plan mode active. File '{}' created. Write your detailed plan in it, then request user approval.", path.display()))),
            Err(e) => Ok(ResultatAbeille::err(format!("Failed to create plan.md: {}", e)))
        }
    }
}
