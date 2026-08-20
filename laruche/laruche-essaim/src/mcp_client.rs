use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceDef {
    pub uri: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, rename = "mimeType")]
    pub mime_type: String,
}

/// Upper bound on any single MCP round-trip. The engine already applies its own
/// per-tool timeout on top; this one protects the boot path (initialize/list_tools)
/// from a server that spawns but never answers the handshake.
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

#[derive(Clone)]
pub struct McpClient {
    tx: mpsc::Sender<(Value, oneshot::Sender<Result<Value>>)>,
    next_id: Arc<AtomicU64>,
}

impl McpClient {
    pub async fn start(cmd: &str, args: &[&str]) -> Result<Self> {
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn MCP server")?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // Reap the server process when it exits, otherwise it lingers as a
        // zombie (Unix). The server is meant to outlive this function, so the
        // child is parked in its own task rather than stored on the client.
        tokio::spawn(async move {
            match child.wait().await {
                Ok(status) => tracing::debug!("MCP server exited: {status}"),
                Err(e) => tracing::debug!("MCP server wait failed: {e}"),
            }
        });

        // Log stderr
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            while reader.read_line(&mut line).await.is_ok() && !line.is_empty() {
                tracing::debug!("MCP STDERR: {}", line.trim_end());
                line.clear();
            }
        });

        let (tx, mut rx) = mpsc::channel::<(Value, oneshot::Sender<Result<Value>>)>(100);
        let pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value>>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let pending_clone = pending_requests.clone();

        // Writer task
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some((req, reply_tx)) = rx.recv().await {
                if let Some(id) = req.get("id").and_then(|i| i.as_u64()) {
                    pending_clone.lock().await.insert(id, reply_tx);
                }
                let mut json_str = serde_json::to_string(&req).unwrap();
                json_str.push('\n');
                if let Err(e) = stdin.write_all(json_str.as_bytes()).await {
                    tracing::error!("Failed to write to MCP server: {}", e);
                    break;
                }
            }
        });

        let pending_clone2 = pending_requests.clone();

        // Reader task
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            while reader.read_line(&mut line).await.is_ok() && !line.is_empty() {
                if let Ok(resp) = serde_json::from_str::<Value>(&line) {
                    if let Some(id) = resp.get("id").and_then(|i| i.as_u64()) {
                        if let Some(reply_tx) = pending_clone2.lock().await.remove(&id) {
                            if let Some(error) = resp.get("error") {
                                let _ = reply_tx.send(Err(anyhow::anyhow!("MCP Error: {}", error)));
                            } else {
                                let _ = reply_tx
                                    .send(Ok(resp.get("result").unwrap_or(&Value::Null).clone()));
                            }
                        }
                    } else if let Some(method) = resp.get("method") {
                        // Notification
                        tracing::debug!("MCP Notification: {}", method);
                    }
                }
                line.clear();
            }
        });

        let client = Self {
            tx,
            next_id: Arc::new(AtomicU64::new(1)),
        };

        // Initialize
        client.initialize().await?;

        Ok(client)
    }

    /// Connects to a REMOTE MCP server over HTTP instead of launching a local process.
    ///
    /// The client only ever talks through its channel, so a second transport costs one
    /// worker task: `list_tools`, `call_tool` and the handshake are untouched. Until now
    /// the only way in was `Command::spawn`, which meant a server had to live on this
    /// machine; a hosted one, or one running on another box, could not be used at all.
    ///
    /// Each request is one POST carrying the JSON-RPC body, plus the three things the
    /// streamable HTTP transport requires and a bare POST does not provide:
    ///
    /// 1. **The session.** A server may answer `initialize` with an `Mcp-Session-Id`
    ///    header, and then refuse every later request that does not carry it back. This
    ///    is not optional politeness: Unreal Engine 5.8's own MCP server accepts our
    ///    `initialize`, then answers `tools/list` with a bare 400, so the server was
    ///    registered, listed zero tools, and the editor looked like it exposed nothing.
    /// 2. **Both content types in `Accept`.** The spec lets a server reply as SSE, and
    ///    several refuse a request that only announces `application/json`.
    /// 3. **SSE bodies.** When the answer comes back as `text/event-stream`, the payload
    ///    sits on a `data:` line, and parsing the raw body as JSON fails on it.
    ///
    /// The session lives in the worker task, which is the only place that speaks to the
    /// wire, and requests are serialized through the channel, so the handshake response
    /// is always seen before the next request goes out.
    pub async fn start_http(url: &str) -> Result<Self> {
        let (tx, mut rx) = mpsc::channel::<(Value, oneshot::Sender<Result<Value>>)>(32);
        let url = url.to_string();
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .context("http client for remote MCP")?;

        tokio::spawn(async move {
            let mut session: Option<String> = None;
            while let Some((req, reply)) = rx.recv().await {
                let mut envoi = http
                    .post(&url)
                    .header("Accept", "application/json, text/event-stream")
                    .json(&req);
                if let Some(id) = &session {
                    envoi = envoi.header("Mcp-Session-Id", id);
                }
                let resultat = match envoi.send().await {
                    Ok(rep) => {
                        if let Some(id) = rep
                            .headers()
                            .get("mcp-session-id")
                            .and_then(|v| v.to_str().ok())
                            .filter(|v| !v.is_empty())
                        {
                            session = Some(id.to_string());
                        }
                        let statut = rep.status();
                        let corps = rep.text().await.unwrap_or_default();
                        if !statut.is_success() {
                            // The body carries the reason when there is one, and an empty
                            // body IS the reason on a session rejection. Either way the
                            // status must reach the log, which used to see only
                            // "invalid MCP response".
                            Err(anyhow::anyhow!(
                                "remote MCP answered {statut}: {}",
                                corps.chars().take(200).collect::<String>()
                            ))
                        } else {
                            charge_utile(&corps)
                        }
                    }
                    Err(e) => Err(anyhow::anyhow!("remote MCP unreachable: {e}")),
                };
                let _ = reply.send(resultat);
            }
        });

        let client = Self {
            tx,
            next_id: Arc::new(AtomicU64::new(1)),
        };
        client.initialize().await?;
        Ok(client)
    }

    pub async fn send_request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx.send((req, reply_tx)).await?;
        match tokio::time::timeout(REQUEST_TIMEOUT, reply_rx).await {
            Ok(reply) => reply?,
            Err(_) => Err(anyhow::anyhow!(
                "MCP request '{method}' timed out after {}s",
                REQUEST_TIMEOUT.as_secs()
            )),
        }
    }

    pub async fn send_notification(&self, method: &str, params: Value) -> Result<()> {
        let req = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        // We don't care about the response for notifications, so we just drop the receiver side of the oneshot.
        // But the writer task expects an id to store it. We won't provide an id for notifications.
        let (reply_tx, _) = oneshot::channel();
        self.tx.send((req, reply_tx)).await?;
        Ok(())
    }

    async fn initialize(&self) -> Result<()> {
        let params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "roots": { "listChanged": true },
                "sampling": {}
            },
            "clientInfo": {
                "name": "LaRuche",
                "version": "0.1.0"
            }
        });

        let res = self.send_request("initialize", params).await?;
        tracing::info!("MCP server initialized: {:?}", res.get("serverInfo"));

        // Send initialized notification
        self.send_notification("notifications/initialized", json!({}))
            .await?;
        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<McpToolDef>> {
        let res = self.send_request("tools/list", json!({})).await?;
        let tools = res
            .get("tools")
            .and_then(|t| t.as_array())
            .context("Missing tools array")?;

        let mut defs = Vec::new();
        for t in tools {
            let mut t_val = t.clone();
            if let Some(t_obj) = t_val.as_object_mut() {
                // Rename inputSchema to input_schema for Rust struct mapping if needed
                if let Some(schema) = t_obj.remove("inputSchema") {
                    t_obj.insert("input_schema".to_string(), schema);
                }
                match serde_json::from_value(serde_json::Value::Object(t_obj.clone())) {
                    Ok(def) => defs.push(def),
                    Err(e) => tracing::warn!("Skipping malformed MCP tool definition: {e}"),
                }
            }
        }
        Ok(defs)
    }

    pub async fn list_resources(&self) -> Result<Vec<McpResourceDef>> {
        let res = self.send_request("resources/list", json!({})).await?;
        let resources = res
            .get("resources")
            .and_then(|r| r.as_array())
            .context("Missing resources array")?;

        let mut defs = Vec::new();
        for r in resources {
            if let Ok(def) = serde_json::from_value(r.clone()) {
                defs.push(def);
            }
        }
        Ok(defs)
    }

    pub async fn read_resource(&self, uri: &str) -> Result<Value> {
        self.send_request(
            "resources/read",
            json!({
                "uri": uri,
            }),
        )
        .await
    }

    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value> {
        self.send_request(
            "tools/call",
            json!({
                "name": name,
                "arguments": args
            }),
        )
        .await
    }
}

