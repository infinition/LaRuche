//! ReAct Agent Loop — inspired by OpenClaw's agent architecture.
//!
//! Key patterns from OpenClaw:
//! - Stop reason handling (end_turn, tool_use, max_tokens)
//! - Auto-compaction when context exceeds threshold
//! - Model failover on errors
//! - Streaming with thinking blocks separation
//! - Tool execution with timing

use crate::abeille::{AbeilleRegistry, ContextExecution, NiveauDanger};
use crate::prompt::build_system_prompt;
use crate::session::Session;
use crate::providers::provider_chat_stream;
use anyhow::Result;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Response to an approval request.
#[derive(Debug, Clone)]
pub struct ApprovalResponse {
    pub tool_call_id: String,
    pub approved: bool,
}

/// Channel for receiving approval responses from the UI.
pub type ApprovalReceiver = tokio::sync::mpsc::Receiver<ApprovalResponse>;

/// Configuration for the Essaim agent engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EssaimConfig {
    /// Ollama API URL (default: http://127.0.0.1:11434)
    pub ollama_url: String,
    /// Default model for inference
    pub model: String,
    /// Fallback models (tried in order if primary fails)
    #[serde(default)]
    pub fallback_models: Vec<String>,
    /// Maximum ReAct iterations before giving up
    pub max_iterations: usize,
    /// Temperature for LLM sampling
    pub temperature: f32,
    /// Maximum tokens per response
    pub max_tokens: u32,
    /// Custom system prompt instructions
    pub custom_instructions: Option<String>,
    /// Max messages in context before auto-compaction (default: 30)
    pub context_max_messages: usize,
    /// Context compaction threshold ratio (default: 0.75)
    pub compaction_threshold: f32,
    /// Cost per 1k input tokens in USD (default: 0.0)
    #[serde(default)]
    pub cost_per_1k_input: f32,
    /// Cost per 1k output tokens in USD (default: 0.0)
    #[serde(default)]
    pub cost_per_1k_output: f32,
    /// LLM provider: "ollama" (default), "openai", "anthropic"
    #[serde(default = "default_provider")]
    pub provider: String,
    /// API key for cloud providers (empty for Ollama)
    #[serde(default)]
    pub api_key: String,
    /// API base URL override (e.g., for OpenAI-compatible servers)
    #[serde(default)]
    pub api_base: Option<String>,
}

fn default_provider() -> String {
    "ollama".to_string()
}

impl Default for EssaimConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://127.0.0.1:11434".to_string(),
            model: "gemma4:e4b".to_string(),
            fallback_models: vec![],
            max_iterations: 15,
            temperature: 0.7,
            max_tokens: 4096,
            custom_instructions: None,
            context_max_messages: 30,
            compaction_threshold: 0.75,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            provider: "ollama".to_string(),
            api_key: String::new(),
            api_base: None,
        }
    }
}

/// Events emitted during the ReAct loop — sent to the WebSocket client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChatEvent {
    #[serde(rename = "token")]
    Token { text: String },

    #[serde(rename = "tool_call")]
    ToolCall {
        name: String,
        args: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        iteration: Option<usize>,
    },

    #[serde(rename = "tool_result")]
    ToolResult {
        name: String,
        result: String,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        elapsed_ms: Option<u64>,
    },

    #[serde(rename = "approval_request")]
    ApprovalRequest {
        tool_call_id: String,
        name: String,
        args: serde_json::Value,
    },

    #[serde(rename = "done")]
    Done { full_response: String },

    #[serde(rename = "error")]
    Error { message: String },

    #[serde(rename = "status")]
    Status { message: String },

    #[serde(rename = "plan")]
    Plan { items: Vec<PlanItem> },

    #[serde(rename = "thinking")]
    Thinking { text: String },

    /// Context compaction happened
    #[serde(rename = "compaction")]
    Compaction {
        messages_before: usize,
        messages_after: usize,
    },

    /// Model failover occurred
    #[serde(rename = "failover")]
    Failover {
        from_model: String,
        to_model: String,
        reason: String,
    },

    /// Token usage and cost estimate
    #[serde(rename = "usage")]
    Usage {
        input_tokens: u32,
        output_tokens: u32,
        cost_usd: f32,
    },
}

/// A plan/todo item for the agent sidebar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanItem {
    pub task: String,
    pub status: String,
}

/// A parsed tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
}

