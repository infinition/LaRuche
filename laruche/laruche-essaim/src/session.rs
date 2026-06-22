use anyhow::Result;
use chrono::{DateTime, Utc};
use laruche_compaction::{
    CompactMetadata, CompactSummary, Compactor, Message as CompactMessage, Role, ToolResultStore,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub kind: String, // "image", "audio", "file"
    pub mime_type: String,
    pub data: String, // Base64 data
    pub filename: Option<String>,
}

/// A single message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", content = "content")]
pub enum Message {
    #[serde(rename = "system")]
    System(String),
    #[serde(rename = "user")]
    User(String),
    /// User message with multimodal attachments (images, audio, files)
    #[serde(rename = "user_multimodal")]
    UserMultimodal {
        text: String,
        #[serde(default)]
        attachments: Vec<Attachment>,
    },
    #[serde(rename = "assistant")]
    Assistant(String),
    /// A sanitized, user-visible agent step for restoring the workflow timeline.
    #[serde(rename = "thought")]
    Thought {
        phase: String,
        kind: String,
        text: String,
    },
    /// Exact first-turn payload, retained for the user-facing prompt inspector.
    #[serde(rename = "prompt_debug")]
    PromptDebug {
        payload: serde_json::Value,
        model: String,
        provider: String,
    },
    #[serde(rename = "tool_call")]
    ToolCall {
        name: String,
        args: serde_json::Value,
    },
    #[serde(rename = "observation")]
    Observation {
        tool: String,
        result: String,
        #[serde(default)]
        images: Vec<String>,
    },
}

/// A conversation session with persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub model: String,
    pub title: Option<String>,
    /// Owner user ID (None = legacy/anonymous session, visible to all)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
    /// Optional session-specific working directory (for worktree isolation)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<PathBuf>,
    #[serde(skip)]
    file_path: Option<PathBuf>,
    #[serde(skip)]
    pub event_tx: Option<tokio::sync::broadcast::Sender<crate::ChatEvent>>,
}

fn approx_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

fn message_to_compact(idx: usize, msg: &Message) -> CompactMessage {
    let (role, content) = match msg {
        Message::System(text) => (Role::System, text.clone()),
        Message::User(text) => (Role::User, text.clone()),
        Message::UserMultimodal { text, attachments } => (
            Role::User,
            format!("{text}\n[{} attachment(s)]", attachments.len()),
        ),
        Message::Assistant(text) => (Role::Assistant, text.clone()),
        Message::Thought { phase, kind, text } => {
            (Role::System, format!("[Agent step {phase}/{kind}] {text}"))
        }
        Message::PromptDebug { .. } => (Role::System, "[Prompt inspector payload]".to_string()),
        Message::ToolCall { name, args } => {
            (Role::Assistant, format!("Tool call `{name}`: {args}"))
        }
        Message::Observation { tool, result, .. } => {
            (Role::Tool, format!("[Tool Result: {tool}]\n{result}"))
        }
    };
    CompactMessage {
        id: format!("m-{idx}"),
        approx_tokens: approx_tokens(&content),
        role,
        content,
    }
}

fn render_compact_summary(
    summary: &CompactSummary,
    tokens_before: usize,
    tokens_after_estimate: usize,
) -> String {
    let mut out = String::new();
    out.push_str("[Resume structure de la conversation compactee]\n");
    out.push_str(&format!(
        "Tokens: {tokens_before} -> ~{tokens_after_estimate}\n\n"
    ));
    out.push_str("## Demande principale\n");
    out.push_str(&summary.primary_request);
    out.push_str("\n\n## Contexte technique\n");
    push_list(&mut out, &summary.technical_context);
    out.push_str("\n## Fichiers et code\n");
    push_list(&mut out, &summary.files_and_code);
    out.push_str("\n## Erreurs et corrections\n");
    push_list(&mut out, &summary.errors_and_fixes);
    out.push_str("\n## Taches en attente\n");
    push_list(&mut out, &summary.pending_tasks);
    out.push_str("\n## Travail courant\n");
    out.push_str(&summary.current_work);
    out
}

fn push_list(out: &mut String, items: &[String]) {
    if items.is_empty() {
        out.push_str("- Aucun element preserve.\n");
    } else {
        for item in items {
            out.push_str("- ");
            out.push_str(item);
            out.push('\n');
        }
    }
}

