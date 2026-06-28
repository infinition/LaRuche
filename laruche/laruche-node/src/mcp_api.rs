//! MCP server registry endpoints (list, save, delete configured MCP servers) - split out of main.rs.

use crate::*;
use axum::extract::State;
use std::sync::Arc;

/// GET /api/mcp/servers
pub(crate) async fn api_mcp_list_servers(
    State(_state): State<Arc<AppState>>,
) -> axum::Json<serde_json::Value> {
    let path = std::path::Path::new("mcp_servers.json");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                return axum::Json(json);
            }
        }
    }
    axum::Json(serde_json::json!({ "mcpServers": {} }))
}

/// POST /api/mcp/servers/:name
pub(crate) async fn api_mcp_save_server(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> axum::Json<serde_json::Value> {
    let command = body["command"].as_str().unwrap_or("").to_string();
    let mut args = vec![];
    if let Some(args_arr) = body["args"].as_array() {
        for a in args_arr {
            if let Some(s) = a.as_str() {
                args.push(s.to_string());
            }
        }
    }

    let path = std::path::Path::new("mcp_servers.json");
    let mut servers: laruche_essaim::mcp_client::McpServersFile = if path.exists() {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_else(|_| {
            laruche_essaim::mcp_client::McpServersFile {
                mcpServers: std::collections::HashMap::new(),
            }
        })
    } else {
        laruche_essaim::mcp_client::McpServersFile {
            mcpServers: std::collections::HashMap::new(),
        }
    };

    servers.mcpServers.insert(
        name.clone(),
        laruche_essaim::mcp_client::McpServerConfig { command, args },
    );

    if let Ok(json) = serde_json::to_string_pretty(&servers) {
        let _ = std::fs::write(path, json);
    }

    // Reload all MCP tools
    state
        .essaim_registry
        .supprimer_par_origine(laruche_essaim::abeille::ToolOrigin::Mcp);
    let _ = laruche_essaim::mcp_client::charger_mcp_servers(path, &state.essaim_registry).await;

    axum::Json(serde_json::json!({ "status": "ok", "name": name }))
}

/// DELETE /api/mcp/servers/:name
pub(crate) async fn api_mcp_delete_server(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> axum::Json<serde_json::Value> {
    let path = std::path::Path::new("mcp_servers.json");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(mut servers) =
                serde_json::from_str::<laruche_essaim::mcp_client::McpServersFile>(&content)
            {
                servers.mcpServers.remove(&name);
                if let Ok(json) = serde_json::to_string_pretty(&servers) {
                    let _ = std::fs::write(path, json);
                }
            }
        }
    }

    // Reload all MCP tools
    state
        .essaim_registry
        .supprimer_par_origine(laruche_essaim::abeille::ToolOrigin::Mcp);
    let _ = laruche_essaim::mcp_client::charger_mcp_servers(path, &state.essaim_registry).await;

    axum::Json(serde_json::json!({ "status": "deleted", "name": name }))
}
