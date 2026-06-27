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
        "List all resources available across connected MCP servers. Returns URI, name, MIME type, and description for each resource."
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
            return Ok(ResultatAbeille::ok("No MCP server is connected."));
        }

        let mut output = String::new();
        for (server_name, client) in self.clients.iter() {
            output.push_str(&format!("--- MCP Server: {} ---\n", server_name));
            match client.list_resources().await {
                Ok(resources) => {
                    if resources.is_empty() {
                        output.push_str("No resources available.\n");
                    } else {
                        for res in resources {
                            output.push_str(&format!(
                                "- URI: {}\n  Name: {}\n  Type: {}\n  Description: {}\n",
                                res.uri, res.name, res.mime_type, res.description
                            ));
                        }
                    }
                }
                Err(e) => {
                    output.push_str(&format!("Error fetching resources: {}\n", e));
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
        "Read the content of a specific MCP resource by URI. Use list_mcp_resources first to discover available URIs."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "server_name": {
                    "type": "string",
                    "description": "Name of the MCP server hosting the resource"
                },
                "uri": {
                    "type": "string",
                    "description": "The URI of the resource to read"
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
            return Ok(ResultatAbeille::err("server_name and uri are required"));
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
                Err(e) => Ok(ResultatAbeille::err(format!("Error: {}", e))),
            }
        } else {
            Ok(ResultatAbeille::err(format!(
                "MCP server '{}' not found.",
                server_name
            )))
        }
    }
}
