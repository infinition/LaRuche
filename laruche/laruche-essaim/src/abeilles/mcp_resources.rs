use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use crate::mcp_client::McpClient;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

pub struct McpListResources {
    pub clients: Arc<HashMap<String, McpClient>>,
}

#[async_trait]
impl Abeille for McpListResources {
    fn nom(&self) -> &str {
        "list_mcp_resources"
    }

    fn description(&self) -> &str {
        "Liste les ressources disponibles depuis tous les serveurs MCP connectés. Affiche leur URI, nom, et description."
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
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        if self.clients.is_empty() {
            return Ok(ResultatAbeille::ok("Aucun serveur MCP n'est connecté."));
        }

        let mut output = String::new();
        for (server_name, client) in self.clients.iter() {
            output.push_str(&format!("--- Serveur MCP: {} ---\n", server_name));
            match client.list_resources().await {
                Ok(resources) => {
                    if resources.is_empty() {
                        output.push_str("Aucune ressource disponible.\n");
                    } else {
                        for res in resources {
                            output.push_str(&format!(
                                "- URI: {}\n  Nom: {}\n  Type: {}\n  Description: {}\n",
                                res.uri, res.name, res.mime_type, res.description
                            ));
                        }
                    }
                }
                Err(e) => {
                    output.push_str(&format!("Erreur lors de la récupération: {}\n", e));
                }
            }
        }

        Ok(ResultatAbeille::ok(output))
    }
}

pub struct McpReadResource {
    pub clients: Arc<HashMap<String, McpClient>>,
}

#[async_trait]
impl Abeille for McpReadResource {
    fn nom(&self) -> &str {
        "read_mcp_resource"
    }

    fn description(&self) -> &str {
        "Lit le contenu d'une ressource MCP spécifique en utilisant son URI. Pour trouver l'URI, utiliser list_mcp_resources d'abord."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "server_name": {
                    "type": "string",
                    "description": "Nom du serveur MCP hébergeant la ressource"
                },
                "uri": {
                    "type": "string",
                    "description": "L'URI de la ressource à lire"
                }
            },
            "required": ["server_name", "uri"],
            "additionalProperties": false
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
        let server_name = args
            .get("server_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let uri = args.get("uri").and_then(|v| v.as_str()).unwrap_or("");

        if server_name.is_empty() || uri.is_empty() {
            return Ok(ResultatAbeille::err("server_name et uri sont requis"));
        }

        if let Some(client) = self.clients.get(server_name) {
            match client.read_resource(uri).await {
                Ok(data) => {
                    let mut text_output = String::new();
                    if let Some(contents) = data.get("contents").and_then(|c| c.as_array()) {
                        for item in contents {
                            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                text_output.push_str(text);
                                text_output.push('\n');
                            }
                        }
                    } else {
                        text_output = serde_json::to_string_pretty(&data)?;
                    }
                    Ok(ResultatAbeille::ok(text_output))
                }
                Err(e) => Ok(ResultatAbeille::err(format!("Erreur: {}", e))),
            }
        } else {
            Ok(ResultatAbeille::err(format!(
                "Serveur MCP '{}' introuvable.",
                server_name
            )))
        }
    }
}
