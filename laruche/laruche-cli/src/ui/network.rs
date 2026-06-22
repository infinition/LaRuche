use super::app::TuiEvent;
use super::app::dirs_config_path;

/// Discover a LaRuche server: LARUCHE_URL env → mDNS → localhost probe.
async fn discover_server() -> String {
    // 1. Explicit URL
    if let Ok(url) = std::env::var("LARUCHE_URL") {
        if probe_server(&url).await {
            return url;
        }
    }

    // 2. mDNS discovery via laruche-client
    if let Ok(lr) = laruche_client::LaRuche::discover().await {
        for node in lr.nodes() {
            if let Some(url) = node.manifest.api_url() {
                if probe_server(&url).await {
                    return url;
                }
            }
        }
    }

    // 3. Localhost probe
    let local = "http://127.0.0.1:8419".to_string();
    if probe_server(&local).await {
        return local;
    }

    String::new()
}

async fn probe_server(url: &str) -> bool {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .ok()
        .and_then(|c| {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    c.get(format!("{}/health", url))
                        .send()
                        .await
                        .ok()
                        .map(|r| r.status().is_success())
                })
            })
        })
        .unwrap_or(false)
}

async fn fetch_model(url: &str) -> String {
    if url.is_empty() {
        return std::env::var("LARUCHE_MODEL").unwrap_or_else(|_| "?".into());
    }
    reqwest::Client::new()
        .get(format!("{}/models", url))
        .send()
        .await
        .ok()
        .and_then(|r| {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(r.json::<serde_json::Value>())
            })
            .ok()
        })
        .and_then(|d| {
            d["models"]
                .as_array()?
                .first()?
                .get("name")?
                .as_str()
                .map(String::from)
        })
        .unwrap_or_else(|| "?".into())
}

