use anyhow::Result;
use futures_util::Stream;
use serde::Deserialize;
use std::pin::Pin;
use tokio_stream::wrappers::ReceiverStream;

use crate::brain::ToolCall;

/// A single chunk from Ollama's streaming response.
#[derive(Debug, Clone)]
pub struct OllamaChunk {
    pub text: String,
    pub done: bool,
    pub finish_reason: Option<String>,
    pub eval_count: Option<u64>,
    pub eval_duration: Option<u64>,
    /// Input (prompt) tokens actually consumed, returned by Ollama on the final chunk.
    pub prompt_eval_count: Option<u64>,
    /// Native tool calls (OpenAI format), present only on the last chunk
    /// when the model decides to call tools via the native `tools:` API.
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Raw Ollama streaming JSON line (works for both /api/chat and /api/generate).
#[derive(Debug, Deserialize)]
struct OllamaStreamLine {
    message: Option<OllamaStreamMessage>,
    response: Option<String>,
    done: Option<bool>,
    /// Real stop reason on the final chunk ("stop", "length", ...).
    done_reason: Option<String>,
    eval_count: Option<u64>,
    eval_duration: Option<u64>,
    prompt_eval_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamMessage {
    content: Option<String>,
    /// Native tool_calls in the Ollama response (OpenAI-compatible format)
    tool_calls: Option<Vec<OllamaToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OllamaToolCall {
    function: OllamaToolFunction,
}

#[derive(Debug, Deserialize)]
struct OllamaToolFunction {
    name: Option<String>,
    arguments: Option<serde_json::Value>,
}

/// Start a streaming request to Ollama and return a stream of chunks.
pub async fn ollama_chat_stream(
    ollama_url: &str,
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
    tools: Option<&[serde_json::Value]>,
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    let client = reqwest::Client::new();

    let mut options = serde_json::json!({
        "temperature": temperature,
    });
    if max_tokens > 0 {
        options["num_predict"] = serde_json::json!(max_tokens);
    }

    let mut chat_body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "options": options,
    });
    // Add tool definitions if provided (Ollama >= 0.5.0)
    if let Some(tools_list) = tools {
        chat_body["tools"] = serde_json::json!(tools_list);
    }

    let mut response = client
        .post(format!("{}/api/chat", ollama_url))
        .json(&chat_body)
        .send()
        .await?;

    // If chat endpoint fails, fallback to /api/generate (without tools)
    if !response.status().is_success() {
        if tools.is_some() {
            // Retry without tools for older models
            let mut fallback_body = chat_body.clone();
            fallback_body.as_object_mut().map(|obj| obj.remove("tools"));
            response = client
                .post(format!("{}/api/chat", ollama_url))
                .json(&fallback_body)
                .send()
                .await?;
        }
        if !response.status().is_success() {
            // Final fallback to generate
            let generate_body = serde_json::json!({
                "model": model,
                "prompt": messages.iter()
                    .filter_map(|m| m["content"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
                "stream": true,
                "options": options,
            });
            response = client
                .post(format!("{}/api/generate", ollama_url))
                .json(&generate_body)
                .send()
                .await?;
        }
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<OllamaChunk>(64);

    tokio::spawn(async move {
        // Accumulate bytes and only parse COMPLETE lines (up to the last newline). Ollama
        // streams newline-delimited JSON; decoding per TCP chunk would drop any JSON object
        // (including the final `done`/usage/tool_calls) split across two chunks, and corrupt
        // multibyte chars split at a chunk boundary.
        let mut buf: Vec<u8> = Vec::new();
        loop {
            match response.chunk().await {
                Ok(Some(bytes)) => {
                    buf.extend_from_slice(&bytes);
                    while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                        let line_bytes: Vec<u8> = buf.drain(..=pos).collect();
                        let line_cow = String::from_utf8_lossy(&line_bytes);
                        let line = line_cow.trim();
                        if line.is_empty() {
                            continue;
                        }
                        if let Ok(parsed) = serde_json::from_str::<OllamaStreamLine>(line) {
                            let content = parsed
                                .message
                                .as_ref()
                                .and_then(|m| m.content.clone())
                                .or(parsed.response.clone())
                                .unwrap_or_default();
                            let done = parsed.done.unwrap_or(false);
                            // Real stop reason when Ollama supplies it ("stop"/"length");
                            // hardcoding "stop" broke truncation detection downstream.
                            let finish_reason = if done {
                                Some(parsed.done_reason.clone().unwrap_or_else(|| "stop".to_string()))
                            } else {
                                None
                            };

                            // Native tool_calls: recent Ollama streams them on INTERMEDIATE
                            // chunks (done:false, empty content) - qwen3 notably - while the
                            // final done-chunk only carries usage. Reading them only on `done`
                            // dropped EVERY call (observed: qwen3 evals 0/8, "zero tool
                            // calls"). Parse on every chunk; the consumer accumulates.
                            let tool_calls = parsed.message.as_ref()
                                .and_then(|m| m.tool_calls.as_ref())
                                .map(|calls| {
                                    calls.iter().filter_map(|tc| {
                                        // Some models emit `arguments` as an embedded JSON
                                        // STRING: unwrap it so the engine sees an object.
                                        let args = match tc.function.arguments.clone() {
                                            Some(serde_json::Value::String(s)) => {
                                                serde_json::from_str(&s)
                                                    .unwrap_or(serde_json::Value::String(s))
                                            }
                                            Some(v) => v,
                                            None => serde_json::Value::Null,
                                        };
                                        Some(ToolCall {
                                            id: format!("call_{}", uuid::Uuid::new_v4()),
                                            name: tc.function.name.clone()?,
                                            args,
                                        })
                                    }).collect::<Vec<_>>()
                                })
                                .filter(|v: &Vec<ToolCall>| !v.is_empty());

                            // A tool-call-only chunk (no text, not done) must NOT be dropped.
                            if !content.is_empty() || done || tool_calls.is_some() {
                                let _ = tx
                                    .send(OllamaChunk {
                                        text: content,
                                        done,
                                        finish_reason,
                                        eval_count: parsed.eval_count,
                                        eval_duration: parsed.eval_duration,
                                        prompt_eval_count: parsed.prompt_eval_count,
                                        tool_calls,
                                    })
                                    .await;
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    tracing::error!(error = %e, "Error reading Ollama stream");
                    break;
                }
            }
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}
