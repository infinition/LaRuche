//! Multi-provider LLM streaming abstraction.
//!
//! Supports:
//! - **ollama** (default): local Ollama instance
//! - **openai**: OpenAI-compatible APIs (Deepseek, Together, Groq, etc.)
//! - **anthropic**: Anthropic Claude API
//!
//! All providers support **native tool calling** (OpenAI `tools:` format)
//! when a tools array is provided. The parser accumulates `tool_calls` from
//! streaming chunks and delivers them on the final chunk.

use crate::brain::ToolCall;
use crate::streaming::{ollama_chat_stream, OllamaChunk};
use anyhow::Result;
use futures_util::Stream;
use std::pin::Pin;
use tokio_stream::wrappers::ReceiverStream;

/// Structured provider error (HTTP code + body) returned on non-2xx responses.
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

/// Converts the LaRuche tool format (name, description, parameters)
/// to the OpenAI `tools` format (type: function, function: {name, description, parameters}).
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

/// Finalize the streaming tool-call accumulator into an ordered list. The accumulator is
/// keyed by the streaming `index`, so we sort by that (NOT by the provider-random `id`),
/// which preserves the model's intended order of parallel tool calls.
fn finaliser_tool_calls(
    acc: &std::collections::HashMap<u32, (String, String, String)>,
) -> Option<Vec<ToolCall>> {
    if acc.is_empty() {
        return None;
    }
    let mut calls: Vec<(u32, ToolCall)> = acc
        .iter()
        .map(|(idx, (id, name, args_str))| {
            (
                *idx,
                ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    args: serde_json::from_str(args_str).unwrap_or(serde_json::Value::Null),
                },
            )
        })
        .collect();
    calls.sort_by_key(|(idx, _)| *idx);
    Some(calls.into_iter().map(|(_, c)| c).collect())
}

