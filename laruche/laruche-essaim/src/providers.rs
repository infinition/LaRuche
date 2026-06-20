//! Multi-provider LLM streaming abstraction.
//!
//! Supports:
//! - **ollama** (default): local Ollama instance
//! - **openai**: OpenAI-compatible APIs (OpenAI, Together, Groq, LM Studio, etc.)
//! - **anthropic**: Anthropic Claude API

use crate::streaming::{ollama_chat_stream, OllamaChunk};
use anyhow::Result;
use futures_util::Stream;
use std::pin::Pin;
use tokio_stream::wrappers::ReceiverStream;

/// Erreur provider structurée (code HTTP + corps) renvoyée sur réponse non-2xx.
/// Permet au failover (`brain.rs`) de classer l'erreur via `error_classifier`
/// (429 → RateLimited, 401/403 → ReloginRequired, etc.) plutôt que de parser
/// une chaîne. Wrappée dans `anyhow` → récupérable par `downcast_ref`.
#[derive(Debug, Clone)]
pub struct ProviderError {
    pub status: u16,
    pub body: String,
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Provider API error {}: {}", self.status, self.body)
    }
}

impl std::error::Error for ProviderError {}

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
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    match provider {
        // "miel" = node LaRuche distant exposé en OpenAI-compatible (passerelle mesh).
        "openai" | "miel" => {
            openai_chat_stream(model, messages, temperature, max_tokens, api_key, api_base).await
        }
        "anthropic" => {
            anthropic_chat_stream(model, messages, temperature, max_tokens, api_key, api_base).await
        }
        "codex" => codex_chat_stream(model, messages, temperature, max_tokens, api_base).await,
        // Default: "ollama" or anything else
        _ => ollama_chat_stream(ollama_url, model, messages, temperature, max_tokens).await,
    }
}

// ─── OpenAI-compatible streaming ────────────────────────────────────────────

async fn openai_chat_stream(
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
    api_key: &str,
    api_base: Option<&str>,
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    let base = api_base.unwrap_or("https://api.openai.com");
    if api_key.is_empty() && !is_loopback_base_url(base) {
        anyhow::bail!("API key is required for OpenAI-compatible provider. Configure in Settings > Providers.");
    }
    let bearer = if api_key.is_empty() {
        "local-no-key"
    } else {
        api_key
    };
    let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));

    // Convert messages: ensure role/content only (strip Ollama-specific fields)
    let openai_messages: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| {
            let attachments_val = m.get("attachments").and_then(|a| a.as_array());
            let has_attachments = attachments_val.map(|a| !a.is_empty()).unwrap_or(false);

            if has_attachments {
                let mut parts = vec![serde_json::json!({
                    "type": "text",
                    "text": m["content"].as_str().unwrap_or("")
                })];
                for att in attachments_val.unwrap() {
                    let kind = att["kind"].as_str().unwrap_or("");
                    let mime_type = att["mime_type"].as_str().unwrap_or("");
                    let data = att["data"].as_str().unwrap_or("");

                    if kind == "image" {
                        parts.push(serde_json::json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:{};base64,{}", mime_type, data)
                            }
                        }));
                    } else if kind == "audio" {
                        let format = match mime_type {
                            "audio/wav" | "audio/x-wav" => "wav",
                            "audio/mp3" | "audio/mpeg" => "mp3",
                            _ => "wav",
                        };
                        parts.push(serde_json::json!({
                            "type": "input_audio",
                            "input_audio": {
                                "data": data,
                                "format": format
                            }
                        }));
                    }
                }
                serde_json::json!({
                    "role": m["role"].as_str().unwrap_or("user"),
                    "content": parts
                })
            } else {
                serde_json::json!({
                    "role": m["role"].as_str().unwrap_or("user"),
                    "content": m["content"].as_str().unwrap_or("")
                })
            }
        })
        .collect();

    let body = serde_json::json!({
        "model": model,
        "messages": openai_messages,
        "stream": true,
        "temperature": temperature,
        "max_tokens": max_tokens,
    });

    let client = reqwest::Client::new();
    let mut response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", bearer))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ProviderError {
            status: status.as_u16(),
            body,
        }
        .into());
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

                        if line.is_empty() || line == "data: [DONE]" {
                            if line == "data: [DONE]" {
                                let _ = tx
                                    .send(OllamaChunk {
                                        text: String::new(),
                                        done: true,
                                        finish_reason: Some("stop".to_string()),
                                        eval_count: None,
                                        eval_duration: None,
                                    })
                                    .await;
                                return;
                            }
                            continue;
                        }

                        // SSE format: "data: {...}"
                        let json_str = if let Some(stripped) = line.strip_prefix("data: ") {
                            stripped
                        } else {
                            &line
                        };

                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                            // OpenAI format: choices[0].delta.content
                            let text = parsed["choices"][0]["delta"]["content"]
                                .as_str()
                                .unwrap_or("")
                                .to_string();

                            let finish_reason = parsed["choices"][0]["finish_reason"]
                                .as_str()
                                .map(str::to_string);
                            let done = finish_reason.is_some();

                            if !text.is_empty() || done {
                                let chunk = OllamaChunk {
                                    text,
                                    done,
                                    finish_reason,
                                    eval_count: None,
                                    eval_duration: None,
                                };
                                if tx.send(chunk).await.is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    tracing::error!(error = %e, "Error reading OpenAI stream");
                    return;
                }
            }
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}

