//! clarify — l'agent pose une question à l'utilisateur au lieu de deviner.
//!
//! L'abeille déclare le schéma pour que le modèle sache qu'elle existe. Le vrai
//! comportement (rendre la main à l'utilisateur = fin de tour) est court-circuité
//! dans `brain.rs` : quand le modèle appelle `clarify`, la question devient la réponse
//! du tour et la boucle s'arrête (l'utilisateur répond au tour suivant).

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;

pub struct Clarify;

#[async_trait]
impl Abeille for Clarify {
    fn nom(&self) -> &str {
        "clarify"
    }

    fn description(&self) -> &str {
        "Pose UNE question de clarification à l'utilisateur quand la demande est ambiguë ou qu'il \
         manque une info essentielle. Préfère ça à deviner. Le tour s'arrête et l'utilisateur répond."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "question": { "type": "string", "description": "The question to ask the user" }
            },
            "required": ["question"]
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
        // Normalement court-circuité par brain.rs ; ce repli sert si appelé hors boucle.
        let q = args["question"].as_str().unwrap_or("(question vide)");
        Ok(ResultatAbeille::ok(format!(
            "Question posée à l'utilisateur : {q}"
        )))
    }
}