impl Session {
    /// Create a new empty session.
    pub fn new(model: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            messages: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            model: model.to_string(),
            title: None,
            user_id: None,
            working_dir: None,
            file_path: None,
            event_tx: None,
        }
    }

    /// Create a new session with a persistence path.
    pub fn new_with_path(model: &str, dir: &Path) -> Self {
        let mut session = Self::new(model);
        let file_path = dir.join(format!("{}.json", session.id));
        session.file_path = Some(file_path);
        session
    }

    /// Create a session with a specific ID and persistence path.
    pub fn new_with_id(id: Uuid, model: &str, dir: &Path) -> Self {
        let mut session = Self::new(model);
        session.id = id;
        session.file_path = Some(dir.join(format!("{}.json", id)));
        session
    }

    /// Add a user message (text only).
    pub fn ajouter_user(&mut self, text: &str) {
        self.messages.push(Message::User(text.to_string()));
        self.updated_at = Utc::now();
    }

    /// Add a user message with images (multimodal).
    pub fn ajouter_user_multimodal(&mut self, text: &str, attachments: Vec<Attachment>) {
        if attachments.is_empty() {
            self.ajouter_user(text);
        } else {
            self.messages.push(Message::UserMultimodal {
                text: text.to_string(),
                attachments,
            });
            self.updated_at = Utc::now();
        }
    }

    /// Add an assistant response.
    pub fn ajouter_assistant(&mut self, text: &str) {
        self.messages.push(Message::Assistant(text.to_string()));
        self.updated_at = Utc::now();
    }

    /// Records an attempted tool invocation so the agent workflow can be
    /// reconstructed after a browser refresh.
    pub fn ajouter_tool_call(&mut self, name: &str, args: serde_json::Value) {
        self.messages.push(Message::ToolCall {
            name: name.to_string(),
            args,
        });
        self.updated_at = Utc::now();
    }

    /// Records a sanitized agent step for the reloadable workflow timeline.
    pub fn ajouter_thought(&mut self, phase: &str, kind: &str, text: &str) {
        self.messages.push(Message::Thought {
            phase: phase.to_string(),
            kind: kind.to_string(),
            text: text.to_string(),
        });
        self.updated_at = Utc::now();
    }

    /// Stores the exact first-turn LLM payload for the prompt inspector after reload.
    pub fn ajouter_prompt_debug(
        &mut self,
        payload: serde_json::Value,
        model: String,
        provider: String,
    ) {
        self.messages.push(Message::PromptDebug {
            payload,
            model,
            provider,
        });
        self.updated_at = Utc::now();
    }

    /// Add a tool call observation (tool name + result).
    pub fn ajouter_observation(&mut self, tool: &str, result: &str) {
        self.ajouter_observation_avec_images(tool, result, vec![]);
    }

    pub fn ajouter_observation_avec_images(
        &mut self,
        tool: &str,
        result: &str,
        images: Vec<String>,
    ) {
        self.messages.push(Message::Observation {
            tool: tool.to_string(),
            result: result.to_string(),
            images,
        });
        self.updated_at = Utc::now();
    }

    /// Build the messages array for the Ollama /api/chat endpoint.
    /// Includes system prompt with tools schema, then the LAST `max_history` messages.
    /// This prevents context overflow on small models like Gemma 4 E4B.
    pub fn build_ollama_messages(&self, system_prompt: &str) -> Vec<serde_json::Value> {
        let mut msgs = vec![serde_json::json!({
            "role": "system",
            "content": system_prompt,
        })];

        // Limit history to avoid context overflow.
        // Keep the last N messages — enough for multi-turn but not too much for small models.
        let max_history = 30;
        let skip = if self.messages.len() > max_history {
            self.messages.len() - max_history
        } else {
            0
        };

        // If we skipped messages, add a summary reminder
        if skip > 0 {
            msgs.push(serde_json::json!({
                "role": "system",
                "content": format!(
                    "[Note: {} earlier messages were omitted to fit context. \
                     Focus on the recent conversation. You still have access to all your tools — \
                     use them when needed.]",
                    skip
                ),
            }));
        }

        for msg in self.messages.iter().skip(skip) {
            match msg {
                Message::System(text) => {
                    msgs.push(serde_json::json!({
                        "role": "system",
                        "content": text,
                    }));
                }
                Message::User(text) => {
                    msgs.push(serde_json::json!({
                        "role": "user",
                        "content": text,
                    }));
                }
                Message::UserMultimodal { text, attachments } => {
                    // Ollama multimodal format: images as base64 array (we extract only images for ollama)
                    let images: Vec<String> = attachments
                        .iter()
                        .filter(|a| a.kind == "image")
                        .map(|a| a.data.clone())
                        .collect();
                    msgs.push(serde_json::json!({
                        "role": "user",
                        "content": text,
                        "images": images,
                        "attachments": attachments,
                    }));
                }
                Message::Assistant(text) => {
                    // Strip <tool_call> blocks entirely from assistant text.
                    // The tool results are already stored as Observation messages.
                    let mut clean = text.clone();
                    while let Some(start) = clean.find("<tool_call>") {
                        if let Some(end) = clean.find("</tool_call>") {
                            let after = &clean[end + "</tool_call>".len()..];
                            clean = format!("{}{}", &clean[..start], after);
                        } else {
                            clean.truncate(start);
                            break;
                        }
                    }
                    let trimmed = clean.trim();
                    if !trimmed.is_empty() {
                        msgs.push(serde_json::json!({
                            "role": "assistant",
                            "content": trimmed,
                        }));
                    }
                }
                Message::Thought { .. } => {
                    // Workflow display metadata is not part of the model context.
                }
                Message::PromptDebug { .. } => {
                    // Debug snapshots are UI-only and must not enter the LLM context.
                }
                Message::ToolCall { name, args } => {
                    // Tool calls are part of the assistant message that triggered them
                    // Already included in the assistant text via <tool_call> tags
                    let _ = (name, args);
                }
                Message::Observation {
                    tool,
                    result,
                    images,
                } => {
                    // Observations from tools are injected as user messages.
                    // Truncate long results to prevent context explosion.
                    let truncated: String = result.chars().take(2000).collect();
                    let content = if result.len() > 2000 {
                        format!(
                            "[Tool Result: {}]\n{}...\n(truncated, {} chars total)",
                            tool,
                            truncated,
                            result.len()
                        )
                    } else {
                        format!("[Tool Result: {}]\n{}", tool, result)
                    };
                    if !images.is_empty() {
                        msgs.push(serde_json::json!({
                            "role": "user",
                            "content": content,
                            "images": images
                        }));
                    } else {
                        msgs.push(serde_json::json!({
                            "role": "user",
                            "content": content
                        }));
                    }
                }
            }
        }

        msgs
    }

    /// Auto-generate a title from the first user message.
    pub fn auto_title(&mut self) {
        if self.title.is_some() {
            return;
        }
        for msg in &self.messages {
            if let Message::User(text) = msg {
                let title: String = text.chars().take(60).collect();
                self.title = Some(if text.len() > 60 {
                    format!("{}...", title)
                } else {
                    title
                });
                return;
            }
        }
    }

    /// Save session to JSONL file.
    pub fn sauvegarder(&self) -> Result<()> {
        let path = match &self.file_path {
            Some(p) => p,
            None => return Ok(()),
        };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string(self)?;
        std::fs::write(path, json)?;
        tracing::debug!(session_id = %self.id, path = %path.display(), "Session saved");
        Ok(())
    }

    /// Load session from file.
    pub fn charger(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut session: Session = serde_json::from_str(&content)?;
        session.file_path = Some(path.to_path_buf());
        Ok(session)
    }

    /// Compact conversation: keep only the last N message pairs to avoid context overflow.
    pub fn compacter(&mut self, garder_n_derniers: usize) {
        if self.messages.len() <= garder_n_derniers {
            return;
        }

        let original_len = self.messages.len();
        let store = ToolResultStore::new(self.tool_result_store_root());
        let mut messages = std::mem::take(&mut self.messages);
        for (idx, msg) in messages.iter_mut().enumerate() {
            if let Message::Observation { tool, result, .. } = msg {
                if let Ok(Some(stored)) =
                    store.persist_if_large(&format!("{}-{idx}-{tool}", self.id), result, 4_000)
                {
                    *result = format!(
                        "{}\n\n[Resultat complet externalise: {} ({} octets)]",
                        stored.preview,
                        stored.path.display(),
                        stored.original_bytes
                    );
                }
            }
        }

        let compact_messages: Vec<CompactMessage> = messages
            .iter()
            .enumerate()
            .map(|(idx, msg)| message_to_compact(idx, msg))
            .collect();
        let compacted = Compactor::new()
            .retain_recent_messages(garder_n_derniers)
            .compact(&compact_messages, CompactMetadata::default());

        let keep_from = messages.len().saturating_sub(garder_n_derniers.max(1));
        let mut kept = Vec::with_capacity(garder_n_derniers + 1);
        kept.push(Message::System(render_compact_summary(
            &compacted.summary,
            compacted.tokens_before,
            compacted.tokens_after_estimate,
        )));
        kept.extend(messages.drain(keep_from..));
        self.messages = kept;
        self.updated_at = Utc::now();
        tracing::info!(
            session_id = %self.id,
            removed = original_len.saturating_sub(self.messages.len()),
            remaining = self.messages.len(),
            tokens_before = compacted.tokens_before,
            tokens_after_estimate = compacted.tokens_after_estimate,
            "Session compacted"
        );
    }

    fn tool_result_store_root(&self) -> PathBuf {
        self.file_path
            .as_ref()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .join("tool-results")
            .join(self.id.to_string())
    }

    /// Number of messages.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Fork (branch) a session: creates a copy with a new ID and all messages so far.
    pub fn fork(&self, model: &str, dir: &Path) -> Self {
        let mut forked = self.clone();
        forked.id = Uuid::new_v4();
        forked.file_path = Some(dir.join(format!("{}.json", forked.id)));
        forked.title = self.title.as_ref().map(|t| format!("{} (fork)", t));
        forked.created_at = Utc::now();
        forked.model = model.to_string();
        forked
    }

    /// Estimate the total token count of the session.
    pub fn estimated_tokens(&self) -> usize {
        let total_chars: usize = self
            .messages
            .iter()
            .map(|m| match m {
                Message::System(t) | Message::User(t) | Message::Assistant(t) => t.len(),
                Message::Thought { .. } => 0,
                Message::PromptDebug { .. } => 0,
                Message::UserMultimodal { text, attachments } => {
                    text.len() + attachments.iter().map(|a| a.data.len()).sum::<usize>() / 3
                }
                Message::ToolCall { name, args } => name.len() + args.to_string().len(),
                Message::Observation { tool, result, .. } => tool.len() + result.len(),
            })
            .sum();
        total_chars / 4
    }
}
