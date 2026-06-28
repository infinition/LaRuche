//! clarify: the agent asks the user a question instead of guessing.
//!
//! The abeille declares the schema so the model knows it exists. The actual
//! behavior (handing control back to the user = end of turn) is short-circuited
//! in `brain.rs`: when the model calls `clarify`, the question becomes the turn's
//! response and the loop stops (the user answers on the next turn).

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
        "Ask the user ONE clarifying question when the request is ambiguous or a required detail is \
         missing. Prefer this over guessing. The turn stops and the user answers next."
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
        // Normally short-circuited by brain.rs; this fallback applies if called outside the loop.
        let q = args["question"].as_str().unwrap_or("(empty question)");
        Ok(ResultatAbeille::ok(format!(
            "Question sent to user: {q}"
        )))
    }
}