/// Read the JSON-RPC payload of an HTTP response, whether it is a plain JSON body or an
/// SSE stream where the document sits on a `data:` line.
///
/// A JSON-RPC error is turned into an Err here rather than handed up as a result: every
/// caller treated a payload carrying `error` as a success otherwise.
fn charge_utile(corps: &str) -> Result<Value> {
    let brut = corps.trim();
    let document = if brut.starts_with('{') || brut.starts_with('[') {
        brut.to_string()
    } else {
        brut.lines()
            .filter_map(|l| l.strip_prefix("data:"))
            .map(str::trim)
            .find(|l| !l.is_empty() && *l != "[DONE]")
            .map(str::to_string)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "unreadable MCP response: {}",
                    brut.chars().take(200).collect::<String>()
                )
            })?
    };
    let valeur: Value = serde_json::from_str(&document)
        .with_context(|| format!("invalid MCP response: {}", document.chars().take(200).collect::<String>()))?;
    if let Some(err) = valeur.get("error") {
        return Err(anyhow::anyhow!("MCP Error: {err}"));
    }
    Ok(valeur.get("result").unwrap_or(&Value::Null).clone())
}

use crate::abeille::AbeilleRegistry;
use crate::abeilles::mcp_tool::McpAbeille;

#[derive(Serialize, Deserialize, Clone)]
pub struct McpServerConfig {
    /// Command to launch, for a LOCAL server managed by LaRuche. Empty when `url` is set.
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Endpoint of a REMOTE server, which LaRuche talks to without launching anything.
    /// Mutually exclusive with `command`: whichever is filled decides the transport.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// A disabled server is configured but never started, so its tools stay out of the
    /// registry. Absent from a file written before this field existed, which is why it
    /// defaults to enabled: an upgrade must not silently switch a server off.
    #[serde(default = "actif_par_defaut")]
    pub enabled: bool,
}

