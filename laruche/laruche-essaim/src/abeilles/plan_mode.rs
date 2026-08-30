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

        // Un plan.md deja la est le travail de quelqu'un: soit le plan en cours,
        // soit celui d'hier qu'on relit. L'ecraser en silence est une perte de
        // donnees, et c'etait assez previsible pour que le skill le documente
        // comme un piege. On le met de cote plutot que de le detruire, et on dit
        // ou il est parti.
        let mut ecarte = None;
        if path.exists() {
            let quand = chrono::Local::now().format("%Y-%m-%d_%H%M%S");
            let sauvegarde = ctx.working_dir.join(format!("plan-{quand}.md"));
            match std::fs::rename(&path, &sauvegarde) {
                Ok(_) => ecarte = Some(sauvegarde),
                Err(e) => {
                    return Ok(ResultatAbeille::err(format!(
                        "A plan.md is already here and could not be moved aside ({e}), so \
                         nothing was written. Move it yourself, then call plan_mode again."
                    )))
                }
            }
        }

        let initial_content = format!(
            "# {titre}\n\n## Context\n\n## Proposed Steps\n\n## Approval Required\n(Write your questions here)\n"
        );
        match std::fs::write(&path, initial_content) {
            Ok(_) => Ok(ResultatAbeille::ok(format!(
                "Plan mode active. '{}' created. Write your detailed plan in it, then request \
                 user approval.{}",
                path.display(),
                match ecarte {
                    Some(p) => format!(
                        " The plan that was already there was moved to '{}'.",
                        p.display()
                    ),
                    None => String::new(),
                }
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!(
                "Failed to create plan.md: {e}"
            ))),
        }
    }
}
