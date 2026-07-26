//! `research_mode`: the model SELF-DECLARES a long-running research mission.
//!
//! Keyword heuristics on the user prompt ("recherche profonde"...) are a fragile
//! entry gate: the model, once it has read the mission, knows best whether it needs
//! the deep-research protocol (exploration mode: no premature conclusions, scout
//! fan-out, web-effort floor). This tool is the declaration channel:
//!
//! - **butinage engine**: the call is INTERCEPTED by `analyser` (cycle.rs) and never
//!   reaches this abeille - it flips `Carnet.mode` to `Exploration` and injects the
//!   deep-research protocol nudge. One-way escalation (a model cannot downgrade to
//!   escape the exploration rails).
//! - **legacy loop / direct call**: executing it is harmless (returns an ack), so the
//!   schema can be exposed everywhere.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;

pub struct ResearchMode;

#[async_trait]
impl Abeille for ResearchMode {
    fn nom(&self) -> &str {
        "research_mode"
    }

    fn description(&self) -> &str {
        "Declare that the current mission is a LONG-RUNNING research (deep research). \
         Call this FIRST when the user asks for a thorough/deep/exhaustive search, or when \
         you realize a quick lookup will not be enough. It activates the deep-research \
         protocol: parallel scout delegation, no premature conclusions, source cross-checking."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "mode": {
                    "type": "string",
                    "enum": ["deep", "standard"],
                    "description": "deep = long-running exhaustive research (default)"
                },
                "reason": {
                    "type": "string",
                    "description": "One line: why this mission needs deep research"
                }
            },
            "required": []
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        _args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        // Normally intercepted by the butinage engine before execution; this ack only
        // runs on the legacy path.
        Ok(ResultatAbeille::ok(
            "Deep-research mode noted. Decompose the question into independent angles, \
             dispatch parallel scouts (delegate), cross-check, and only conclude when new \
             angles stop yielding new information.",
        ))
    }
}
