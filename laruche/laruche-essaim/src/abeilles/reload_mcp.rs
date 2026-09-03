use crate::abeille::{Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Relire `mcp_servers.json` et s'y reconnecter, sans redemarrer la ruche.
///
/// Il existait `reload_plugins` pour les plugins, et rien pour le MCP. Un agent
/// qui venait d'enregistrer un serveur avec `mcp_add` n'avait donc aucun moyen
/// de le rendre joignable: il appelait `reload_plugins`, qui recharge un tout
/// autre dossier et ne dit pas non, puis constatait que ses outils n'existaient
/// toujours pas et concluait qu'il fallait redemarrer. Redemarrer est justement
/// ce qu'il ne peut pas faire, et ce qu'on ne veut pas lui apprendre a demander.
///
/// La connexion refait ce que fait le demarrage: elle relit le fichier, ouvre
/// chaque serveur, et enregistre ses outils. Les abeilles de ressources sont
/// reposees avec la nouvelle table de clients, sans quoi elles continueraient de
/// repondre « aucun serveur connecte » en tenant une table vide capturee au
/// lancement.
pub struct ReloadMcpTool {
    pub registry: Arc<AbeilleRegistry>,
}

#[async_trait]
impl Abeille for ReloadMcpTool {
    fn nom(&self) -> &str {
        "reload_mcp"
    }

    fn description(&self) -> &str {
        "Reload mcp_servers.json and CONNECT to the servers it lists, without restarting \
         LaRuche. Call this immediately after mcp_add or mcp_remove: registering a server \
         only writes it to the config, it does not connect to it, and its tools stay \
         unavailable until this runs. Returns how many tools became available."
    }

    fn schema(&self) -> serde_json::Value {
        json!({ "type": "object", "properties": {}, "additionalProperties": false })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        _args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let chemin = ctx.working_dir.join("mcp_servers.json");
        let (nb, clients) =
            crate::mcp_client::charger_mcp_servers(&chemin, &self.registry).await;
        let noms: Vec<String> = clients.keys().cloned().collect();
        let clients = Arc::new(clients);
        self.registry
            .enregistrer(Box::new(crate::abeilles::mcp_resources::McpListResources {
                clients: clients.clone(),
            }));
        self.registry
            .enregistrer(Box::new(crate::abeilles::mcp_resources::McpReadResource {
                clients,
            }));
        if noms.is_empty() {
            return Ok(ResultatAbeille::ok(
                "No MCP server connected. Check mcp_servers.json: a server that fails its \
                 handshake is skipped, and its command must be runnable from the home folder."
                    .to_string(),
            ));
        }
        Ok(ResultatAbeille::ok(format!(
            "{nb} tool(s) available from {} MCP server(s): {}.",
            noms.len(),
            noms.join(", ")
        )))
    }
}
