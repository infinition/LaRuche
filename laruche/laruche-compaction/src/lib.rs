use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: Role,
    pub content: String,
    pub approx_tokens: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompactMetadata {
    pub discovered_tools: BTreeSet<String>,
    pub invoked_skills: BTreeSet<String>,
    pub recalled_memories: BTreeSet<String>,
    pub active_tasks: BTreeSet<String>,
    pub files_touched: BTreeSet<String>,
    pub decisions: Vec<String>,
    pub errors_and_fixes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactSummary {
    pub primary_request: String,
    pub technical_context: Vec<String>,
    pub files_and_code: Vec<String>,
    pub errors_and_fixes: Vec<String>,
    pub pending_tasks: Vec<String>,
    pub current_work: String,
    pub preserved_metadata: CompactMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactedConversation {
    pub summary: CompactSummary,
    pub retained_messages: Vec<Message>,
    pub tokens_before: usize,
    pub tokens_after_estimate: usize,
}

#[derive(Debug, Clone)]
pub struct Compactor {
    retain_recent_messages: usize,
    target_tokens: usize,
}

impl Default for Compactor {
    fn default() -> Self {
        Self {
            retain_recent_messages: 12,
            target_tokens: 12_000,
        }
    }
}

impl Compactor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn retain_recent_messages(mut self, count: usize) -> Self {
        self.retain_recent_messages = count.max(1);
        self
    }

    pub fn target_tokens(mut self, target_tokens: usize) -> Self {
        self.target_tokens = target_tokens.max(1000);
        self
    }

    pub fn compact(
        &self,
        messages: &[Message],
        metadata: CompactMetadata,
    ) -> CompactedConversation {
        let tokens_before = messages.iter().map(|msg| msg.approx_tokens).sum();
        let split = messages.len().saturating_sub(self.retain_recent_messages);
        let summarized = &messages[..split];
        let retained_messages = messages[split..].to_vec();

        let summary = CompactSummary {
            primary_request: first_user_request(messages)
                .unwrap_or_else(|| "Unidentified request".to_string()),
            technical_context: collect_role_snippets(summarized, Role::Assistant, 8),
            files_and_code: metadata.files_touched.iter().cloned().collect(),
            errors_and_fixes: metadata.errors_and_fixes.clone(),
            pending_tasks: metadata.active_tasks.iter().cloned().collect(),
            current_work: retained_messages
                .last()
                .map(|msg| msg.content.clone())
                .unwrap_or_default(),
            preserved_metadata: metadata,
        };
        let summary_tokens = estimate_tokens(&format!("{summary:?}"));
        let retained_tokens: usize = retained_messages.iter().map(|msg| msg.approx_tokens).sum();
        CompactedConversation {
            summary,
            retained_messages,
            tokens_before,
            tokens_after_estimate: summary_tokens + retained_tokens,
        }
    }

    pub fn compact_with_events<S: CompactEventSink>(
        &self,
        messages: &[Message],
        metadata: CompactMetadata,
        sink: &mut S,
    ) -> Result<CompactedConversation> {
        sink.emit_compaction_event(
            CompactEventKind::Started,
            "compactor",
            serde_json::json!({"messages": messages.len()}),
        )?;
        let compacted = self.compact(messages, metadata);
        sink.emit_compaction_event(
            CompactEventKind::Finished,
            "compactor",
            serde_json::json!({
                "tokens_before": compacted.tokens_before,
                "tokens_after_estimate": compacted.tokens_after_estimate
            }),
        )?;
        Ok(compacted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompactEventKind {
    Started,
    Finished,
}

pub trait CompactEventSink {
    fn emit_compaction_event(
        &mut self,
        kind: CompactEventKind,
        actor: &str,
        payload: serde_json::Value,
    ) -> Result<()>;
}

pub trait BudgetStatusLike {
    fn ratio(&self) -> f32;
    fn critical(&self) -> bool;
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct CompactionBudgetStatus {
    pub used: usize,
    pub max: usize,
    pub ratio: f32,
    pub warn: bool,
    pub critical: bool,
}

impl BudgetStatusLike for CompactionBudgetStatus {
    fn ratio(&self) -> f32 {
        self.ratio
    }

    fn critical(&self) -> bool {
        self.critical
    }
}

pub fn doit_compacter<B: BudgetStatusLike + ?Sized>(
    messages: &[Value],
    ratio_seuil: f32,
    budget: &B,
) -> bool {
    !messages.is_empty() && (budget.critical() || budget.ratio() >= ratio_seuil)
}

pub fn micro_compacter(messages: Vec<Value>, garder_n_derniers: usize) -> Vec<Value> {
    let split = messages.len().saturating_sub(garder_n_derniers);
    messages
        .into_iter()
        .enumerate()
        .map(|(index, message)| {
            if index >= split {
                message
            } else {
                compacter_anciens_resultats_outil(message)
            }
        })
        .collect()
}

pub fn nettoyage_post_compaction(messages: Vec<Value>) -> Vec<Value> {
    let mut vus = BTreeSet::new();
    let mut tool_uses_vus = BTreeSet::new();
    let mut nettoyes = Vec::with_capacity(messages.len());

    for message in messages {
        collecter_tool_use_ids(&message, &mut tool_uses_vus);

        if let Some(tool_use_id) = tool_result_id(&message) {
            if !tool_uses_vus.contains(&tool_use_id) {
                continue;
            }
        }

        let key = serde_json::to_string(&message).unwrap_or_default();
        if vus.insert(key) {
            nettoyes.push(message);
        }
    }

    nettoyes
}

pub fn compresser_trajectoire<F>(messages: Vec<Value>, aux_resumer: F) -> Vec<Value>
where
    F: Fn(&str) -> String,
{
    let mut compressees = Vec::with_capacity(messages.len());
    let mut i = 0;
    while i < messages.len() {
        if i + 1 < messages.len()
            && message_contient_tool_use(&messages[i])
            && tool_result_id(&messages[i + 1]).is_some()
        {
            let payload = format!(
                "{}\n\n{}",
                serde_json::to_string(&messages[i]).unwrap_or_default(),
                serde_json::to_string(&messages[i + 1]).unwrap_or_default()
            );
            let summary = aux_resumer(&payload);
            compressees.push(serde_json::json!({
                "role": "system",
                "type": "trajectory_summary",
                "content": format!("[Compressed tool step]\n{}", summary.trim())
            }));
            i += 2;
        } else {
            compressees.push(messages[i].clone());
            i += 1;
        }
    }
    compressees
}

#[derive(Debug, Clone)]
pub struct ToolResultStore {
    root: PathBuf,
    preview_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToolResult {
    pub path: PathBuf,
    pub original_bytes: usize,
    pub preview: String,
    pub truncated: bool,
}

impl ToolResultStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            preview_bytes: 2_000,
        }
    }

    pub fn preview_bytes(mut self, preview_bytes: usize) -> Self {
        self.preview_bytes = preview_bytes.max(1);
        self
    }

    pub fn persist_if_large(
        &self,
        tool_use_id: &str,
        content: &str,
        threshold_bytes: usize,
    ) -> Result<Option<StoredToolResult>> {
        if content.len() <= threshold_bytes {
            return Ok(None);
        }
        fs::create_dir_all(&self.root)?;
        let path = self.root.join(format!("{tool_use_id}.txt"));
        fs::write(&path, content)?;
        let preview: String = content.chars().take(self.preview_bytes).collect();
        Ok(Some(StoredToolResult {
            path,
            original_bytes: content.len(),
            truncated: content.len() > preview.len(),
            preview,
        }))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Message {
    pub fn new(id: impl Into<String>, role: Role, content: impl Into<String>) -> Self {
        let content = content.into();
        Self {
            id: id.into(),
            role,
            approx_tokens: estimate_tokens(&content),
            content,
        }
    }
}

fn first_user_request(messages: &[Message]) -> Option<String> {
    messages
        .iter()
        .find(|msg| msg.role == Role::User)
        .map(|msg| msg.content.clone())
}

fn collect_role_snippets(messages: &[Message], role: Role, limit: usize) -> Vec<String> {
    messages
        .iter()
        .filter(|msg| msg.role == role)
        .take(limit)
        .map(|msg| msg.content.chars().take(240).collect())
        .collect()
}

fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

const TOOL_RESULT_PLACEHOLDER: &str = "[Old tool result compacted]";

fn compacter_anciens_resultats_outil(mut message: Value) -> Value {
    if est_message_resultat_outil(&message) {
        remplacer_content_par_placeholder(&mut message);
        return message;
    }

    remplacer_blocs_tool_result(&mut message);
    message
}

fn est_message_resultat_outil(message: &Value) -> bool {
    role(message)
        .map(|role| role == "tool" || role == "observation")
        .unwrap_or(false)
        || type_json(message)
            .map(|kind| kind == "tool_result")
            .unwrap_or(false)
        || message.get("tool_use_id").is_some()
        || message.get("tool_call_id").is_some()
}

fn remplacer_content_par_placeholder(value: &mut Value) {
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "content".to_string(),
            Value::String(TOOL_RESULT_PLACEHOLDER.to_string()),
        );
    }
}

fn remplacer_blocs_tool_result(value: &mut Value) -> bool {
    let Some(obj) = value.as_object_mut() else {
        return false;
    };

    if type_json_obj(obj)
        .map(|kind| kind == "tool_result")
        .unwrap_or(false)
    {
        obj.insert(
            "content".to_string(),
            Value::String(TOOL_RESULT_PLACEHOLDER.to_string()),
        );
        return true;
    }

    let Some(content) = obj.get_mut("content") else {
        return false;
    };

    match content {
        Value::Array(blocks) => blocks
            .iter_mut()
            .map(remplacer_blocs_tool_result)
            .fold(false, |acc, touched| acc || touched),
        Value::Object(_) => remplacer_blocs_tool_result(content),
        _ => false,
    }
}

fn collecter_tool_use_ids(value: &Value, ids: &mut BTreeSet<String>) {
    if type_json(value)
        .map(|kind| kind == "tool_use")
        .unwrap_or(false)
    {
        if let Some(id) = value.get("id").and_then(Value::as_str) {
            ids.insert(id.to_string());
        }
    }

    if let Some(content) = value.get("content") {
        match content {
            Value::Array(blocks) => {
                for block in blocks {
                    collecter_tool_use_ids(block, ids);
                }
            }
            Value::Object(_) => collecter_tool_use_ids(content, ids),
            _ => {}
        }
    }
}

fn message_contient_tool_use(value: &Value) -> bool {
    if type_json(value)
        .map(|kind| kind == "tool_use")
        .unwrap_or(false)
    {
        return true;
    }
    match value.get("content") {
        Some(Value::Array(blocks)) => blocks.iter().any(message_contient_tool_use),
        Some(Value::Object(_)) => message_contient_tool_use(value.get("content").unwrap()),
        _ => false,
    }
}

fn tool_result_id(value: &Value) -> Option<String> {
    if type_json(value)
        .map(|kind| kind == "tool_result")
        .unwrap_or(false)
        || est_message_resultat_outil(value)
    {
        if let Some(id) = value
            .get("tool_use_id")
            .or_else(|| value.get("tool_call_id"))
        {
            return id.as_str().map(str::to_string);
        }
    }

    match value.get("content") {
        Some(Value::Array(blocks)) => blocks.iter().find_map(tool_result_id),
        Some(Value::Object(_)) => tool_result_id(value.get("content")?),
        _ => None,
    }
}

fn role(value: &Value) -> Option<String> {
    value.get("role")?.as_str().map(|role| role.to_lowercase())
}

fn type_json(value: &Value) -> Option<&str> {
    value.get("type")?.as_str()
}

fn type_json_obj(obj: &serde_json::Map<String, Value>) -> Option<&str> {
    obj.get("type")?.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compaction_preserves_metadata_and_recent_messages() {
        let messages = (0..20)
            .map(|i| {
                Message::new(
                    format!("m-{i}"),
                    if i == 0 { Role::User } else { Role::Assistant },
                    format!("message {i}"),
                )
            })
            .collect::<Vec<_>>();
        let mut metadata = CompactMetadata::default();
        metadata.discovered_tools.insert("memory_recall".into());
        metadata.active_tasks.insert("Finir memory update".into());
        metadata.files_touched.insert("src/lib.rs".into());
        let compacted = Compactor::new()
            .retain_recent_messages(5)
            .compact(&messages, metadata);
        assert_eq!(compacted.retained_messages.len(), 5);
        assert_eq!(compacted.summary.primary_request, "message 0");
        assert!(compacted
            .summary
            .files_and_code
            .contains(&"src/lib.rs".into()));
        assert!(compacted
            .summary
            .preserved_metadata
            .discovered_tools
            .contains("memory_recall"));
    }

    #[test]
    fn tool_result_store_persists_large_outputs_to_disk() {
        let root =
            std::env::temp_dir().join(format!("laruche-compaction-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let store = ToolResultStore::new(&root).preview_bytes(4);
        let stored = store
            .persist_if_large("tool-1", "abcdef", 3)
            .unwrap()
            .unwrap();
        assert_eq!(stored.preview, "abcd");
        assert!(stored.truncated);
        assert_eq!(fs::read_to_string(stored.path).unwrap(), "abcdef");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tool_result_store_keeps_small_outputs_inline() {
        let store = ToolResultStore::new(std::env::temp_dir());
        assert!(store
            .persist_if_large("tool-1", "small", 20)
            .unwrap()
            .is_none());
    }

    #[derive(Default)]
    struct RecordingSink {
        events: Vec<(CompactEventKind, String, serde_json::Value)>,
    }

    impl CompactEventSink for RecordingSink {
        fn emit_compaction_event(
            &mut self,
            kind: CompactEventKind,
            actor: &str,
            payload: serde_json::Value,
        ) -> Result<()> {
            self.events.push((kind, actor.to_string(), payload));
            Ok(())
        }
    }

    #[test]
    fn compact_with_events_emits_start_and_finish() {
        let messages = vec![Message::new("m-1", Role::User, "bonjour")];
        let mut sink = RecordingSink::default();
        let compacted = Compactor::new()
            .compact_with_events(&messages, CompactMetadata::default(), &mut sink)
            .unwrap();
        assert_eq!(compacted.retained_messages.len(), 1);
        assert_eq!(sink.events.len(), 2);
        assert_eq!(sink.events[0].0, CompactEventKind::Started);
        assert_eq!(sink.events[1].0, CompactEventKind::Finished);
    }

    #[test]
    fn doit_compacter_suit_le_ratio_ou_critical() {
        let messages = vec![serde_json::json!({"role": "user", "content": "bonjour"})];
        let low = CompactionBudgetStatus {
            ratio: 0.5,
            ..Default::default()
        };
        let critical = CompactionBudgetStatus {
            ratio: 0.2,
            critical: true,
            ..Default::default()
        };

        assert!(!doit_compacter(&messages, 0.75, &low));
        assert!(doit_compacter(&messages, 0.75, &critical));
    }

    #[test]
    fn micro_compacter_reduit_anciens_resultats_et_preserve_les_derniers() {
        let ancien = serde_json::json!({
            "role": "tool",
            "tool_use_id": "tool-1",
            "content": "x".repeat(2_000)
        });
        let recent_user = serde_json::json!({"role": "user", "content": "question recente"});
        let recent_assistant =
            serde_json::json!({"role": "assistant", "content": "reponse recente"});
        let messages = vec![
            ancien.clone(),
            recent_user.clone(),
            recent_assistant.clone(),
        ];
        let taille_avant = serde_json::to_string(&messages).unwrap().len();

        let compactes = micro_compacter(messages, 2);
        let taille_apres = serde_json::to_string(&compactes).unwrap().len();

        assert!(taille_apres < taille_avant);
        assert_eq!(compactes[1], recent_user);
        assert_eq!(compactes[2], recent_assistant);
        assert_eq!(
            compactes[0]["content"],
            Value::String(TOOL_RESULT_PLACEHOLDER.to_string())
        );
    }

    #[test]
    fn nettoyage_post_compaction_retire_doublons_et_observations_orphelines() {
        let assistant = serde_json::json!({
            "role": "assistant",
            "content": [{"type": "tool_use", "id": "tool-1", "name": "shell_exec"}]
        });
        let observation = serde_json::json!({
            "role": "tool",
            "tool_use_id": "tool-1",
            "content": "ok"
        });
        let orpheline = serde_json::json!({
            "role": "tool",
            "tool_use_id": "tool-absent",
            "content": "orpheline"
        });

        let nettoyes = nettoyage_post_compaction(vec![
            assistant.clone(),
            assistant.clone(),
            observation.clone(),
            orpheline,
        ]);

        assert_eq!(nettoyes, vec![assistant.clone(), observation]);
    }

    #[test]
    fn compresser_trajectoire_resume_action_observation() {
        let assistant = serde_json::json!({
            "role": "assistant",
            "content": [{"type": "tool_use", "id": "tool-1", "name": "shell_exec"}]
        });
        let observation = serde_json::json!({
            "role": "tool",
            "tool_use_id": "tool-1",
            "content": "cargo test ok"
        });
        let final_msg = serde_json::json!({"role": "assistant", "content": "termine"});

        let compressees =
            compresser_trajectoire(vec![assistant, observation, final_msg.clone()], |payload| {
                assert!(payload.contains("shell_exec"));
                "A lance les tests avec succes".to_string()
            });

        assert_eq!(compressees.len(), 2);
        assert_eq!(compressees[0]["type"], "trajectory_summary");
        assert!(compressees[0]["content"]
            .as_str()
            .unwrap()
            .contains("A lance les tests"));
        assert_eq!(compressees[1], final_msg);
    }
}
