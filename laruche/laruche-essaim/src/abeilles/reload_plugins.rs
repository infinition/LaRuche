use crate::abeille::{Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

pub struct ReloadPluginsTool {
    pub registry: Arc<AbeilleRegistry>,
}

#[async_trait]
impl Abeille for ReloadPluginsTool {
    fn nom(&self) -> &str {
        "reload_plugins"
    }

    fn description(&self) -> &str {
        "Recharge à chaud le dossier 'plugins/' et 'plugins/scripts/'. À appeler IMMÉDIATEMENT après avoir créé ou modifié un fichier JSON de plugin pour qu'il devienne utilisable."
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
        let plugins_dir = ctx.working_dir.join("plugins");
        let count = crate::abeilles::plugins::charger_plugins(&plugins_dir, &self.registry);
        Ok(ResultatAbeille::ok(format!(
            "{} plugins chargés ou rechargés avec succès depuis {}.",
            count,
            plugins_dir.display()
        )))
    }
}