/// Parse tool calls from the LLM response text.
pub fn parse_tool_calls(text: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();
    let mut search_from = 0;

    while let Some(start) = text[search_from..].find("<tool_call>") {
        let abs_start = search_from + start + "<tool_call>".len();
        if let Some(end) = text[abs_start..].find("</tool_call>") {
            let abs_end = abs_start + end;
            let json_str = text[abs_start..abs_end].trim();
            match serde_json::from_str::<ToolCallRaw>(json_str) {
                Ok(raw) => {
                    calls.push(ToolCall {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: raw.name,
                        args: raw.arguments,
                    });
                }
                Err(e) => {
                    tracing::warn!(json = %json_str, error = %e, "Failed to parse tool_call JSON");
                }
            }
            search_from = abs_end + "</tool_call>".len();
        } else {
            break;
        }
    }

    calls
}

#[derive(Debug, Deserialize)]
struct ToolCallRaw {
    name: String,
    arguments: serde_json::Value,
}

/// Parse plan items from `<plan>[...]</plan>` tags in the response.
pub fn parse_plan(text: &str) -> Option<Vec<PlanItem>> {
    let start = text.find("<plan>")?;
    let end = text.find("</plan>")?;
    if end <= start {
        return None;
    }
    let json_str = text[start + "<plan>".len()..end].trim();
    serde_json::from_str::<Vec<PlanItem>>(json_str).ok()
}

/// Strip `<plan>...</plan>` blocks from text.
fn strip_plan_tags(text: &str) -> String {
    let mut result = text.to_string();
    while let Some(start) = result.find("<plan>") {
        if let Some(end) = result.find("</plan>") {
            result = format!("{}{}", &result[..start], &result[end + "</plan>".len()..]);
        } else {
            result.truncate(start);
            break;
        }
    }
    result.trim().to_string()
}

/// The main ReAct loop — inspired by OpenClaw's agent architecture.
///
/// Flow:
/// 1. Build system prompt with tools schema
/// 2. Stream LLM response (with thinking separation)
/// 3. Handle stop reason: end_turn → done, tool_use → execute + loop
/// 4. Auto-compact context if too large
/// 5. Failover to fallback model on error
/// Run the ReAct loop (convenience wrapper without images or approval).
pub async fn boucle_react(
    prompt_utilisateur: &str,
    session: &mut Session,
    registry: &AbeilleRegistry,
    config: &EssaimConfig,
    tx: &tokio::sync::broadcast::Sender<ChatEvent>,
) -> Result<String> {
    boucle_react_multimodal(prompt_utilisateur, session, registry, config, tx, vec![], None).await
}

