use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// A single message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", content = "content")]
pub enum Message {
    #[serde(rename = "system")]
    System(String),
    #[serde(rename = "user")]
    User(String),
    /// User message with images (base64-encoded)
    #[serde(rename = "user_multimodal")]
    UserMultimodal {
        text: String,
        images: Vec<String>,
    },
    #[serde(rename = "assistant")]
    Assistant(String),
    #[serde(rename = "tool_call")]
    ToolCall {
        name: String,
        args: serde_json::Value,
    },
    #[serde(rename = "observation")]
    Observation { tool: String, result: String },
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
    #[serde(skip)]
    file_path: Option<PathBuf>,
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
            file_path: None,
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
    pub fn ajouter_user_multimodal(&mut self, text: &str, images: Vec<String>) {
        if images.is_empty() {
            self.ajouter_user(text);
        } else {
            self.messages.push(Message::UserMultimodal {
                text: text.to_string(),
                images,
            });
            self.updated_at = Utc::now();
        }
    }

    /// Add an assistant response.
    pub fn ajouter_assistant(&mut self, text: &str) {
        self.messages.push(Message::Assistant(text.to_string()));
        self.updated_at = Utc::now();
    }

    /// Add a tool call observation (tool name + result).
    pub fn ajouter_observation(&mut self, tool: &str, result: &str) {
        self.messages.push(Message::Observation {
            tool: tool.to_string(),
            result: result.to_string(),
        });
        self.updated_at = Utc::now();
    }

    /// Build the messages array for the Ollama /api/chat endpoint.
    /// Includes system prompt with tools schema, then the LAST `max_history` messages.
    /// This prevents context overflow on small models like Gemma 4 E4B.
    pub fn build_ollama_messages(
        &self,
        system_prompt: &str,
    ) -> Vec<serde_json::Value> {
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
                Message::UserMultimodal { text, images } => {
                    // Ollama multimodal format: images as base64 array
                    msgs.push(serde_json::json!({
                        "role": "user",
                        "content": text,
                        "images": images,
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
                Message::ToolCall { name, args } => {
                    // Tool calls are part of the assistant message that triggered them
                    // Already included in the assistant text via <tool_call> tags
                    let _ = (name, args);
                }
                Message::Observation { tool, result } => {
                    // Observations from tools are injected as user messages.
                    // Truncate long results to prevent context explosion.
                    let truncated: String = result.chars().take(2000).collect();
                    let content = if result.len() > 2000 {
                        format!("[Tool Result: {}]\n{}...\n(truncated, {} chars total)", tool, truncated, result.len())
                    } else {
                        format!("[Tool Result: {}]\n{}", tool, result)
                    };
                    msgs.push(serde_json::json!({
                        "role": "user",
                        "content": content,
                    }));
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
        let to_remove = self.messages.len() - garder_n_derniers;
        // Keep system messages, remove oldest non-system messages
        let mut kept = Vec::new();
        let mut removed = 0;
        for msg in self.messages.drain(..) {
            if removed < to_remove {
                if matches!(msg, Message::System(_)) {
                    kept.push(msg);
                } else {
                    removed += 1;
                }
            } else {
                kept.push(msg);
            }
        }
        self.messages = kept;
        tracing::info!(
            session_id = %self.id,
            removed,
            remaining = self.messages.len(),
            "Session compacted"
        );
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
    /// Uses a simple heuristic: ~4 chars per token (rough average for English/French).
    pub fn estimated_tokens(&self) -> usize {
        let total_chars: usize = self.messages.iter().map(|m| {
            match m {
                Message::System(t) | Message::User(t) | Message::Assistant(t) => t.len(),
                Message::UserMultimodal { text, images } => text.len() + images.len() * 1000, // Images are ~1000 tokens each
                Message::ToolCall { name, args } => name.len() + args.to_string().len(),
                Message::Observation { tool, result } => tool.len() + result.len(),
            }
        }).sum();
        total_chars / 4
    }
}