// ─── ChatGPT Codex (abonnement, OAuth) — Responses API ──────────────────────

/// Streaming via le backend ChatGPT Codex (`chatgpt.com/backend-api/codex`).
///
/// Utilise l'abonnement ChatGPT (pas une clé API) : les credentials sont résolus
/// au moment de l'appel depuis `~/.laruche/auth.json` (refresh auto si expiré).
/// On parle la **Responses API** (`/responses`), pas chat/completions, avec les
/// en-têtes anti-Cloudflare (`originator`, `ChatGPT-Account-ID`).
async fn codex_chat_stream(
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
    api_base: Option<&str>,
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    use crate::codex_auth;

    let _ = (temperature, max_tokens); // Non utilisés par la Responses API Codex.

    // 1. Résoudre un access token utilisable (refresh auto, fallback CLI Codex).
    let access_token = codex_auth::resolve_codex_access_token()
        .await
        .map_err(|e| anyhow::anyhow!("Auth Codex: {e}"))?;

    // Le backend Codex est imposé : on n'honore un api_base custom que s'il
    // pointe bien vers un backend Codex, sinon on retombe sur le défaut (un
    // profil mal configuré — base_url vide ou pointant ailleurs — ne casse pas).
    let base = match api_base.map(|b| b.trim_end_matches('/').to_string()) {
        Some(b) if b.contains("backend-api/codex") => b,
        _ => codex_auth::DEFAULT_CODEX_BASE_URL.to_string(),
    };
    let url = format!("{}/responses", base);

    // 2. Construire le body Responses API : on extrait le system prompt en
    //    `instructions`, le reste devient `input` (rôle/contenu).
    let mut instructions = String::new();
    let mut input: Vec<serde_json::Value> = Vec::new();
    for m in messages {
        let role = m["role"].as_str().unwrap_or("user");
        let content = m["content"].as_str().unwrap_or("");
        if role == "system" {
            if !instructions.is_empty() {
                instructions.push_str("\n\n");
            }
            instructions.push_str(content);
        } else {
            input.push(serde_json::json!({ "role": role, "content": content }));
        }
    }

    let mut body = serde_json::json!({
        "model": model,
        "input": input,
        "stream": true,
        "store": false,
    });
    if !instructions.is_empty() {
        body["instructions"] = serde_json::Value::String(instructions);
    }

    // 3. En-têtes anti-Cloudflare + Bearer.
    let client = reqwest::Client::new();
    let mut req = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream");
    for (k, v) in codex_auth::codex_headers(&access_token) {
        req = req.header(k, v);
    }
    let mut response = req.json(&body).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ProviderError {
            status: status.as_u16(),
            body,
        }
        .into());
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

                        if line.is_empty() {
                            continue;
                        }
                        // SSE Responses API : lignes "event: ..." puis "data: {...}".
                        let json_str = match line.strip_prefix("data: ") {
                            Some(s) => s,
                            None => continue, // ignore "event:" et autres
                        };
                        if json_str == "[DONE]" {
                            let _ = tx
                                .send(OllamaChunk {
                                    text: String::new(),
                                    done: true,
                                    finish_reason: Some("stop".to_string()),
                                    eval_count: None,
                                    eval_duration: None,
                                })
                                .await;
                            return;
                        }
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                            let event_type = parsed["type"].as_str().unwrap_or("");
                            match event_type {
                                "response.output_text.delta" => {
                                    let text = parsed["delta"].as_str().unwrap_or("").to_string();
                                    if !text.is_empty() {
                                        if tx
                                            .send(OllamaChunk {
                                                text,
                                                done: false,
                                                finish_reason: None,
                                                eval_count: None,
                                                eval_duration: None,
                                            })
                                            .await
                                            .is_err()
                                        {
                                            return;
                                        }
                                    }
                                }
                                "response.completed" => {
                                    let _ = tx
                                        .send(OllamaChunk {
                                            text: String::new(),
                                            done: true,
                                            finish_reason: Some("stop".to_string()),
                                            eval_count: None,
                                            eval_duration: None,
                                        })
                                        .await;
                                    return;
                                }
                                "response.failed" | "error" => {
                                    let msg = parsed["response"]["error"]["message"]
                                        .as_str()
                                        .or_else(|| parsed["error"]["message"].as_str())
                                        .or_else(|| parsed["message"].as_str())
                                        .unwrap_or("Codex stream error");
                                    tracing::error!(error = %msg, "Codex stream error");
                                    let _ = tx
                                        .send(OllamaChunk {
                                            text: String::new(),
                                            done: true,
                                            finish_reason: Some("error".to_string()),
                                            eval_count: None,
                                            eval_duration: None,
                                        })
                                        .await;
                                    return;
                                }
                                _ => {} // response.created, output_item.*, reasoning.*, etc.
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    tracing::error!(error = %e, "Error reading Codex stream");
                    return;
                }
            }
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}

