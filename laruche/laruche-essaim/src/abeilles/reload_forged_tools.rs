use crate::abeille::{Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct ReloadForgedToolsTool {
    pub registry: Arc<AbeilleRegistry>,
}

#[async_trait]
impl Abeille for ReloadForgedToolsTool {
    fn nom(&self) -> &str {
        "reload_forged_tools"
    }

    fn description(&self) -> &str {
        "Hot-reload the 'forged_tools/' directory, one folder per Forged Tool. Call this immediately after creating or editing forged_tools/<name>/tool.json to make it available."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        _args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        use crate::abeille::ToolOrigin;

        let legacy_dir = ctx.working_dir.join("plugins");
        let forged_tools_dir = ctx.working_dir.join("forged_tools");
        self.registry.supprimer_par_origine(ToolOrigin::Forged);
        let legacy =
            crate::abeilles::forged_tools::charger_outils_herites(&legacy_dir, &self.registry);
        let count = legacy
            + crate::abeilles::forged_tools::charger_outils_forges(
                &forged_tools_dir,
                &self.registry,
            );
        Ok(ResultatAbeille::ok(format!(
            "{} Forged Tool(s) loaded or reloaded from {} ({} legacy).",
            count,
            forged_tools_dir.display(),
            legacy
        )))
    }
}
