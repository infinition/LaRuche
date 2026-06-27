//! Multi-provider LLM streaming abstraction.
//!
//! Supports:
//! - **ollama** (default): local Ollama instance
//! - **openai**: OpenAI-compatible APIs (Deepseek, Together, Groq, etc.)
//! - **anthropic**: Anthropic Claude API
//!
//! Tous les providers supportent le **tool calling natif** (format OpenAI `tools:`)
//! quand un tableau d'outils est fourni. Le parser accumule les `tool_calls` des
//! chunks streaming et les livre sur le dernier chunk.

use crate::brain::ToolCall;
use crate::streaming::{ollama_chat_stream, OllamaChunk};
use anyhow::Result;
use futures_util::Stream;
use std::pin::Pin;
use tokio_stream::wrappers::ReceiverStream;

/// Erreur provider structurée (code HTTP + corps) renvoyée sur réponse non-2xx.
#[derive(Debug, Clone)]
pub struct ProviderError {
    pub status: u16,
    pub body: String,
    pub retry_after: Option<String>,
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Provider API error {}: {}", self.status, self.body)
    }
}

impl std::error::Error for ProviderError {}

/// Convertit le format d'outil LaRuche (name, description, parameters)
/// vers le format OpenAI `tools` (type: function, function: {name, description, parameters}).
pub fn convertir_tools_openai(tools: &[serde_json::Value]) -> Vec<serde_json::Value> {
    tools.iter().filter_map(|t| {
        let name = t["name"].as_str()?;
        let description = t["description"].as_str().unwrap_or("");
        let parameters = t.get("parameters").cloned().unwrap_or(serde_json::json!({}));
        Some(serde_json::json!({
            "type": "function",
            "function": {
                "name": name,
                "description": description,
                "parameters": parameters,
            }
        }))
    }).collect()
}

/// Unified streaming entry point — dispatches to the correct provider.
pub async fn provider_chat_stream(
    provider: &str,
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
    api_key: &str,
    api_base: Option<&str>,
    ollama_url: &str,
    tools: Option<&[serde_json::Value]>,  // ← nouveau paramètre
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    match provider {
        "openai" | "miel" => {
            openai_chat_stream(model, messages, temperature, max_tokens, api_key, api_base, tools).await
        }
        "anthropic" => {
            anthropic_chat_stream(model, messages, temperature, max_tokens, api_key, api_base, tools).await
        }
        "codex" => codex_chat_stream(model, messages, temperature, max_tokens, api_base).await,
        _ => ollama_chat_stream(ollama_url, model, messages, temperature, max_tokens, tools).await,
    }
}

// ─── Signer mesh ────────────────────────────────────────────────────────────
pub type MeshSigner = std::sync::Arc<dyn Fn(&str) -> Vec<(String, String)> + Send + Sync>;
static MESH_SIGNER: std::sync::OnceLock<MeshSigner> = std::sync::OnceLock::new();
pub fn set_mesh_signer(s: MeshSigner) { let _ = MESH_SIGNER.set(s); }
fn mesh_headers(path: &str) -> Vec<(String, String)> {
    MESH_SIGNER.get().map(|s| s(path)).unwrap_or_default()
}

// ─── OpenAI-compatible streaming (Deepseek, Together, Groq, etc.) ────────────