fn is_loopback_base_url(base: &str) -> bool {
    let lower = base.trim().to_ascii_lowercase();
    lower.starts_with("http://127.")
        || lower.starts_with("https://127.")
        || lower.starts_with("http://localhost")
        || lower.starts_with("https://localhost")
        || lower.starts_with("http://[::1]")
        || lower.starts_with("https://[::1]")
}

// ─── Anthropic Claude streaming ─────────────────────────────────────────────

async fn anthropic_chat_stream(
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
    api_key: &str,
    api_base: Option<&str>,
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    if api_key.is_empty() {
        anyhow::bail!(
            "API key is required for Anthropic provider. Configure in Settings > Providers."
        );
    }
    let base = api_base.unwrap_or("https://api.anthropic.com");
    let url = format!("{}/v1/messages", base.trim_end_matches('/'));

    // Separate system message from user/assistant messages
    let mut system_text = String::new();
    let mut anthropic_messages: Vec<serde_json::Value> = Vec::new();

    for m in messages {
        let role = m["role"].as_str().unwrap_or("user");
        let content = m["content"].as_str().unwrap_or("");
        let attachments_val = m.get("attachments").and_then(|a| a.as_array());
        let has_attachments = attachments_val.map(|a| !a.is_empty()).unwrap_or(false);

        if role == "system" {
            system_text.push_str(content);
        } else {
            if has_attachments {
                let mut parts = vec![serde_json::json!({
                    "type": "text",
                    "text": content
                })];
                for att in attachments_val.unwrap() {
                    let kind = att["kind"].as_str().unwrap_or("");
                    let mime_type = att["mime_type"].as_str().unwrap_or("");
                    let data = att["data"].as_str().unwrap_or("");

                    if kind == "image" {
                        parts.push(serde_json::json!({
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": mime_type,
                                "data": data
                            }
                        }));
                    } else if kind == "file" && mime_type == "application/pdf" {
                        parts.push(serde_json::json!({
                            "type": "document",
                            "source": {
                                "type": "base64",
                                "media_type": "application/pdf",
                                "data": data
                            }
                        }));
                    }
                }
                anthropic_messages.push(serde_json::json!({
                    "role": role,
                    "content": parts
                }));
            } else {
                anthropic_messages.push(serde_json::json!({
                    "role": role,
                    "content": content
                }));
            }
        }
    }

    // Ensure messages alternate user/assistant — merge consecutive same-role
    anthropic_messages = merge_consecutive_roles(anthropic_messages);

    let mut body = serde_json::json!({
        "model": model,
        "messages": anthropic_messages,
        "stream": true,
        "max_tokens": max_tokens,
        "temperature": temperature,
    });

    if !system_text.is_empty() {
        body["system"] = serde_json::Value::String(system_text);
    }

    let client = reqwest::Client::new();
    let mut response = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ProviderError {
            status: status.as_u16(),
            body,
        }
        .into());
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

                        if line.is_empty() {
                            continue;
                        }

                        // SSE format: "event: ..." then "data: {...}"
                        let json_str = if let Some(stripped) = line.strip_prefix("data: ") {
                            stripped
                        } else {
                            continue; // skip "event:" lines
                        };

                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                            let event_type = parsed["type"].as_str().unwrap_or("");

                            match event_type {
                                "content_block_delta" => {
                                    let text =
                                        parsed["delta"]["text"].as_str().unwrap_or("").to_string();
                                    if !text.is_empty() {
                                        let chunk = OllamaChunk {
                                            text,
                                            done: false,
                                            finish_reason: None,
                                            eval_count: None,
                                            eval_duration: None,
                                        };
                                        if tx.send(chunk).await.is_err() {
                                            return;
                                        }
                                    }
                                }
                                "message_delta" => {
                                    // End of message — check stop reason
                                    let stop_reason =
                                        parsed["delta"]["stop_reason"].as_str().map(str::to_string);
                                    if stop_reason.is_some() {
                                        let _ = tx
                                            .send(OllamaChunk {
                                                text: String::new(),
                                                done: true,
                                                finish_reason: stop_reason,
                                                eval_count: None,
                                                eval_duration: None,
                                            })
                                            .await;
                                        return;
                                    }
                                }
                                "message_stop" => {
                                    let _ = tx
                                        .send(OllamaChunk {
                                            text: String::new(),
                                            done: true,
                                            finish_reason: Some("stop".to_string()),
                                            eval_count: None,
                                            eval_duration: None,
                                        })
                                        .await;
                                    return;
                                }
                                "error" => {
                                    let msg = parsed["error"]["message"]
                                        .as_str()
                                        .unwrap_or("Unknown error");
                                    tracing::error!(error = %msg, "Anthropic stream error");
                                    return;
                                }
                                _ => {} // ping, content_block_start, etc.
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    tracing::error!(error = %e, "Error reading Anthropic stream");
                    return;
                }
            }
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}

/// Merge consecutive messages with the same role (Anthropic requires alternation).
fn merge_consecutive_roles(messages: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    let mut merged: Vec<serde_json::Value> = Vec::new();
    for msg in messages {
        let role = msg["role"].as_str().unwrap_or("user").to_string();
        let content = msg["content"].as_str().unwrap_or("").to_string();

        if let Some(last) = merged.last_mut() {
            if last["role"].as_str() == Some(&role) {
                // Merge content
                let prev = last["content"].as_str().unwrap_or("").to_string();
                last["content"] = serde_json::Value::String(format!("{}\n\n{}", prev, content));
                continue;
            }
        }
        merged.push(serde_json::json!({"role": role, "content": content}));
    }

    // Anthropic requires first message to be "user"
    if merged.first().map(|m| m["role"].as_str()) == Some(Some("assistant")) {
        merged.insert(
            0,
            serde_json::json!({"role": "user", "content": "Continue."}),
        );
    }

    merged
}
