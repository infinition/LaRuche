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
        "Active le mode planification. Crée un fichier plan.md où tu dois écrire ton plan technique détaillé avant toute modification. Utilise-le pour demander la validation de l'utilisateur sur tes changements majeurs."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "titre": {
                    "type": "string",
                    "description": "Titre du plan d'implémentation"
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
            .unwrap_or("Plan d'implémentation");
        let path = ctx.working_dir.join("plan.md");
        let initial_content = format!("# {}\n\n## Contexte\n\n## Étapes proposées\n\n## Validation requise\n(Pose tes questions ici)\n", titre);

        match std::fs::write(&path, initial_content) {
            Ok(_) => Ok(ResultatAbeille::ok(format!("Mode Plan activé. Fichier '{}' créé. Édite ce fichier avec ton plan détaillé puis demande la validation de l'utilisateur.", path.display()))),
            Err(e) => Ok(ResultatAbeille::err(format!("Erreur lors de la création de plan.md: {}", e)))
        }
    }
}
