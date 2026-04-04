use anyhow::Result;
use futures_util::Stream;
use serde::Deserialize;
use std::pin::Pin;
use tokio_stream::wrappers::ReceiverStream;

/// A single chunk from Ollama's streaming response.
#[derive(Debug, Clone)]
pub struct OllamaChunk {
    pub text: String,
    pub done: bool,
    pub eval_count: Option<u64>,
    pub eval_duration: Option<u64>,
}

/// Raw Ollama streaming JSON line (works for both /api/chat and /api/generate).
#[derive(Debug, Deserialize)]
struct OllamaStreamLine {
    // /api/chat format
    message: Option<OllamaStreamMessage>,
    // /api/generate format
    response: Option<String>,
    done: Option<bool>,
    eval_count: Option<u64>,
    eval_duration: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamMessage {
    content: Option<String>,
}

/// Start a streaming request to Ollama and return a stream of chunks.
///
/// Tries /api/chat first. If the model doesn't support chat (400 error),
/// falls back to /api/generate with a concatenated prompt.
pub async fn ollama_chat_stream(
    ollama_url: &str,
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    let client = reqwest::Client::new();

    let options = serde_json::json!({
        "num_predict": max_tokens,
        "temperature": temperature,
    });

    // Try /api/chat first
    let chat_body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "options": options,
    });

    let mut response = client
        .post(format!("{}/api/chat", ollama_url))
        .json(&chat_body)
        .send()
        .await?;

    // If chat endpoint fails (model doesn't support it), fallback to /api/generate
    if !response.status().is_success() {
        tracing::info!(
            model = model,
            "Model does not support /api/chat, falling back to /api/generate"
        );

        // Concatenate messages into a single prompt
        let prompt = messages_to_prompt(messages);
        let generate_body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": true,
            "options": options,
        });

        response = client
            .post(format!("{}/api/generate", ollama_url))
            .json(&generate_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama error {}: {}", status, body);
        }
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<OllamaChunk>(64);

    // Spawn a task to read the streaming response line by line
    tokio::spawn(async move {
        // Read the full response as a byte stream and process NDJSON lines
        let mut buffer = String::new();

        // Use chunk-based reading for streaming
        loop {
            let maybe_chunk = response.chunk().await;
            match maybe_chunk {
                Ok(Some(bytes)) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));

                    // Process complete lines (Ollama sends newline-delimited JSON)
                    while let Some(newline_pos) = buffer.find('\n') {
                        let line = buffer[..newline_pos].trim().to_string();
                        buffer = buffer[newline_pos + 1..].to_string();

                        if line.is_empty() {
                            continue;
                        }

                        match serde_json::from_str::<OllamaStreamLine>(&line) {
                            Ok(parsed) => {
                                let done = parsed.done.unwrap_or(false);
                                // Support both /api/chat (message.content) and /api/generate (response)
                                let text = parsed
                                    .message
                                    .and_then(|m| m.content)
                                    .or(parsed.response)
                                    .unwrap_or_default();

                                let chunk = OllamaChunk {
                                    text,
                                    done,
                                    eval_count: parsed.eval_count,
                                    eval_duration: parsed.eval_duration,
                                };

                                if tx.send(chunk).await.is_err() {
                                    return; // Receiver dropped
                                }

                                if done {
                                    return;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    line = %line,
                                    error = %e,
                                    "Failed to parse Ollama stream line"
                                );
                            }
                        }
                    }
                }
                Ok(None) => break, // Stream ended
                Err(e) => {
                    tracing::error!(error = %e, "Error reading Ollama stream");
                    return;
                }
            }
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}

/// Convert a list of chat messages into a single prompt string
/// for use with /api/generate (fallback for models without chat support).
fn messages_to_prompt(messages: &[serde_json::Value]) -> String {
    let mut prompt = String::new();
    for msg in messages {
        let role = msg["role"].as_str().unwrap_or("user");
        let content = msg["content"].as_str().unwrap_or("");
        match role {
            "system" => {
                prompt.push_str(&format!("[System]\n{}\n\n", content));
            }
            "user" => {
                prompt.push_str(&format!("[User]\n{}\n\n", content));
            }
            "assistant" => {
                prompt.push_str(&format!("[Assistant]\n{}\n\n", content));
            }
            _ => {
                prompt.push_str(&format!("[{}]\n{}\n\n", role, content));
            }
        }
    }
    prompt.push_str("[Assistant]\n");
    prompt
}

/// Non-streaming inference — simpler for testing.
pub async fn ollama_chat(
    ollama_url: &str,
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
) -> Result<OllamaChatResponse> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/chat", ollama_url);

    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false,
        "options": {
            "num_predict": max_tokens,
            "temperature": temperature,
        }
    });

    let response = client.post(&url).json(&body).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Ollama error {}: {}", status, body);
    }

    let parsed: serde_json::Value = response.json().await?;

    Ok(OllamaChatResponse {
        content: parsed["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        model: parsed["model"].as_str().unwrap_or(model).to_string(),
        eval_count: parsed["eval_count"].as_u64().unwrap_or(0) as u32,
        eval_duration_ns: parsed["eval_duration"].as_u64().unwrap_or(0),
    })
}

#[derive(Debug)]
pub struct OllamaChatResponse {
    pub content: String,
    pub model: String,
    pub eval_count: u32,
    pub eval_duration_ns: u64,
}

impl OllamaChatResponse {
    pub fn tokens_per_sec(&self) -> f32 {
        let duration_secs = self.eval_duration_ns as f64 / 1_000_000_000.0;
        if duration_secs > 0.0 {
            self.eval_count as f32 / duration_secs as f32
        } else {
            0.0
        }
    }
}
