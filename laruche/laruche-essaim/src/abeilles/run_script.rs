//! run_script — pipeline d'outils en UN seul tour (inspiré des scripts-RPC de third-party).
//!
//! Le modèle fournit une liste d'étapes `[{tool, args}]` ; on les exécute séquentiellement
//! via le registre, **sans repasser par le LLM** entre chaque étape. La sortie de l'étape N
//! est injectable dans les args des étapes suivantes via le jeton `{{N}}` (1-based).
//! Gros gain de contexte/latence sur les pipelines (ex. web_search → web_fetch → file_write).

use crate::abeille::{Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub struct RunScript {
    pub registry: Arc<AbeilleRegistry>,
}

/// Remplace `{{N}}` (1-based) par la sortie de l'étape N, récursivement dans les chaînes.
fn substitute_refs(value: &mut serde_json::Value, outputs: &[String]) {
    match value {
        serde_json::Value::String(s) => {
            for (idx, out) in outputs.iter().enumerate() {
                let token = format!("{{{{{}}}}}", idx + 1);
                if s.contains(&token) {
                    *s = s.replace(&token, out);
                }
            }
        }
        serde_json::Value::Array(arr) => arr.iter_mut().for_each(|v| substitute_refs(v, outputs)),
        serde_json::Value::Object(map) => {
            map.values_mut().for_each(|v| substitute_refs(v, outputs))
        }
        _ => {}
    }
}

#[async_trait]
impl Abeille for RunScript {
    fn nom(&self) -> &str {
        "run_script"
    }

    fn description(&self) -> &str {
        "Exécute une SÉQUENCE d'outils en un seul tour, sans repasser par le LLM entre les étapes \
         (pipeline). `steps` = liste d'objets {tool, args}. La sortie de l'étape N est injectable \
         dans les args suivants via le jeton {{N}} (1-based). Idéal pour enchaîner \
         ex. web_search → web_fetch → file_write sans aller-retours. (run_script et delegate sont interdits comme étapes.)"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "steps": {
                    "type": "array",
                    "description": "Liste ordonnée d'étapes",
                    "items": {
                        "type": "object",
                        "properties": {
                            "tool": { "type": "string", "description": "Nom de l'outil" },
                            "args": { "type": "object", "description": "Arguments de l'outil (peuvent contenir {{N}})" }
                        },
                        "required": ["tool"]
                    }
                }
            },
            "required": ["steps"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let steps = args["steps"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("'steps' (array) manquant"))?;
        if steps.is_empty() {
            return Ok(ResultatAbeille::err("steps vide"));
        }
        if steps.len() > 12 {
            return Ok(ResultatAbeille::err("trop d'étapes (max 12)"));
        }

        let mut outputs: Vec<String> = Vec::new();
        let mut report = String::new();

        for (i, step) in steps.iter().enumerate() {
            let tool = step["tool"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("étape {}: 'tool' manquant", i + 1))?;

            // Garde-fou : pas de récursion ni de délégation imbriquée.
            if tool == "run_script" || tool == "delegate" {
                report.push_str(&format!(
                    "── étape {} ({tool}) ──\n(interdit dans un pipeline)\n\n",
                    i + 1
                ));
                outputs.push(String::new());
                continue;
            }

            let mut step_args = step
                .get("args")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            substitute_refs(&mut step_args, &outputs);

            let res = self.registry.executer(tool, step_args, ctx).await?;
            let out = if res.success {
                res.output
            } else {
                format!("Erreur: {}", res.error.unwrap_or_default())
            };
            let preview: String = out.chars().take(1500).collect();
            report.push_str(&format!("── étape {} ({tool}) ──\n{preview}\n\n", i + 1));
            let failed = !res.success;
            outputs.push(out);
            if failed {
                report.push_str("(pipeline interrompu : étape en échec)\n");
                break;
            }
        }

        Ok(ResultatAbeille::ok(report))
    }
}