async fn fetch_tools(url: &str) -> Vec<String> {
    if url.is_empty() {
        return vec![];
    }
    reqwest::Client::new()
        .get(format!("{}/api/tools", url))
        .send()
        .await
        .ok()
        .and_then(|r| {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(r.json::<serde_json::Value>())
            })
            .ok()
        })
        .and_then(|d| {
            d.as_array().map(|a| {
                a.iter()
                    .filter_map(|t| t["name"].as_str().map(String::from))
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// Connect to ws://{server}/ws/chat, send the message, and stream TuiEvents
/// back through the channel. Falls back to POST /api/webhook on WS failure.
async fn stream_via_websocket(
    url: String,
    text: String,
    model: String,
    auth_token: Option<String>,
    _session_id: Option<String>,
    tx: tokio::sync::mpsc::Sender<TuiEvent>,
) {
    // Build the WebSocket URL: http://host:port -> ws://host:port/ws/chat
    let ws_url = format!(
        "ws://{}/ws/chat",
        url.trim_start_matches("https://")
            .trim_start_matches("http://")
    );

    // Build WS request with auth cookie if available
    let ws_request = if let Some(ref token) = auth_token {
        tokio_tungstenite::tungstenite::http::Request::builder()
            .uri(&ws_url)
            .header("Cookie", format!("laruche_auth={}", token))
            .header(
                "Host",
                url.trim_start_matches("https://")
                    .trim_start_matches("http://"),
            )
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .body(())
            .unwrap()
    } else {
        tokio_tungstenite::tungstenite::http::Request::builder()
            .uri(&ws_url)
            .header(
                "Host",
                url.trim_start_matches("https://")
                    .trim_start_matches("http://"),
            )
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .body(())
            .unwrap()
    };

    // Try WebSocket first
    match tokio_tungstenite::connect_async(ws_request).await {
        Ok((ws_stream, _)) => {
            let (mut write, mut read) = ws_stream.split();

            // Send the message
            let payload = serde_json::json!({ "type": "message", "text": text, "model": model });
            if let Err(e) = write
                .send(WsMessage::Text(payload.to_string().into()))
                .await
            {
                let _ = tx
                    .send(TuiEvent::Error(format!("WS send error: {}", e)))
                    .await;
                let _ = tx.send(TuiEvent::Done(String::new())).await;
                return;
            }

            let mut full_response = String::new();

            // Read events from the stream
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(WsMessage::Text(raw)) => {
                        let text_str: &str = raw.as_ref();
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(text_str) {
                            let event_type = data["type"].as_str().unwrap_or("");
                            match event_type {
                                "token" => {
                                    let tok = data["text"].as_str().unwrap_or("").to_string();
                                    full_response.push_str(&tok);
                                    let _ = tx.send(TuiEvent::Token(tok)).await;
                                }
                                "tool_call" => {
                                    let name = data["name"].as_str().unwrap_or("?").to_string();
                                    let args = data["args"]
                                        .as_str()
                                        .or_else(|| data["arguments"].as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let _ = tx.send(TuiEvent::ToolCall { name, args }).await;
                                }
                                "tool_result" => {
                                    let name = data["name"].as_str().unwrap_or("?").to_string();
                                    let success = data["success"].as_bool().unwrap_or(true);
                                    let ms = data["elapsed_ms"]
                                        .as_u64()
                                        .or_else(|| data["ms"].as_u64())
                                        .unwrap_or(0);
                                    let _ =
                                        tx.send(TuiEvent::ToolResult { name, success, ms }).await;
                                }
                                "plan" => {
                                    let steps: Vec<(String, String)> = data["steps"]
                                        .as_array()
                                        .map(|arr| {
                                            arr.iter()
                                                .map(|s| {
                                                    let task = s["task"]
                                                        .as_str()
                                                        .unwrap_or("?")
                                                        .to_string();
                                                    let status = s["status"]
                                                        .as_str()
                                                        .unwrap_or("pending")
                                                        .to_string();
                                                    (task, status)
                                                })
                                                .collect()
                                        })
                                        .unwrap_or_default();
                                    let _ = tx.send(TuiEvent::Plan(steps)).await;
                                }
                                "thinking" => {
                                    let thought = data["text"].as_str().unwrap_or("").to_string();
                                    let _ = tx.send(TuiEvent::Thinking(thought)).await;
                                }
                                "done" => {
                                    // The server may send the full response in "text"
                                    let final_text = data["text"]
                                        .as_str()
                                        .map(|s| s.to_string())
                                        .unwrap_or(full_response.clone());
                                    let _ = tx.send(TuiEvent::Done(final_text)).await;
                                    return;
                                }
                                "error" => {
                                    let err = data["text"]
                                        .as_str()
                                        .or_else(|| data["message"].as_str())
                                        .unwrap_or("Unknown error")
                                        .to_string();
                                    let _ = tx.send(TuiEvent::Error(err)).await;
                                    let _ = tx.send(TuiEvent::Done(full_response.clone())).await;
                                    return;
                                }
                                _ => {
                                    // Unknown event type — if it has text, treat as token
                                    if let Some(t) = data["text"].as_str() {
                                        full_response.push_str(t);
                                        let _ = tx.send(TuiEvent::Token(t.to_string())).await;
                                    }
                                }
                            }
                        }
                    }
                    Ok(WsMessage::Close(_)) => {
                        // Server closed the connection — finalize
                        let _ = tx.send(TuiEvent::Done(full_response)).await;
                        return;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(TuiEvent::Error(format!("WS read error: {}", e)))
                            .await;
                        let _ = tx.send(TuiEvent::Done(full_response)).await;
                        return;
                    }
                    _ => {} // Ping/Pong/Binary — ignore
                }
            }

            // Stream ended without explicit done
            let _ = tx.send(TuiEvent::Done(full_response)).await;
        }
        Err(ws_err) => {
            // WebSocket connection failed — fall back to HTTP POST /api/webhook
            let _ = tx
                .send(TuiEvent::Thinking(format!(
                    "WS failed ({}), falling back to HTTP...",
                    ws_err
                )))
                .await;
            fallback_http_send(&url, &text, &tx).await;
        }
    }
}

/// Fallback: POST /api/webhook and send the result as a single Done event.
async fn fallback_http_send(url: &str, text: &str, tx: &tokio::sync::mpsc::Sender<TuiEvent>) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap();

    let body = serde_json::json!({ "prompt": text });
    let resp = client
        .post(format!("{}/api/webhook", url))
        .json(&body)
        .send()
        .await;

    match resp {
        Ok(r) => {
            if let Ok(data) = r.json::<serde_json::Value>().await {
                if let Some(err) = data["error"].as_str() {
                    if !err.is_empty() {
                        let _ = tx
                            .send(TuiEvent::Error(format!("Erreur serveur: {}", err)))
                            .await;
                        let _ = tx.send(TuiEvent::Done(String::new())).await;
                        return;
                    }
                }
                let response = data["response"].as_str().unwrap_or("").to_string();
                // Clean tool_call/plan tags
                let mut clean = response.clone();
                while let Some(s) = clean.find("<tool_call>") {
                    if let Some(e) = clean.find("</tool_call>") {
                        clean = format!("{}{}", &clean[..s], &clean[e + "</tool_call>".len()..]);
                    } else {
                        clean.truncate(s);
                        break;
                    }
                }
                while let Some(s) = clean.find("<plan>") {
                    if let Some(e) = clean.find("</plan>") {
                        clean = format!("{}{}", &clean[..s], &clean[e + "</plan>".len()..]);
                    } else {
                        clean.truncate(s);
                        break;
                    }
                }
                // Extract tool info for activity log
                if let Some(tools) = data["tools_used"].as_array() {
                    for t in tools {
                        let name = t["name"].as_str().unwrap_or("?").to_string();
                        let ms = t["elapsed_ms"].as_u64().unwrap_or(0);
                        let ok = t["success"].as_bool().unwrap_or(true);
                        let _ = tx
                            .send(TuiEvent::ToolResult {
                                name,
                                success: ok,
                                ms,
                            })
                            .await;
                    }
                }
                let _ = tx.send(TuiEvent::Done(clean.trim().to_string())).await;
            } else {
                let _ = tx
                    .send(TuiEvent::Error("Error parsing response".into()))
                    .await;
                let _ = tx.send(TuiEvent::Done(String::new())).await;
            }
        }
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("HTTP error: {}", e))).await;
            let _ = tx.send(TuiEvent::Done(String::new())).await;
        }
    }
}

