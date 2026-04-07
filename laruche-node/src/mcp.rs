//! MCP (Model Context Protocol) Server for LaRuche
//!
//! Exposes LaRuche's Abeilles (tools) as MCP tools for external AI clients
//! like Claude Desktop, Cursor, etc.
//!
//! Protocol: JSON-RPC 2.0 over stdio or HTTP POST.
//!
//! Supported methods:
//!   - `initialize` — handshake, returns server capabilities
//!   - `tools/list` — list all registered Abeilles as MCP tools
//!   - `tools/call` — execute an Abeille by name

use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use laruche_essaim::{AbeilleRegistry, ContextExecution};

// ======================== JSON-RPC Types ========================

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<serde_json::Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

// ======================== MCP Tool Schema ========================

#[derive(Debug, Serialize)]
struct McpToolInfo {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: serde_json::Value,
}

// ======================== Handler Dispatch ========================

/// Handle a JSON-RPC request and dispatch to the appropriate MCP method.
pub async fn handle_mcp_request(
    registry: &AbeilleRegistry,
    req: JsonRpcRequest,
) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => handle_initialize(req.id),
        "tools/list" => handle_tools_list(registry, req.id),
        "tools/call" => handle_tools_call(registry, req.id, req.params).await,
        "notifications/initialized" => {
            // Client acknowledgment — no response needed for notifications,
            // but since we may receive it via HTTP, return empty success
            JsonRpcResponse::success(req.id, serde_json::json!({}))
        }
        _ => JsonRpcResponse::error(
            req.id,
            -32601,
            format!("Method not found: {}", req.method),
        ),
    }
}

// ======================== MCP Methods ========================

fn handle_initialize(id: Option<serde_json::Value>) -> JsonRpcResponse {
    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "laruche-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

fn handle_tools_list(
    registry: &AbeilleRegistry,
    id: Option<serde_json::Value>,
) -> JsonRpcResponse {
    let tools: Vec<McpToolInfo> = registry
        .noms()
        .into_iter()
        .filter_map(|name| {
            let abeille = registry.get(name)?;
            Some(McpToolInfo {
                name: name.to_string(),
                description: abeille.description().to_string(),
                input_schema: abeille.schema(),
            })
        })
        .collect();

    JsonRpcResponse::success(id, serde_json::json!({ "tools": tools }))
}

async fn handle_tools_call(
    registry: &AbeilleRegistry,
    id: Option<serde_json::Value>,
    params: serde_json::Value,
) -> JsonRpcResponse {
    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => {
            return JsonRpcResponse::error(id, -32602, "Missing 'name' parameter");
        }
    };

    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let ctx = ContextExecution::default();

    match registry.executer(&name, arguments, &ctx).await {
        Ok(result) => {
            let content = if result.success {
                serde_json::json!([{
                    "type": "text",
                    "text": result.output
                }])
            } else {
                serde_json::json!([{
                    "type": "text",
                    "text": result.error.unwrap_or_else(|| "Unknown error".into())
                }])
            };

            JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "content": content,
                    "isError": !result.success
                }),
            )
        }
        Err(e) => JsonRpcResponse::error(id, -32000, format!("Tool execution failed: {}", e)),
    }
}

// ======================== Axum HTTP Handler ========================

/// Axum handler for POST /api/mcp — accepts JSON-RPC requests.
pub async fn api_mcp_handler(
    State(state): State<Arc<super::AppState>>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let response = handle_mcp_request(&state.essaim_registry, req).await;
    Json(response)
}

// ======================== Stdio Server ========================

/// Run the MCP server over stdio (for Claude Desktop integration).
/// Reads JSON-RPC messages from stdin (one per line), writes responses to stdout.
#[allow(dead_code)]
pub async fn run_mcp_stdio(registry: Arc<AbeilleRegistry>) {
    use std::io::{BufRead, BufReader};

    let stdin = BufReader::new(std::io::stdin());
    let stdout = std::io::stdout();

    eprintln!("LaRuche MCP server started (stdio mode)");

    for line in stdin.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("MCP: stdin read error: {}", e);
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = JsonRpcResponse::error(None, -32700, format!("Parse error: {}", e));
                let json = serde_json::to_string(&err_resp).unwrap_or_default();
                {
                    let mut out = stdout.lock();
                    let _ = std::io::Write::write_all(&mut out, json.as_bytes());
                    let _ = std::io::Write::write_all(&mut out, b"\n");
                    let _ = std::io::Write::flush(&mut out);
                }
                continue;
            }
        };

        let response = handle_mcp_request(&registry, req).await;
        let json = serde_json::to_string(&response).unwrap_or_default();
        {
            let mut out = stdout.lock();
            let _ = std::io::Write::write_all(&mut out, json.as_bytes());
            let _ = std::io::Write::write_all(&mut out, b"\n");
            let _ = std::io::Write::flush(&mut out);
        }
    }
}
