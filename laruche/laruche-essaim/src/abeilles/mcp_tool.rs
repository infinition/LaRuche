use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille, ToolOrigin};
use crate::mcp_client::{McpClient, McpToolDef};
use anyhow::Result;
use async_trait::async_trait;

pub struct McpAbeille {
    pub client: McpClient,
    pub def: McpToolDef,
}

#[async_trait]
impl Abeille for McpAbeille {
    fn nom(&self) -> &str {
        &self.def.name
    }

    fn origin(&self) -> ToolOrigin {
        ToolOrigin::Mcp
    }

    fn description(&self) -> &str {
        &self.def.description
    }

    fn schema(&self) -> serde_json::Value {
        self.def.input_schema.clone()
    }

    fn niveau_danger(&self) -> NiveauDanger {
        // By default, assume MCP tools might need approval unless we parse some special tag.
        // For safety, let's treat them as NeedsApproval, or maybe Safe if we trust the MCP server.
        NiveauDanger::NeedsApproval
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let response = self.client.call_tool(&self.def.name, args).await?;

        // MCP tools/call returns:
        // { "content": [ { "type": "text", "text": "..." } ], "isError": false }
        let mut text_output = String::new();
        let mut images = Vec::new();
        let is_error = response
            .get("isError")
            .and_then(|b| b.as_bool())
            .unwrap_or(false);

        if let Some(content) = response.get("content").and_then(|c| c.as_array()) {
            for item in content {
                if let Some(type_str) = item.get("type").and_then(|t| t.as_str()) {
                    if type_str == "image" {
                        if let Some(data) = item.get("data").and_then(|d| d.as_str()) {
                            images.push(data.to_string());
                            text_output.push_str("[Image attachée]\n");
                        }
                    } else if type_str == "text" {
                        if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                            text_output.push_str(text);
                            text_output.push('\n');
                        }
                    }
                } else if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    text_output.push_str(text);
                    text_output.push('\n');
                }
            }
        } else {
            text_output = serde_json::to_string_pretty(&response)?;
        }

        if is_error {
            Ok(ResultatAbeille {
                success: false,
                output: text_output.clone(),
                error: Some(text_output),
                metadata: None,
                cwd_change: None,
                images,
            })
        } else {
            Ok(ResultatAbeille {
                success: true,
                output: text_output,
                error: None,
                metadata: None,
                cwd_change: None,
                images,
            })
        }
    }
}