fn actif_par_defaut() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
pub struct McpServersFile {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

pub async fn charger_mcp_servers(
    config_path: &std::path::Path,
    registry: &AbeilleRegistry,
) -> (usize, HashMap<String, McpClient>) {
    let mut count = 0;
    let mut clients = HashMap::new();
    if !config_path.exists() {
        return (0, clients);
    }

    let content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(_) => return (0, clients),
    };

    let servers: McpServersFile = match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Failed to parse {}: {}", config_path.display(), e);
            return (0, clients);
        }
    };

    for (name, config) in servers.mcp_servers {
        if !config.enabled {
            tracing::info!(server = %name, "MCP server disabled, not started");
            continue;
        }
        tracing::info!(
            "Starting MCP server '{}': {} {:?}",
            name,
            config.command,
            config.args
        );

        let args_ref: Vec<&str> = config.args.iter().map(|s| s.as_str()).collect();
        // A url means a remote server: nothing to spawn, we just talk to it.
        let ouverture = match config.url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
            Some(url) => McpClient::start_http(url).await,
            None => McpClient::start(&config.command, &args_ref).await,
        };
        match ouverture {
            Ok(client) => {
                clients.insert(name.clone(), client.clone());
                match client.list_tools().await {
                    Ok(tools) => {
                        for def in tools {
                            tracing::info!("Registered MCP tool: {}", def.name);
                            let abeille = McpAbeille {
                                client: client.clone(),
                                def,
                            };
                            registry.enregistrer(Box::new(abeille));
                            count += 1;
                        }
                    }
                    Err(e) => tracing::warn!("Failed to list tools for MCP {}: {}", name, e),
                }
            }
            Err(e) => tracing::warn!("Failed to start MCP {}: {}", name, e),
        }
    }

    (count, clients)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A server answering in SSE must be understood, and the streamable HTTP transport
    /// lets any server choose that form at any time.
    #[test]
    fn une_reponse_sse_est_lue_comme_du_json() {
        let corps = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[]}}\n\n";
        let r = charge_utile(corps).expect("payload readable");
        assert!(r["tools"].is_array());
    }

    #[test]
    fn une_reponse_json_simple_passe_aussi() {
        let r = charge_utile("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"ok\":true}}").unwrap();
        assert_eq!(r["ok"], true);
    }

    /// A JSON-RPC error used to be returned as a successful result, so a refused call
    /// looked like an empty answer.
    #[test]
    fn une_erreur_jsonrpc_ne_passe_pas_pour_un_succes() {
        let e = charge_utile("{\"jsonrpc\":\"2.0\",\"id\":1,\"error\":{\"code\":-32602,\"message\":\"Unknown tool\"}}")
            .unwrap_err();
        assert!(e.to_string().contains("Unknown tool"));
    }

    #[test]
    fn un_corps_illisible_le_dit() {
        assert!(charge_utile("<html>gateway</html>").is_err());
    }
}
