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
        "openai" => {
            openai_chat_stream(model, messages, temperature, max_tokens, api_key, api_base).await
        }
        "anthropic" => {
            anthropic_chat_stream(model, messages, temperature, max_tokens, api_key, api_base).await
        }
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
    if api_key.is_empty() {
        anyhow::bail!("API key is required for OpenAI-compatible provider. Configure in Settings > Providers.");
    }
    let base = api_base.unwrap_or("https://api.openai.com");
    let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));

    // Convert messages: ensure role/content only (strip Ollama-specific fields)
    let openai_messages: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| {
            serde_json::json!({
                "role": m["role"].as_str().unwrap_or("user"),
                "content": m["content"].as_str().unwrap_or("")
            })
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
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("OpenAI API error {}: {}", status, body);
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

                            let finish = parsed["choices"][0]["finish_reason"]
                                .as_str()
                                .map(|s| s == "stop")
                                .unwrap_or(false);

                            if !text.is_empty() || finish {
                                let chunk = OllamaChunk {
                                    text,
                                    done: finish,
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
        anyhow::bail!("API key is required for Anthropic provider. Configure in Settings > Providers.");
    }
    let base = api_base.unwrap_or("https://api.anthropic.com");
    let url = format!("{}/v1/messages", base.trim_end_matches('/'));

    // Separate system message from user/assistant messages
    let mut system_text = String::new();
    let mut anthropic_messages: Vec<serde_json::Value> = Vec::new();

    for m in messages {
        let role = m["role"].as_str().unwrap_or("user");
        let content = m["content"].as_str().unwrap_or("");
        if role == "system" {
            system_text.push_str(content);
        } else {
            anthropic_messages.push(serde_json::json!({
                "role": role,
                "content": content
            }));
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
        anyhow::bail!("Anthropic API error {}: {}", status, body);
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
                                    let text = parsed["delta"]["text"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string();
                                    if !text.is_empty() {
                                        let chunk = OllamaChunk {
                                            text,
                                            done: false,
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
                                    let stop = parsed["delta"]["stop_reason"]
                                        .as_str()
                                        .map(|s| s == "end_turn" || s == "stop_sequence")
                                        .unwrap_or(false);
                                    if stop {
                                        let _ = tx
                                            .send(OllamaChunk {
                                                text: String::new(),
                                                done: true,
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