/// The main ReAct loop — inspired by OpenClaw's agent architecture.
/// Supports multimodal input and interactive approval gating.
pub async fn boucle_react_multimodal(
    prompt_utilisateur: &str,
    session: &mut Session,
    registry: &AbeilleRegistry,
    config: &EssaimConfig,
    tx: &tokio::sync::broadcast::Sender<ChatEvent>,
    images: Vec<String>,
    mut approval_rx: Option<ApprovalReceiver>,
) -> Result<String> {
    session.ajouter_user_multimodal(prompt_utilisateur, images);

    let system_prompt = build_system_prompt(
        &registry.schema_complet(),
        config.custom_instructions.as_deref(),
    );

    // Track which model we're using (for failover)
    let mut current_model = config.model.clone();
    let mut failover_attempted = false;

    for iteration in 0..config.max_iterations {
        tracing::debug!(iteration, model = %current_model, "ReAct iteration");

        // Auto-compaction: if session is too large, compact before sending
        if session.len() > config.context_max_messages {
            let before = session.len();
            session.compacter(config.context_max_messages);
            let after = session.len();
            if before != after {
                let _ = tx.send(ChatEvent::Compaction {
                    messages_before: before,
                    messages_after: after,
                });
                tracing::info!(before, after, "Auto-compacted session context");
            }
        }

        if iteration > 0 {
            let _ = tx.send(ChatEvent::Status {
                message: format!("Step {}/{} — processing...", iteration + 1, config.max_iterations),
            });
        }

        // Build messages for LLM
        let messages = session.build_ollama_messages(&system_prompt);

        // Stream LLM response — with failover on error
        let stream_result = provider_chat_stream(
            &config.provider,
            &current_model,
            &messages,
            config.temperature,
            config.max_tokens,
            &config.api_key,
            config.api_base.as_deref(),
            &config.ollama_url,
        )
        .await;

        let mut stream = match stream_result {
            Ok(s) => s,
            Err(e) => {
                // Model failover: try fallback models
                if !failover_attempted && !config.fallback_models.is_empty() {
                    for fallback in &config.fallback_models {
                        tracing::warn!(
                            from = %current_model,
                            to = %fallback,
                            error = %e,
                            "Model failover"
                        );
                        let _ = tx.send(ChatEvent::Failover {
                            from_model: current_model.clone(),
                            to_model: fallback.clone(),
                            reason: e.to_string(),
                        });
                        current_model = fallback.clone();
                        let _ = failover_attempted; // used below
                        failover_attempted = true;

                        // Retry with fallback
                        match provider_chat_stream(
                            &config.provider,
                            &current_model,
                            &messages,
                            config.temperature,
                            config.max_tokens,
                            &config.api_key,
                            config.api_base.as_deref(),
                            &config.ollama_url,
                        )
                        .await
                        {
                            Ok(_s) => break,
                            Err(_) => continue,
                        }
                    }
                    // If we get here, all fallbacks failed
                    return Err(anyhow::anyhow!(
                        "All models failed. Primary: {}, Fallbacks: {:?}. Error: {}",
                        config.model,
                        config.fallback_models,
                        e
                    ));
                }
                return Err(e);
            }
        };

        // Collect streamed response
        let mut response_text = String::new();
        while let Some(chunk) = stream.next().await {
            if !chunk.text.is_empty() {
                response_text.push_str(&chunk.text);
                let _ = tx.send(ChatEvent::Token {
                    text: chunk.text.clone(),
                });
            }
        }

        // Parse plan tags (<plan>[...]</plan>)
        if let Some(plan_items) = parse_plan(&response_text) {
            let _ = tx.send(ChatEvent::Plan { items: plan_items });
        }

        // Parse tool calls
        let tool_calls = parse_tool_calls(&response_text);

        // Send thinking text to sidebar (text before <tool_call>)
        if !tool_calls.is_empty() {
            if let Some(idx) = response_text.find("<tool_call>") {
                let thinking = response_text[..idx].trim();
                // Strip plan tags from thinking text
                let thinking = thinking
                    .replace(|_: char| false, ""); // no-op, just to own
                let thinking = strip_plan_tags(&thinking);
                if !thinking.is_empty() {
                    let _ = tx.send(ChatEvent::Thinking {
                        text: thinking,
                    });
                }
            }
        }

        // === Stop reason handling (OpenClaw pattern) ===

        if tool_calls.is_empty() {
            // STOP REASON: end_turn — model finished naturally
            session.ajouter_assistant(&response_text);

            // Emit Usage event with estimated tokens and cost
            let input_tokens = session.estimated_tokens() as u32;
            let output_tokens = (response_text.len() / 4) as u32;
            let cost_usd = (input_tokens as f32 / 1000.0) * config.cost_per_1k_input
                + (output_tokens as f32 / 1000.0) * config.cost_per_1k_output;
            let _ = tx.send(ChatEvent::Usage {
                input_tokens,
                output_tokens,
                cost_usd,
            });

            let _ = tx.send(ChatEvent::Done {
                full_response: response_text.clone(),
            });
            return Ok(response_text);
        }

        // STOP REASON: tool_use — execute tools and continue
        session.ajouter_assistant(&response_text);

        let ctx = ContextExecution::default();

        // Notify all tool calls
        for call in &tool_calls {
            let _ = tx.send(ChatEvent::ToolCall {
                name: call.name.clone(),
                args: call.args.clone(),
                iteration: Some(iteration),
            });
        }

        // Execute tools — parallel when multiple, sequential when single
        if tool_calls.len() > 1 {
            // PARALLEL execution (OpenClaw pattern)
            let _ = tx.send(ChatEvent::Status {
                message: format!("Executing {} tools in parallel...", tool_calls.len()),
            });

            let mut handles = Vec::new();
            for call in &tool_calls {
                // Check approval gating first
                if let Some(abeille) = registry.get(&call.name) {
                    if abeille.niveau_danger() == NiveauDanger::Dangerous {
                        session.ajouter_observation(&call.name, "Error: tool blocked (dangerous).");
                        let _ = tx.send(ChatEvent::ToolResult {
                            name: call.name.clone(),
                            result: "Blocked: dangerous tool.".to_string(),
                            success: false,
                            elapsed_ms: Some(0),
                        });
                        continue;
                    }
                }

                let name = call.name.clone();
                let args = call.args.clone();
                let ctx_clone = ctx.clone();
                let registry_ref = &registry;
                handles.push(async move {
                    let start = Instant::now();
                    let result = registry_ref.executer(&name, args, &ctx_clone).await;
                    let elapsed = start.elapsed().as_millis() as u64;
                    (name, result, elapsed)
                });
            }

            // Await all in parallel
            let results = futures_util::future::join_all(handles).await;

            for (name, result, elapsed) in results {
                match result {
                    Ok(res) => {
                        let _ = tx.send(ChatEvent::ToolResult {
                            name: name.clone(),
                            result: res.output.clone(),
                            success: res.success,
                            elapsed_ms: Some(elapsed),
                        });
                        let observation = if res.success {
                            res.output
                        } else {
                            format!("Error: {}", res.error.unwrap_or_else(|| "Unknown".to_string()))
                        };
                        session.ajouter_observation(&name, &observation);
                    }
                    Err(e) => {
                        let _ = tx.send(ChatEvent::ToolResult {
                            name: name.clone(),
                            result: format!("Error: {}", e),
                            success: false,
                            elapsed_ms: Some(elapsed),
                        });
                        session.ajouter_observation(&name, &format!("Error: {}", e));
                    }
                }
            }
        } else {
            // Single tool — sequential execution with approval gating
            for call in &tool_calls {
                if let Some(abeille) = registry.get(&call.name) {
                    let danger = abeille.niveau_danger();

                    // Dangerous = always blocked
                    if danger == NiveauDanger::Dangerous {
                        let _ = tx.send(ChatEvent::ToolResult {
                            name: call.name.clone(),
                            result: "Blocked: dangerous tool.".to_string(),
                            success: false,
                            elapsed_ms: Some(0),
                        });
                        session.ajouter_observation(&call.name, "Error: tool blocked (dangerous).");
                        continue;
                    }

                    // NeedsApproval = ask user via WebSocket, wait for response
                    if danger == NiveauDanger::NeedsApproval {
                        if let Some(ref mut rx) = approval_rx {
                            // Send approval request to UI
                            let _ = tx.send(ChatEvent::ApprovalRequest {
                                tool_call_id: call.id.clone(),
                                name: call.name.clone(),
                                args: call.args.clone(),
                            });

                            // Wait for approval (with 60s timeout)
                            let approval = tokio::time::timeout(
                                std::time::Duration::from_secs(60),
                                rx.recv(),
                            ).await;

                            match approval {
                                Ok(Some(resp)) if resp.approved => {
                                    tracing::info!(tool = %call.name, "Tool approved by user");
                                }
                                Ok(Some(_)) => {
                                    let _ = tx.send(ChatEvent::ToolResult {
                                        name: call.name.clone(),
                                        result: "Denied by user.".to_string(),
                                        success: false,
                                        elapsed_ms: Some(0),
                                    });
                                    session.ajouter_observation(&call.name, "Error: denied by user.");
                                    continue;
                                }
                                _ => {
                                    // Timeout or channel closed — auto-approve
                                    tracing::warn!(tool = %call.name, "Approval timeout — auto-approving");
                                }
                            }
                        }
                        // If no approval channel, auto-approve (backward compat)
                    }
                }

                let tool_start = Instant::now();
                let _ = tx.send(ChatEvent::Status {
                    message: format!("Executing: {}", call.name),
                });

                let result = registry.executer(&call.name, call.args.clone(), &ctx).await?;
                let elapsed = tool_start.elapsed().as_millis() as u64;

                let _ = tx.send(ChatEvent::ToolResult {
                    name: call.name.clone(),
                    result: result.output.clone(),
                    success: result.success,
                    elapsed_ms: Some(elapsed),
                });

                let observation = if result.success {
                    result.output
                } else {
                    format!("Error: {}", result.error.unwrap_or_else(|| "Unknown".to_string()))
                };
                session.ajouter_observation(&call.name, &observation);
            }
        }

        // Continue loop — LLM will see tool results in next iteration
    }

    // STOP REASON: max_iterations — forced stop
    let msg = format!(
        "Agent reached maximum iterations ({}). The task may be incomplete.",
        config.max_iterations
    );
    let _ = tx.send(ChatEvent::Error {
        message: msg.clone(),
    });
    Err(anyhow::anyhow!(msg))
}