async fn openai_chat_stream(
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
    api_key: &str,
    api_base: Option<&str>,
    tools: Option<&[serde_json::Value]>,
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    let api_key = api_key.trim();
    let base = normalize_base_url(api_base.unwrap_or("https://api.openai.com"));
    let base = base.as_str();
    if api_key.is_empty() && !is_local_base_url(base) {
        anyhow::bail!("API key is required for OpenAI-compatible provider. Configure in Settings > Providers.");
    }
    let bearer = if api_key.is_empty() { "local-no-key" } else { api_key };
    let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));

    let openai_messages: Vec<serde_json::Value> = messages.iter().map(|m| {
        let attachments_val = m.get("attachments").and_then(|a| a.as_array());
        let has_attachments = attachments_val.map(|a| !a.is_empty()).unwrap_or(false);
        if has_attachments {
            let mut parts = vec![serde_json::json!({"type": "text", "text": m["content"].as_str().unwrap_or("")})];
            for att in attachments_val.unwrap() {
                let kind = att["kind"].as_str().unwrap_or("");
                let mime_type = att["mime_type"].as_str().unwrap_or("");
                let data = att["data"].as_str().unwrap_or("");
                if kind == "image" {
                    parts.push(serde_json::json!({"type": "image_url", "image_url": {
                        "url": format!("data:{};base64,{}", mime_type, data)
                    }}));
                } else if kind == "audio" {
                    let format = match mime_type { "audio/wav" | "audio/x-wav" => "wav", _ => "wav" };
                    parts.push(serde_json::json!( {
                        "type": "input_audio",
                        "input_audio": {"data": data, "format": format}
                    }));
                }
            }
            serde_json::json!({"role": m["role"], "content": parts})
        } else {
            serde_json::json!({"role": m["role"], "content": m["content"].as_str().unwrap_or("")})
        }
    }).collect();

    let mut body = serde_json::json!({
        "model": model,
        "messages": openai_messages,
        "stream": true,
        "temperature": temperature,
    });
    if max_tokens > 0 {
        body["max_tokens"] = serde_json::json!(max_tokens);
    }
    // Envoyer les définitions d'outils natifs (OpenAI format)
    if let Some(tools_list) = tools {
        let openai_tools = convertir_tools_openai(tools_list);
        if !openai_tools.is_empty() {
            body["tools"] = serde_json::json!(openai_tools);
        }
    }

    let client = reqwest::Client::new();
    let mut req = client.post(&url)
        .header("Authorization", format!("Bearer {}", bearer))
        .header("Content-Type", "application/json");
    if is_local_base_url(base) {
        for (k, v) in mesh_headers("/v1/chat/completions") { req = req.header(k, v); }
    }
    let mut response = req.json(&body).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        return Err(ProviderError { status: status.as_u16(), body: body_text, retry_after: None }.into());
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<OllamaChunk>(64);

    tokio::spawn(async move {
        let mut buffer = String::new();
        // Accumulateur de tool_calls indexé par index (delta streaming)
        // Chaque entrée : (id, name, partial_args_string)
        let mut tool_call_acc: std::collections::HashMap<u32, (String, String, String)> = std::collections::HashMap::new();

        loop {
            match response.chunk().await {
                Ok(Some(bytes)) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                    while let Some(newline_pos) = buffer.find('\n') {
                        let line = buffer[..newline_pos].trim().to_string();
                        buffer = buffer[newline_pos + 1..].to_string();
                        if line.is_empty() || line == "data: [DONE]" {
                            if line == "data: [DONE]" {
                                // Finaliser les tool_calls accumulés
                                let tool_calls = if tool_call_acc.is_empty() {
                                    None
                                } else {
                                    let mut calls: Vec<ToolCall> = tool_call_acc.iter()
                                        .map(|(_, (id, name, args_str))| ToolCall {
                                            id: id.clone(),
                                            name: name.clone(),
                                            args: serde_json::from_str(args_str).unwrap_or(serde_json::Value::Null),
                                        })
                                        .collect();
                                    calls.sort_by_key(|c| c.id.clone());
                                    Some(calls)
                                };
                                let _ = tx.send(OllamaChunk {
                                    text: String::new(), done: true,
                                    finish_reason: Some("stop".to_string()),
                                    eval_count: None, eval_duration: None,
                                    prompt_eval_count: None,
                                    tool_calls,
                                }).await;
                                return;
                            }
                            continue;
                        }
                        let json_str = if let Some(stripped) = line.strip_prefix("data: ") { stripped } else { &line };
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                            let text = parsed["choices"][0]["delta"]["content"].as_str().unwrap_or("").to_string();
                            let finish_reason = parsed["choices"][0]["finish_reason"].as_str().map(str::to_string);
                            let done = finish_reason.is_some();

                            // Parser les tool_calls delta (format OpenAI streaming)
                            if let Some(tc_deltas) = parsed["choices"][0]["delta"]["tool_calls"].as_array() {
                                for tc_delta in tc_deltas {
                                    let idx = tc_delta["index"].as_u64().unwrap_or(0) as u32;
                                    let is_id = tc_delta.get("id").and_then(|v| v.as_str()).is_some();
                                    let entry = tool_call_acc.entry(idx).or_insert_with(|| {
                                        (String::new(), String::new(), String::new())
                                    });
                                    // id: présent seulement sur le premier chunk du tool call
                                    if let Some(id_val) = tc_delta["id"].as_str() {
                                        entry.0 = id_val.to_string();
                                    }
                                    if entry.0.is_empty() {
                                        entry.0 = format!("call_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x"));
                                    }
                                    // function.name: présent sur le premier chunk
                                    if let Some(name_val) = tc_delta["function"]["name"].as_str() {
                                        entry.1 = name_val.to_string();
                                    }
                                    // function.arguments: concaténé sur plusieurs chunks
                                    if let Some(args_val) = tc_delta["function"]["arguments"].as_str() {
                                        entry.2.push_str(args_val);
                                    }
                                }
                            }

                            if !text.is_empty() || done {
                                // Envoyer les tool_calls accumulés uniquement sur le dernier chunk
                                let tool_calls = if done && !tool_call_acc.is_empty() {
                                    let mut calls: Vec<ToolCall> = tool_call_acc.iter()
                                        .map(|(_, (id, name, args_str))| ToolCall {
                                            id: id.clone(),
                                            name: name.clone(),
                                            args: serde_json::from_str(args_str).unwrap_or(serde_json::Value::Null),
                                        })
                                        .collect();
                                    calls.sort_by_key(|c| c.id.clone());
                                    Some(calls)
                                } else { None };

                                let chunk = OllamaChunk {
                                    text, done, finish_reason,
                                    eval_count: None, eval_duration: None,
                                    prompt_eval_count: None,
                                    tool_calls,
                                };
                                if tx.send(chunk).await.is_err() { return; }
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => { tracing::error!(error = %e, "Error reading OpenAI stream"); return; }
            }
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}

// ─── Anthropic (Claude) streaming ──────────────────────────────────────────

async fn anthropic_chat_stream(
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
    api_key: &str,
    api_base: Option<&str>,
    tools: Option<&[serde_json::Value]>,
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    let api_key = api_key.trim();
    let base = normalize_base_url(api_base.unwrap_or("https://api.anthropic.com"));
    let url = format!("{}/v1/messages", base.trim_end_matches('/'));

    let anthropic_max: u32 = if max_tokens > 0 { max_tokens } else { 4096 };

    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "max_tokens": anthropic_max,
        "temperature": temperature,
    });

    // Anthropic supporte aussi le tool calling natif, format légèrement différent
    if let Some(tools_list) = tools {
        let anthropic_tools: Vec<serde_json::Value> = tools_list.iter().filter_map(|t| {
            Some(serde_json::json!({
                "name": t["name"].as_str()?,
                "description": t["description"].as_str().unwrap_or(""),
                "input_schema": t.get("parameters").cloned().unwrap_or(serde_json::json!({})),
            }))
        }).collect();
        if !anthropic_tools.is_empty() {
            body["tools"] = serde_json::json!(anthropic_tools);
        }
    }

    // ... le reste du code Anthropic reste identique
    _anthropic_send_request(&url, api_key, body).await
}

async fn _anthropic_send_request(
    url: &str,
    api_key: &str,
    body: serde_json::Value,
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    let client = reqwest::Client::new();
    let mut response = client.post(url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        return Err(ProviderError { status: status.as_u16(), body: body_text, retry_after: None }.into());
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<OllamaChunk>(64);

    tokio::spawn(async move {
        let mut buffer = String::new();
        loop {
            match response.chunk().await {
                Ok(Some(bytes)) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                    while let Some(newline_pos) = buffer.find('\n') {
                        let line = buffer[..newline_pos].trim().to_string();
                        buffer = buffer[newline_pos + 1..].to_string();
                        if line.is_empty() { continue; }

                        // Anthropix SSE: event: ..., data: {...}
                        if let Some(data) = line.strip_prefix("data: ") {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                                let chunk_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                let text = match chunk_type {
                                    "content_block_delta" => parsed["delta"]["text"].as_str().unwrap_or("").to_string(),
                                    _ => String::new(),
                                };
                                let done = chunk_type == "message_stop";
                                let finish_reason = if done { Some("stop".to_string()) } else { None };

                                if !text.is_empty() || done {
                                    let _ = tx.send(OllamaChunk {
                                        text, done, finish_reason,
                                        eval_count: None, eval_duration: None,
                                        prompt_eval_count: None,
                                        tool_calls: None,
                                    }).await;
                                }
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => { tracing::error!(error = %e, "Error reading Anthropic stream"); return; }
            }
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}

// ─── Codex (ChatGPT) ────────────────────────────────────────────────────────

async fn codex_chat_stream(
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
    api_base: Option<&str>,
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    use crate::codex_auth;
    let _ = (temperature, max_tokens);
    let access_token = codex_auth::resolve_codex_access_token()
        .await.map_err(|e| anyhow::anyhow!("Auth Codex: {e}"))?;
    let base = match api_base.map(|b| b.trim_end_matches('/').to_string()) {
        Some(b) if b.contains("backend-api/codex") => b,
        _ => codex_auth::DEFAULT_CODEX_BASE_URL.to_string(),
    };
    let url = format!("{}/responses", base);

    let mut instructions = String::new();
    let mut input: Vec<serde_json::Value> = Vec::new();
    for m in messages {
        match m["role"].as_str().unwrap_or("user") {
            "system" => instructions.push_str(&format!("{}\n", m["content"].as_str().unwrap_or(""))),
            role => {
                let entry = serde_json::json!({
                    "role": if role == "assistant" { "assistant" } else { "user" },
                    "content": m["content"].as_str().unwrap_or(""),
                });
                input.push(entry);
            }
        }
    }

    let mut body = serde_json::json!({
        "model": model,
        "input": input,
        "stream": true,
    });
    if !instructions.trim().is_empty() {
        body["instructions"] = serde_json::json!(instructions.trim());
    }

    let client = reqwest::Client::new();
    let mut req = client.post(&url)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Content-Type", "application/json");
    for (k, v) in mesh_headers("/responses") { req = req.header(k, v); }
    let mut response = req.json(&body).send().await?;

    let (tx, rx) = tokio::sync::mpsc::channel::<OllamaChunk>(64);
    tokio::spawn(async move {
        let mut buffer = String::new();
        loop {
            match response.chunk().await {
                Ok(Some(bytes)) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                    while let Some(newline_pos) = buffer.find('\n') {
                        let line = buffer[..newline_pos].trim().to_string();
                        buffer = buffer[newline_pos + 1..].to_string();
                        if line.is_empty() { continue; }
                        if let Some(data) = line.strip_prefix("data: ") {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                                let ctype = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                let text = match ctype {
                                    "response.output_text.delta" => parsed["delta"].as_str().unwrap_or("").to_string(),
                                    _ => String::new(),
                                };
                                let done = ctype == "response.completed" || ctype == "response.incomplete";
                                if !text.is_empty() || done {
                                    let _ = tx.send(OllamaChunk {
                                        text, done,
                                        finish_reason: if done { Some("stop".to_string()) } else { None },
                                        eval_count: None, eval_duration: None,
                                        prompt_eval_count: None,
                                        tool_calls: None,
                                    }).await;
                                }
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => { tracing::error!(error = %e, "Error reading Codex stream"); return; }
            }
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn normalize_base_url(url: &str) -> String {
    let url = url.trim();
    if url.is_empty() { return "https://api.openai.com".to_string(); }
    let lower = url.to_lowercase();
    // Handle "localhost" addresses by keeping http:// if explicitly set
    if lower.starts_with("http://") || lower.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    }
}

fn is_local_base_url(url: &str) -> bool {
    let u = url.to_lowercase();
    u.contains("localhost") || u.contains("127.0.0.1") || u.contains("::1") || u.contains(".local")
}
