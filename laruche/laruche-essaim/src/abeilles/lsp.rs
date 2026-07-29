use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::{Context, Result};
use async_trait::async_trait;
use lazy_static::lazy_static;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot, Mutex};

#[derive(Clone)]
pub struct LspClient {
    tx: mpsc::Sender<(Value, oneshot::Sender<Result<Value>>)>,
    next_id: Arc<AtomicU64>,
}

impl LspClient {
    pub async fn start(cmd: &str, args: &[&str], root_uri: &str) -> Result<Self> {
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to spawn LSP server")?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        let (tx, mut rx) = mpsc::channel::<(Value, oneshot::Sender<Result<Value>>)>(100);
        let pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value>>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let pending_clone = pending_requests.clone();

        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some((req, reply_tx)) = rx.recv().await {
                if let Some(id) = req.get("id").and_then(|i| i.as_u64()) {
                    pending_clone.lock().await.insert(id, reply_tx);
                }
                let json_str = serde_json::to_string(&req).unwrap();
                let message = format!("Content-Length: {}\r\n\r\n{}", json_str.len(), json_str);
                if stdin.write_all(message.as_bytes()).await.is_err() {
                    break;
                }
            }
        });

        let pending_clone2 = pending_requests.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).await.is_err() || line.is_empty() {
                    break;
                }

                if line.starts_with("Content-Length: ") {
                    let len_str = line.trim_start_matches("Content-Length: ").trim();
                    if let Ok(len) = len_str.parse::<usize>() {
                        // Read empty line
                        let mut empty = String::new();
                        let _ = reader.read_line(&mut empty).await;

                        // Read content
                        let mut content = vec![0; len];
                        if reader.read_exact(&mut content).await.is_err() {
                            break;
                        }

                        if let Ok(resp) = serde_json::from_slice::<Value>(&content) {
                            if let Some(id) = resp.get("id").and_then(|i| i.as_u64()) {
                                if let Some(reply_tx) = pending_clone2.lock().await.remove(&id) {
                                    if let Some(error) = resp.get("error") {
                                        let _ = reply_tx
                                            .send(Err(anyhow::anyhow!("LSP Error: {:?}", error)));
                                    } else {
                                        let _ = reply_tx.send(Ok(resp
                                            .get("result")
                                            .unwrap_or(&Value::Null)
                                            .clone()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let client = Self {
            tx,
            next_id: Arc::new(AtomicU64::new(1)),
        };

        // Initialize LSP
        let params = json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {}
        });

        client.send_request("initialize", params).await?;
        client.send_notification("initialized", json!({})).await?;

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
        reply_rx.await?
    }

    pub async fn send_notification(&self, method: &str, params: Value) -> Result<()> {
        let req = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        let (reply_tx, _) = oneshot::channel();
        self.tx.send((req, reply_tx)).await?;
        Ok(())
    }
}

lazy_static! {
    static ref GLOBAL_LSP_CLIENTS: Mutex<HashMap<String, LspClient>> = Mutex::new(HashMap::new());
}

pub struct AbeilleLsp;

#[async_trait]
impl Abeille for AbeilleLsp {
    fn nom(&self) -> &str {
        "lsp"
    }

    fn description(&self) -> &str {
        "Provides language intelligence like 'goToDefinition', 'findReferences', and 'hover'. Supported servers: rust-analyzer, tsserver (typescript-language-server)."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["goToDefinition", "findReferences", "hover", "documentSymbol"],
                    "description": "LSP operation to perform"
                },
                "file": {
                    "type": "string",
                    "description": "Absolute path to the source file"
                },
                "line": {
                    "type": "integer",
                    "description": "Line number (0-indexed)"
                },
                "character": {
                    "type": "integer",
                    "description": "Character position (0-indexed)"
                }
            },
            "required": ["operation", "file", "line", "character"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let operation = args["operation"].as_str().unwrap_or("hover");
        let file = args["file"].as_str().unwrap_or("");
        let line = args["line"].as_u64().unwrap_or(0);
        let character = args["character"].as_u64().unwrap_or(0);

        let file_path = std::path::Path::new(file);
        let ext = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");

        let (cmd, ls_args) = match ext {
            "rs" => ("rust-analyzer", vec![]),
            "ts" | "js" | "tsx" | "jsx" => ("typescript-language-server", vec!["--stdio"]),
            "py" => ("pyright-langserver", vec!["--stdio"]),
            _ => {
                return Ok(ResultatAbeille::err(format!(
                    "No LSP server configured for extension '{}'",
                    ext
                )))
            }
        };

        let root_uri = format!(
            "file:///{}",
            ctx.working_dir.display().to_string().replace("\\", "/")
        );
        let file_uri = format!(
            "file:///{}",
            file_path.display().to_string().replace("\\", "/")
        );

        let client = {
            let mut clients = GLOBAL_LSP_CLIENTS.lock().await;
            if let Some(c) = clients.get(ext) {
                c.clone()
            } else {
                let c = LspClient::start(cmd, &ls_args, &root_uri).await?;
                clients.insert(ext.to_string(), c.clone());
                c
            }
        };

        // Open the document in LSP
        let file_content = std::fs::read_to_string(file_path).unwrap_or_default();
        let _ = client
            .send_notification(
                "textDocument/didOpen",
                json!({
                    "textDocument": {
                        "uri": file_uri,
                        "languageId": ext,
                        "version": 1,
                        "text": file_content
                    }
                }),
            )
            .await;

        let pos = json!({ "line": line, "character": character });
        let req_params = json!({
            "textDocument": { "uri": file_uri },
            "position": pos
        });

        let (method, final_params) = match operation {
            "goToDefinition" => ("textDocument/definition", req_params),
            "findReferences" => (
                "textDocument/references",
                json!({
                    "textDocument": { "uri": file_uri },
                    "position": pos,
                    "context": { "includeDeclaration": true }
                }),
            ),
            "hover" => ("textDocument/hover", req_params),
            "documentSymbol" => (
                "textDocument/documentSymbol",
                json!({
                    "textDocument": { "uri": file_uri }
                }),
            ),
            _ => return Ok(ResultatAbeille::err("Unknown operation")),
        };

        let result = client.send_request(method, final_params).await?;
        Ok(ResultatAbeille::ok(serde_json::to_string_pretty(&result)?))
    }
}
