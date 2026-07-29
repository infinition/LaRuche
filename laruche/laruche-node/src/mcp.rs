//! MCP (Model Context Protocol) Server for LaRuche
//!
//! Exposes LaRuche's Abeilles (tools) as MCP tools for external AI clients
//! like Claude Desktop, Cursor, etc.
//!
//! Protocol: JSON-RPC 2.0 over stdio or HTTP POST.
//!
//! Supported methods:
//!   - `initialize`: handshake, returns server capabilities
//!   - `tools/list`: list all registered Abeilles as MCP tools
//!   - `tools/call`: execute an Abeille by name

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
    desactives: &[String],
    req: JsonRpcRequest,
) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => handle_initialize(req.id),
        "tools/list" => handle_tools_list(registry, desactives, req.id),
        "tools/call" => handle_tools_call(registry, desactives, req.id, req.params).await,
        "notifications/initialized" => {
            // Client acknowledgment, no response needed for notifications,
            // but since we may receive it via HTTP, return empty success
            JsonRpcResponse::success(req.id, serde_json::json!({}))
        }
        _ => JsonRpcResponse::error(req.id, -32601, format!("Method not found: {}", req.method)),
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

/// A tool the user switched off in Settings > Tools is off EVERYWHERE.
///
/// MCP used to serve the whole registry and never consult `disabled_tools`, so a tool
/// deliberately disabled for the chat stayed listed and callable by any authorised
/// external client — including the shell. One switch, one meaning.
fn est_desactive(desactives: &[String], nom: &str) -> bool {
    desactives.iter().any(|t| t == nom)
}

fn handle_tools_list(
    registry: &AbeilleRegistry,
    desactives: &[String],
    id: Option<serde_json::Value>,
) -> JsonRpcResponse {
    let tools: Vec<McpToolInfo> = registry
        .noms()
        .into_iter()
        .filter(|name| !est_desactive(desactives, name))
        .filter_map(|name| {
            let abeille = registry.get(&name)?;
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
    desactives: &[String],
    id: Option<serde_json::Value>,
    params: serde_json::Value,
) -> JsonRpcResponse {
    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => {
            return JsonRpcResponse::error(id, -32602, "Missing 'name' parameter");
        }
    };
    // Hiding it from tools/list is not enough: a client that already knows the name, or
    // cached an older listing, would still reach it.
    if est_desactive(desactives, &name) {
        return JsonRpcResponse::error(
            id,
            -32601,
            format!("Tool '{name}' is disabled in this LaRuche (Settings > Tools)"),
        );
    }

    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    // Belt and braces with the check above: the guard in `executer` is the one nobody
    // can route around, so the list travels with the call.
    let ctx = ContextExecution {
        disabled_tools: desactives.to_vec(),
        ..Default::default()
    };

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

/// Axum handler for POST /api/mcp: accepts JSON-RPC requests.
pub async fn api_mcp_handler(
    State(state): State<Arc<super::AppState>>,
    headers: axum::http::HeaderMap,
    connect_info: Option<axum::extract::ConnectInfo<std::net::SocketAddr>>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    // Same door as `/mcp`: opt-in server, IP allowlist, token, ban on repeated refusals.
    // This surface used to check only the on/off switch, so it accepted calls from any
    // address with no token at all while its twin demanded one.
    let ip = connect_info.as_ref().map(|ci| ci.0.ip());
    let jeton = headers
        .get("x-laruche-mcp-token")
        .and_then(|v| v.to_str().ok());
    let outil = req.params["name"].as_str().map(|s| s.to_string());
    let methode = req.method.clone();
    if let Err(refus) = crate::mcp_pare_feu::controler(&state, ip, jeton).await {
        crate::mcp_pare_feu::journaliser(
            &state, ip, "/api/mcp", &methode, outil.as_deref(), Some(&refus), None,
        )
        .await;
        return Json(JsonRpcResponse::error(req.id, -32601, refus.message()));
    }
    crate::mcp_pare_feu::journaliser(
        &state, ip, "/api/mcp", &methode, outil.as_deref(), None, None,
    )
    .await;
    let _garde = match &outil {
        Some(nom) => {
            let cfg = state.essaim_config.read().await.clone();
            Some(crate::ouvrir_travail(
                &state,
                "mcp",
                nom,
                &cfg,
                ip.map(|a| a.to_string()),
            ))
        }
        None => None,
    };
    let desactives = state.essaim_config.read().await.disabled_tools.clone();
    let response = handle_mcp_request(&state.essaim_registry, &desactives, req).await;
    Json(response)
}

// ======================== Stdio Server ========================

/// Run the MCP server over stdio (for Claude Desktop integration).
/// Reads JSON-RPC messages from stdin (one per line), writes responses to stdout.
#[allow(dead_code)]
pub async fn run_mcp_stdio(registry: Arc<AbeilleRegistry>, desactives: Vec<String>) {
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

        let response = handle_mcp_request(&registry, &desactives, req).await;
        let json = serde_json::to_string(&response).unwrap_or_default();
        {
            let mut out = stdout.lock();
            let _ = std::io::Write::write_all(&mut out, json.as_bytes());
            let _ = std::io::Write::write_all(&mut out, b"\n");
            let _ = std::io::Write::flush(&mut out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tool switched off in Settings > Tools must be off for MCP too. It used to be
    /// listed AND callable by any authorised external client, the shell included: one
    /// switch that meant two different things depending on the door you came through.
    #[test]
    fn un_outil_desactive_est_filtre_de_la_liste_et_refuse_a_lappel() {
        let desactives = vec!["shell_exec".to_string(), "web_fetch".to_string()];

        // tools/list hides them.
        let registre = ["memory_read_node", "shell_exec", "web_fetch", "memory_write"];
        let listes: Vec<&str> = registre
            .iter()
            .copied()
            .filter(|n| !est_desactive(&desactives, n))
            .collect();
        assert_eq!(listes, vec!["memory_read_node", "memory_write"]);

        // tools/call refuses them: hiding alone would not stop a client that already
        // knows the name or cached an older listing.
        assert!(est_desactive(&desactives, "shell_exec"));
        assert!(!est_desactive(&desactives, "memory_write"));
    }

    /// Nothing disabled must not silently filter anything out.
    #[test]
    fn sans_outil_desactive_la_liste_est_complete() {
        let aucun: Vec<String> = Vec::new();
        assert!(!est_desactive(&aucun, "shell_exec"));
    }
}
