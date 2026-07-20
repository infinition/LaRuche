//! `finding`: record a decisive fact into the mission's findings ledger.
//!
//! The call is INTERCEPTED by the butinage engine (`cycle::analyser`) and never
//! reaches this abeille: the fact goes into `Carnet.decouvertes`, a bounded
//! machine-side ledger that SURVIVES context compaction/truncation and is
//! re-rendered at the tail of every outbound context. The final synthesis builds
//! on the ledger instead of whatever the compaction summary happened to keep.
//!
//! On the legacy path (or a direct call) executing it is harmless: it acks.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;

pub struct Finding;

#[async_trait]
impl Abeille for Finding {
    fn nom(&self) -> &str {
        "finding"
    }

    fn description(&self) -> &str {
        "Record ONE decisive fact you just learned into the mission's findings ledger \
         (with its source URL). The ledger survives context compaction: anything NOT \
         recorded may be lost before the final synthesis. Call it the moment you learn \
         a fact that the final answer will need."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "fact": {
                    "type": "string",
                    "description": "The fact, one concise sentence (names, numbers, dates exact)"
                },
                "source": {
                    "type": "string",
                    "description": "Source URL or reference backing this fact"
                }
            },
            "required": ["fact"]
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
        // Normally intercepted by the butinage engine before execution.
        Ok(ResultatAbeille::ok("Fact recorded in the findings ledger."))
    }
}
