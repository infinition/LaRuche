//! `task_complete` tool: explicit completion signal for the LLM.
//!
//! The model calls this tool when the task is fully finished and validated.
//! In the ReAct loop, this call is detected BEFORE executing tools
//! and we exit immediately with the structured summary.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;

/// Une confiance, quelle que soit la forme sous laquelle le modele l'ecrit.
///
/// Nombre, nombre en chaine, pourcentage, ou mot. Absente, elle vaut 1.0: le modele
/// a declare la tache finie, c'est le signal qui compte. Un mot inconnu vaut aussi
/// 1.0 plutot que 0, pour ne jamais transformer une maladresse de formulation en
/// echec apparent.
fn confiance_normalisee(v: &serde_json::Value) -> f64 {
    let brut = match v {
        serde_json::Value::Number(_) => return v.as_f64().unwrap_or(1.0).clamp(0.0, 1.0),
        serde_json::Value::String(s) => s.trim().to_lowercase(),
        _ => return 1.0,
    };
    let sans_pourcent = brut.trim_end_matches('%').trim();
    if let Ok(n) = sans_pourcent.parse::<f64>() {
        // "90%" ou "90" designent 0.9; "0.9" se lit tel quel.
        let n = if brut.ends_with('%') || n > 1.0 { n / 100.0 } else { n };
        return n.clamp(0.0, 1.0);
    }
    match sans_pourcent {
        "certain" | "very high" | "tres haute" | "sure" => 1.0,
        "high" | "haute" | "elevee" => 0.9,
        "medium" | "moyenne" | "moderate" => 0.6,
        "low" | "faible" | "basse" => 0.3,
        "very low" | "tres faible" => 0.1,
        _ => 1.0,
    }
}

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
                // Optional, and deliberately permissive.
                //
                // Required + strictly numeric, it turned the only way to FINISH into a
                // trap: a scout that had done its work spent its remaining passes
                // bouncing between "missing required argument confidence" and
                // "confidence must be a number, got: string" (it answered "high"), and
                // died on its cap without ever handing back its report. A completion
                // signal must never be refused over a self-assessment score.
                //
                // The type union is what lets a string through: the validator only
                // enforces `type` when it is a single string, so both forms reach
                // `executer`, which normalises them.
                "confidence": {
                    "type": ["number", "string"],
                    "description": "Optional. 0.0 to 1.0, or one of: high, medium, low"
                },
                "artifacts": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Files/results produced (paths)"
                }
            },
            "required": ["summary"]
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
        let confidence = confiance_normalisee(&args["confidence"]);
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

#[cfg(test)]
mod tests {
    use super::confiance_normalisee;
    use serde_json::json;

    /// Toutes les formes qu'un modele ecrit reellement, et l'absence.
    #[test]
    fn la_confiance_se_lit_sous_toutes_ses_formes() {
        assert_eq!(confiance_normalisee(&json!(0.8)), 0.8);
        assert_eq!(confiance_normalisee(&json!("0.8")), 0.8);
        assert_eq!(confiance_normalisee(&json!("high")), 0.9);
        assert_eq!(confiance_normalisee(&json!("HIGH")), 0.9);
        assert_eq!(confiance_normalisee(&json!("medium")), 0.6);
        assert_eq!(confiance_normalisee(&json!("low")), 0.3);
        assert_eq!(confiance_normalisee(&json!("90%")), 0.9);
        assert_eq!(confiance_normalisee(&json!("90")), 0.9);
        assert_eq!(confiance_normalisee(&json!(90)), 1.0, "borne haute");
        // Absente ou incomprehensible: la tache est finie quand meme.
        assert_eq!(confiance_normalisee(&json!(null)), 1.0);
        assert_eq!(confiance_normalisee(&json!("pas mal")), 1.0);
    }

    /// La regression exacte: `task_complete` sans `confidence` doit passer la
    /// validation. Requise, elle empechait une eclaireuse de rendre son rapport.
    #[test]
    fn task_complete_sans_confiance_est_accepte() {
        use crate::abeille::{valider_et_normaliser_args, Abeille};
        let schema = super::TaskComplete.schema();
        let mut args = json!({ "summary": "rapport du scout" });
        assert!(valider_et_normaliser_args(&schema, &mut args).is_ok());
        let mut mot = json!({ "summary": "rapport", "confidence": "high" });
        assert!(
            valider_et_normaliser_args(&schema, &mut mot).is_ok(),
            "un mot ne doit plus etre refuse"
        );
    }
}
