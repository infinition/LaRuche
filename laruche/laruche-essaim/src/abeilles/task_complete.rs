//! Outil `task_complete` — signal de fin explicite pour le LLM.
//!
//! Le modèle appelle ce tool quand la tâche est entièrement terminée et validée.
//! Dans la boucle ReAct, on détecte cet appel AVANT d'exécuter les outils
//! et on sort immédiatement avec le résumé structuré.

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
        "Appelle CE tool UNIQUEMENT quand la tâche est entièrement terminée \
         et validée. Fournit un résumé structuré des accomplissements. \
         NE PAS appeler pour des étapes intermédiaires."
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
                    "description": "0.0 to 1.0 — confidence in the result"
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
        let summary = args["summary"].as_str().unwrap_or("Tâche terminée");
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
            "[TASK_COMPLETE] Résumé : {summary}\nConfiance : {:.0}%",
            confidence * 100.0
        );
        if !artifacts.is_empty() {
            out.push_str(&format!("\nArtéfacts : {}", artifacts.join(", ")));
        }

        tracing::info!(
            summary_len = summary.len(),
            confidence,
            artifacts = artifacts.len(),
            "TaskComplete appelé"
        );

        Ok(ResultatAbeille::ok(out))
    }
}