/// Unified streaming entry point: dispatches to the correct provider.
pub async fn provider_chat_stream(
    provider: &str,
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
    api_key: &str,
    api_base: Option<&str>,
    ollama_url: &str,
    tools: Option<&[serde_json::Value]>,  // new parameter
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
    // Build the chat-completions URL. Most OpenAI-compatible APIs (OpenAI, Groq,
    // Deepseek, Together) take a bare host and expect `/v1/chat/completions`. Some
    // (z.ai GLM at `/api/paas/v4`, OpenRouter at `/api/v1`) already carry a version
    // segment in the base path, where forcing another `/v1` would 404. So only add
    // `/v1` when the base does not already end in a version segment or the full path.
    let trimmed = base.trim_end_matches('/');
    let has_version = trimmed
        .rsplit('/')
        .next()
        .map(|s| s.len() >= 2 && s.starts_with('v') && s[1..].chars().all(|c| c.is_ascii_digit()))
        .unwrap_or(false);
    let url = if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else if has_version {
        format!("{trimmed}/chat/completions")
    } else {
        format!("{trimmed}/v1/chat/completions")
    };

    let openai_messages: Vec<serde_json::Value> = messages.iter().map(|m| {
        // Native tool transcript (OpenAI wire format). The generic layer guarantees
        // pairing (every tool_calls entry is followed by its role:"tool" results).
        if m["role"].as_str() == Some("tool") {
            return serde_json::json!({
                "role": "tool",
                "tool_call_id": m["tool_call_id"].as_str().unwrap_or(""),
                "content": m["content"].as_str().unwrap_or(""),
            });
        }
        if let Some(tcs) = m.get("tool_calls").and_then(|v| v.as_array()) {
            let tool_calls: Vec<serde_json::Value> = tcs.iter().map(|t| {
                // OpenAI wants `arguments` as a JSON *string*; the generic layer
                // carries an object.
                let args = t["function"]["arguments"].clone();
                serde_json::json!({
                    "id": t["id"].as_str().unwrap_or(""),
                    "type": "function",
                    "function": {
                        "name": t["function"]["name"].as_str().unwrap_or(""),
                        "arguments": serde_json::to_string(&args).unwrap_or_else(|_| "{}".into()),
                    }
                })
            }).collect();
            return serde_json::json!({
                "role": "assistant",
                "content": m["content"].as_str().unwrap_or(""),
                "tool_calls": tool_calls,
            });
        }
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
                    let format = match mime_type {
                        "audio/mpeg" | "audio/mp3" => "mp3",
                        _ => "wav",
                    };
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
    // Ask for real token usage on the final stream chunk (OpenAI, Groq, Deepseek, ...).
    // Without this, streaming responses carry no usage and the gauge falls back to estimates.
    body["stream_options"] = serde_json::json!({ "include_usage": true });
    // Send native tool definitions (OpenAI format)
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

    tracing::info!(target: "provider", url = %url, model = %model, status = %response.status(), "openai-compatible request sent");
    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        tracing::warn!(target: "provider", status = status.as_u16(), body = %body_text.chars().take(300).collect::<String>(), "openai-compatible request failed");
        return Err(ProviderError { status: status.as_u16(), body: body_text, retry_after: None }.into());
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<OllamaChunk>(64);

    tokio::spawn(async move {
        let mut buffer: Vec<u8> = Vec::new();
        // Opt-in (RUCHE_DEBUG_SSE=1): log the first few raw SSE lines to diagnose an
        // unfamiliar provider's response shape. Off by default to avoid noise.
        let dbg_sse = std::env::var("RUCHE_DEBUG_SSE").as_deref() == Ok("1");
        let mut dbg_lines = 0u8;
        // tool_calls accumulator keyed by index (streaming delta)
        // Each entry: (id, name, partial_args_string)
        let mut tool_call_acc: std::collections::HashMap<u32, (String, String, String)> = std::collections::HashMap::new();
        // Actual usage (if the server includes it: OpenAI with stream_options, llama.cpp by default).
        let mut in_tok: Option<u64> = None;
        let mut out_tok: Option<u64> = None;
        // Reasoning models stream chain-of-thought in `reasoning_content`. We accumulate it but
        // never stream it as the answer. Only if the model produced NO `content` at all (e.g. a
        // broken "flash" proxy) do we surface the reasoning as a last resort, so the turn is not
        // silently empty. `reasoning_emitted` guards against emitting it twice.
        let mut content_streamed = false;
        let mut reasoning_acc = String::new();
        let mut reasoning_emitted = false;

        loop {
            match response.chunk().await {
                Ok(Some(bytes)) => {
                    buffer.extend_from_slice(&bytes);
                    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                        let line_bytes: Vec<u8> = buffer.drain(..=newline_pos).collect();
                        let line = String::from_utf8_lossy(&line_bytes).trim().to_string();
                        if dbg_sse && dbg_lines < 5 && !line.is_empty() {
                            dbg_lines += 1;
                            tracing::info!(target: "provider", line = %line.chars().take(280).collect::<String>(), "raw SSE line");
                        }
                        if line.is_empty() || line == "data: [DONE]" {
                            if line == "data: [DONE]" {
                                // Last resort: model produced only reasoning and no content, and the
                                // stream ends via [DONE] without an in-chunk finish_reason.
                                if !content_streamed && !reasoning_emitted {
                                    let r = reasoning_acc.trim();
                                    if !r.is_empty() {
                                        reasoning_emitted = true;
                                        let _ = tx.send(OllamaChunk {
                                            text: r.to_string(), done: false,
                                            finish_reason: None, eval_count: None,
                                            eval_duration: None, prompt_eval_count: None,
                                            tool_calls: None,
                                        }).await;
                                    }
                                }
                                // Finalize the accumulated tool_calls (ordered by index).
                                let tool_calls = finaliser_tool_calls(&tool_call_acc);
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
                            // Actual usage (top-level, present on the final chunk or a dedicated chunk).
                            if let Some(u) = parsed["usage"]["prompt_tokens"].as_u64() { in_tok = Some(u); }
                            if let Some(u) = parsed["usage"]["completion_tokens"].as_u64() { out_tok = Some(u); }
                            let mut text = parsed["choices"][0]["delta"]["content"].as_str().unwrap_or("").to_string();
                            if !text.is_empty() {
                                content_streamed = true;
                            }
                            // Accumulate reasoning (chain-of-thought) without streaming it as the answer.
                            if let Some(rc) = parsed["choices"][0]["delta"]["reasoning_content"].as_str() {
                                reasoning_acc.push_str(rc);
                            }
                            let finish_reason = parsed["choices"][0]["finish_reason"].as_str().map(str::to_string);
                            let done = finish_reason.is_some();

                            // Last resort: if the model produced NO content at all, surface the
                            // accumulated reasoning on the final chunk so the turn is not silently empty.
                            if done && !content_streamed && !reasoning_emitted && text.is_empty() {
                                let r = reasoning_acc.trim();
                                if !r.is_empty() {
                                    text = r.to_string();
                                    reasoning_emitted = true;
                                }
                            }

                            // Parse the tool_calls delta (OpenAI streaming format)
                            if let Some(tc_deltas) = parsed["choices"][0]["delta"]["tool_calls"].as_array() {
                                for tc_delta in tc_deltas {
                                    let idx = tc_delta["index"].as_u64().unwrap_or(0) as u32;
                                    let entry = tool_call_acc.entry(idx).or_insert_with(|| {
                                        (String::new(), String::new(), String::new())
                                    });
                                    // id: present only on the first chunk of the tool call
                                    if let Some(id_val) = tc_delta["id"].as_str() {
                                        entry.0 = id_val.to_string();
                                    }
                                    if entry.0.is_empty() {
                                        entry.0 = format!("call_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x"));
                                    }
                                    // function.name: present on the first chunk
                                    if let Some(name_val) = tc_delta["function"]["name"].as_str() {
                                        entry.1 = name_val.to_string();
                                    }
                                    // function.arguments: concatenated across multiple chunks
                                    if let Some(args_val) = tc_delta["function"]["arguments"].as_str() {
                                        entry.2.push_str(args_val);
                                    }
                                }
                            }

                            if !text.is_empty() || done {
                                // Send the accumulated tool_calls only on the final chunk
                                let tool_calls = if done { finaliser_tool_calls(&tool_call_acc) } else { None };

                                let chunk = OllamaChunk {
                                    text, done, finish_reason,
                                    eval_count: if done { out_tok } else { None },
                                    eval_duration: None,
                                    prompt_eval_count: if done { in_tok } else { None },
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

    // Anthropic wants the system prompt as a top-level `system` field, NOT a message
    // (a system role inside `messages` is rejected). Pull any system messages out, and
    // mark the last system block with `cache_control: ephemeral` so the large, stable
    // prefix (system prompt) is served from the prompt cache on repeated calls, cutting
    // input cost and latency.
    //
    // Native tool transcript: assistant `tool_calls` become `tool_use` content blocks;
    // `role:"tool"` results become `tool_result` blocks grouped — with any adjacent
    // user text and images — into a SINGLE user message (strict alternation, and
    // parallel results must share one user turn). The generic layer guarantees
    // call/result pairing.
    let mut system_blocks: Vec<serde_json::Value> = Vec::new();
    let mut convo: Vec<serde_json::Value> = Vec::new();
    let mut user_blocks: Vec<serde_json::Value> = Vec::new();
    fn flush_user(convo: &mut Vec<serde_json::Value>, user_blocks: &mut Vec<serde_json::Value>) {
        if !user_blocks.is_empty() {
            convo.push(serde_json::json!({"role": "user", "content": std::mem::take(user_blocks)}));
        }
    }
    for m in messages {
        match m["role"].as_str().unwrap_or("user") {
            "system" => {
                if let Some(text) = m["content"].as_str() {
                    if !text.trim().is_empty() {
                        system_blocks.push(serde_json::json!({"type": "text", "text": text}));
                    }
                }
            }
            "tool" => {
                user_blocks.push(serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": m["tool_call_id"].as_str().unwrap_or(""),
                    "content": m["content"].as_str().unwrap_or(""),
                }));
            }
            "assistant" => {
                flush_user(&mut convo, &mut user_blocks);
                let mut blocks: Vec<serde_json::Value> = Vec::new();
                let text = m["content"].as_str().unwrap_or("");
                if !text.trim().is_empty() {
                    blocks.push(serde_json::json!({"type": "text", "text": text}));
                }
                if let Some(tcs) = m.get("tool_calls").and_then(|v| v.as_array()) {
                    for t in tcs {
                        blocks.push(serde_json::json!({
                            "type": "tool_use",
                            "id": t["id"].as_str().unwrap_or(""),
                            "name": t["function"]["name"].as_str().unwrap_or(""),
                            // `input` is an object: the generic layer carries one.
                            "input": t["function"]["arguments"].clone(),
                        }));
                    }
                }
                if !blocks.is_empty() {
                    convo.push(serde_json::json!({"role": "assistant", "content": blocks}));
                }
            }
            _ => {
                // user: text + native image blocks (Anthropic vision).
                let text = m["content"].as_str().unwrap_or("");
                if !text.trim().is_empty() {
                    user_blocks.push(serde_json::json!({"type": "text", "text": text}));
                }
                if let Some(atts) = m.get("attachments").and_then(|a| a.as_array()) {
                    for att in atts {
                        if att["kind"].as_str() == Some("image") {
                            user_blocks.push(serde_json::json!({
                                "type": "image",
                                "source": {
                                    "type": "base64",
                                    "media_type": att["mime_type"].as_str().unwrap_or("image/png"),
                                    "data": att["data"].as_str().unwrap_or(""),
                                }
                            }));
                        }
                    }
                }
            }
        }
    }
    flush_user(&mut convo, &mut user_blocks);
    if let Some(last) = system_blocks.last_mut() {
        last["cache_control"] = serde_json::json!({"type": "ephemeral"});
    }

    let mut body = serde_json::json!({
        "model": model,
        "messages": convo,
        "stream": true,
        "max_tokens": anthropic_max,
        "temperature": temperature,
    });
    if !system_blocks.is_empty() {
        body["system"] = serde_json::json!(system_blocks);
    }

    // Anthropic also supports native tool calling, with a slightly different format
    if let Some(tools_list) = tools {
        let mut anthropic_tools: Vec<serde_json::Value> = tools_list.iter().filter_map(|t| {
            Some(serde_json::json!({
                "name": t["name"].as_str()?,
                "description": t["description"].as_str().unwrap_or(""),
                "input_schema": t.get("parameters").cloned().unwrap_or(serde_json::json!({})),
            }))
        }).collect();
        if !anthropic_tools.is_empty() {
            // Cache the tool definitions too (stable across a conversation): mark the
            // last tool so the whole tools block is cached up to that point.
            if let Some(last) = anthropic_tools.last_mut() {
                last["cache_control"] = serde_json::json!({"type": "ephemeral"});
            }
            body["tools"] = serde_json::json!(anthropic_tools);
        }
    }

    // ... the rest of the Anthropic code stays identical
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
        let mut buffer: Vec<u8> = Vec::new();
        // Actual usage provided by Anthropic in the stream: input at `message_start`,
        // output at `message_delta`. Emitted on the final chunk for an accurate gauge.
        let mut in_tok: Option<u64> = None;
        let mut out_tok: Option<u64> = None;
        // Native tool_use blocks, keyed by content-block index: (id, name, partial_json).
        let mut tool_acc: std::collections::HashMap<u64, (String, String, String)> =
            std::collections::HashMap::new();
        loop {
            match response.chunk().await {
                Ok(Some(bytes)) => {
                    buffer.extend_from_slice(&bytes);
                    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                        let line_bytes: Vec<u8> = buffer.drain(..=newline_pos).collect();
                        let line = String::from_utf8_lossy(&line_bytes).trim().to_string();
                        if line.is_empty() { continue; }

                        // Anthropic SSE: event: ..., data: {...}
                        if let Some(data) = line.strip_prefix("data: ") {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                                let chunk_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                match chunk_type {
                                    "message_start" => {
                                        if let Some(u) = parsed["message"]["usage"]["input_tokens"].as_u64() {
                                            in_tok = Some(u);
                                        }
                                    }
                                    "message_delta" => {
                                        if let Some(u) = parsed["usage"]["output_tokens"].as_u64() {
                                            out_tok = Some(u);
                                        }
                                    }
                                    "content_block_start" => {
                                        // A tool_use block opens with its id + name.
                                        if parsed["content_block"]["type"].as_str() == Some("tool_use") {
                                            let idx = parsed["index"].as_u64().unwrap_or(0);
                                            let id = parsed["content_block"]["id"].as_str().unwrap_or("").to_string();
                                            let name = parsed["content_block"]["name"].as_str().unwrap_or("").to_string();
                                            tool_acc.insert(idx, (id, name, String::new()));
                                        }
                                    }
                                    _ => {}
                                }
                                // tool_use arguments stream as input_json_delta on the block.
                                if chunk_type == "content_block_delta"
                                    && parsed["delta"]["type"].as_str() == Some("input_json_delta")
                                {
                                    if let Some(pj) = parsed["delta"]["partial_json"].as_str() {
                                        let idx = parsed["index"].as_u64().unwrap_or(0);
                                        tool_acc
                                            .entry(idx)
                                            .or_insert_with(|| (String::new(), String::new(), String::new()))
                                            .2
                                            .push_str(pj);
                                    }
                                }
                                let text = match chunk_type {
                                    "content_block_delta" => parsed["delta"]["text"].as_str().unwrap_or("").to_string(),
                                    _ => String::new(),
                                };
                                let done = chunk_type == "message_stop";
                                let finish_reason = if done { Some("stop".to_string()) } else { None };
                                // Emit the accumulated tool_use blocks (ordered by index) on stop.
                                let tool_calls = if done && !tool_acc.is_empty() {
                                    let mut calls: Vec<(u64, ToolCall)> = tool_acc
                                        .iter()
                                        .map(|(idx, (id, name, args_str))| {
                                            let args = if args_str.trim().is_empty() {
                                                serde_json::json!({})
                                            } else {
                                                serde_json::from_str(args_str).unwrap_or(serde_json::Value::Null)
                                            };
                                            (*idx, ToolCall { id: id.clone(), name: name.clone(), args })
                                        })
                                        .collect();
                                    calls.sort_by_key(|(idx, _)| *idx);
                                    Some(calls.into_iter().map(|(_, c)| c).collect())
                                } else {
                                    None
                                };

                                if !text.is_empty() || done {
                                    let _ = tx.send(OllamaChunk {
                                        text, done, finish_reason,
                                        eval_count: if done { out_tok } else { None },
                                        eval_duration: None,
                                        prompt_eval_count: if done { in_tok } else { None },
                                        tool_calls,
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
            // Text-only Responses API: native tool structures are re-rendered as text
            // so the transcript stays coherent.
            "tool" => {
                input.push(serde_json::json!({
                    "role": "user",
                    "content": format!(
                        "[Tool Result: {}]\n{}",
                        m["name"].as_str().unwrap_or("tool"),
                        m["content"].as_str().unwrap_or("")
                    ),
                }));
            }
            role => {
                let mut content = m["content"].as_str().unwrap_or("").to_string();
                if let Some(tcs) = m.get("tool_calls").and_then(|v| v.as_array()) {
                    for t in tcs {
                        content.push_str(&format!(
                            "\n<tool_call>{}</tool_call>",
                            serde_json::json!({
                                "name": t["function"]["name"],
                                "arguments": t["function"]["arguments"],
                            })
                        ));
                    }
                }
                let entry = serde_json::json!({
                    "role": if role == "assistant" { "assistant" } else { "user" },
                    "content": content,
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
    // Anti-Cloudflare headers (User-Agent, originator, account id) required by the Codex
    // backend; without them requests are likely rejected with a 403.
    for (k, v) in codex_auth::codex_headers(&access_token) { req = req.header(k, v); }
    for (k, v) in mesh_headers("/responses") { req = req.header(k, v); }
    let mut response = req.json(&body).send().await?;

    let (tx, rx) = tokio::sync::mpsc::channel::<OllamaChunk>(64);
    tokio::spawn(async move {
        let mut buffer: Vec<u8> = Vec::new();
        loop {
            match response.chunk().await {
                Ok(Some(bytes)) => {
                    buffer.extend_from_slice(&bytes);
                    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                        let line_bytes: Vec<u8> = buffer.drain(..=newline_pos).collect();
                        let line = String::from_utf8_lossy(&line_bytes).trim().to_string();
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
