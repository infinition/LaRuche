//! Cognitive fatigue and memory consolidation for the ReAct loop.
//!
//! ## FatigueMonitor
//! Detects when the agent is spinning (repetitions, errors, saturated context)
//! and triggers a consolidation into persistent memory.
//!
//! ## Consolidation
//! Extracts durable facts from the history via an auxiliary LLM, writes them
//! to cognitive memory, and produces a fresh context (~500 tokens) so the
//! agent can restart cleanly without losing its discoveries.

use crate::brain::{EssaimConfig, ToolCall};
use crate::providers::provider_chat_stream;
use anyhow::Result;
use futures_util::StreamExt;
use laruche_memoire::{MemoireCognitive, MemoryItem};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Monitors the agent's cognitive fatigue.
#[derive(Debug, Clone)]
pub struct FatigueMonitor {
    pub iterations: u32,
    pub tool_failures: u32,
    pub repetition_score: f32,
    pub tokens_used: usize,
    recent_tool_names: Vec<String>,
    recent_results: Vec<bool>,
}

impl FatigueMonitor {
    const RECENT_WINDOW: usize = 12;

    pub fn new() -> Self {
        Self {
            iterations: 0,
            tool_failures: 0,
            repetition_score: 0.0,
            tokens_used: 0,
            recent_tool_names: Vec::with_capacity(Self::RECENT_WINDOW),
            recent_results: Vec::with_capacity(Self::RECENT_WINDOW),
        }
    }

    /// Updates the monitor after each tool execution.
    pub fn update(
        &mut self,
        tool_calls: &[ToolCall],
        tool_successes: &[bool],
        tokens_used: usize,
        iteration: u32,
    ) {
        self.iterations = iteration;
        self.tokens_used = tokens_used;

        for (call, &success) in tool_calls.iter().zip(tool_successes.iter()) {
            self.recent_tool_names.push(call.name.clone());
            self.recent_results.push(success);
        }

        while self.recent_tool_names.len() > Self::RECENT_WINDOW {
            self.recent_tool_names.remove(0);
            self.recent_results.remove(0);
        }

        self.tool_failures = self
            .recent_results
            .iter()
            .rev()
            .take(5)
            .filter(|&&s| !s)
            .count() as u32;

        self.update_repetition_score();
    }

    /// Simplified version (without success tracker) for the internal loop.
    pub fn update_names(
        &mut self,
        tool_names: &[String],
        tokens_used: usize,
        iteration: u32,
    ) {
        self.iterations = iteration;
        self.tokens_used = tokens_used;

        for name in tool_names {
            self.recent_tool_names.push(name.clone());
        }

        while self.recent_tool_names.len() > Self::RECENT_WINDOW {
            self.recent_tool_names.remove(0);
        }

        self.update_repetition_score();
    }

    fn update_repetition_score(&mut self) {
        let window = &self.recent_tool_names;
        if window.is_empty() {
            self.repetition_score = 0.0;
        } else {
            let unique: std::collections::HashSet<&str> =
                window.iter().map(String::as_str).collect();
            self.repetition_score = 1.0 - (unique.len() as f32 / window.len() as f32);
        }
    }

    /// Computes the overall fatigue level (0.0 = fresh, 1.0 = exhausted).
    pub fn fatigue_level(&self, config: &EssaimConfig) -> f32 {
        let mut score = 0.0f32;
        score += (self.iterations as f32 / config.max_iterations.max(1) as f32).min(1.0) * 0.25;
        let ctx_ratio = if config.context_max_tokens > 0 {
            (self.tokens_used as f32 / config.context_max_tokens as f32).min(1.0)
        } else {
            0.0
        };
        score += ctx_ratio * 0.35;
        score += (self.tool_failures as f32 / 5.0).min(1.0) * 0.20;
        score += self.repetition_score * 0.20;
        score.clamp(0.0, 1.0)
    }

    pub fn should_consolidate(&self, config: &EssaimConfig) -> bool {
        self.fatigue_level(config) > 0.72
    }

    pub fn is_critical(&self, config: &EssaimConfig) -> bool {
        self.fatigue_level(config) > 0.92
    }

    pub fn reset(&mut self) {
        self.tool_failures = 0;
        self.repetition_score = 0.0;
        self.recent_tool_names.clear();
        self.recent_results.clear();
    }
}

impl Default for FatigueMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a cognitive consolidation.
#[derive(Debug, Clone, Serialize)]
pub struct ConsolidationResult {
    pub facts_stored: usize,
    pub checkpoint: String,
    pub task_id: String,
}

#[derive(Deserialize)]
struct FaitConsolide {
    #[serde(default)]
    node_id: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    confidence: f32,
    #[serde(rename = "type")]
    fact_type: Option<String>,
}

/// Consolidates discoveries into memory and saves the task checkpoint.
pub async fn consolider_fatigue(
    task_id: &str,
    messages: &[serde_json::Value],
    config: &EssaimConfig,
    memoire: &Arc<dyn MemoireCognitive>,
) -> Result<ConsolidationResult> {
    let historique: String = messages
        .iter()
        .filter_map(|m| {
            let role = m["role"].as_str().unwrap_or("unknown");
            let content = m["content"].as_str().unwrap_or("");
            if content.is_empty() { None } else { Some(format!("[{role}]\n{content}")) }
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let sys_extract = "You are a fact extractor. \
        Analyze this agent history and extract ALL discovered facts. \
        Strict JSON format: \
        [{\"node_id\":\"<domain>.<subject>\", \
          \"content\":\"<fact>\", \
          \"confidence\":0.0-1.0, \
          \"type\":\"decouverte|hypothese|echec|insight\"}] \
        Domains: research, experiments, decisions, insights. \
        If nothing durable -> []. No text outside the JSON.";

    let facts_json = appel_llm_auxiliaire(sys_extract, &historique, config).await
        .unwrap_or_else(|_| "[]".to_string());

    let facts: Vec<FaitConsolide> = crate::brain::extraire_json_array(&facts_json)
        .and_then(|js| serde_json::from_str(&js).ok())
        .unwrap_or_default();

    let mut facts_stored = 0usize;
    for fact in &facts {
        if fact.content.trim().is_empty() || fact.node_id.trim().is_empty() { continue; }
        if !crate::brain::node_id_valide(&fact.node_id) { continue; }
        let node_id = format!("research.{}.{}",
            task_id.replace('-', "_"),
            fact.node_id.rsplit('.').next().unwrap_or("fact")
        );
        let confidence_label = (fact.confidence * 100.0).clamp(0.0, 100.0) as u32;
        let fact_type = fact.fact_type.as_deref().unwrap_or("decouverte");
        let content = format!("[confiance:{}%][{}] {}", confidence_label, fact_type, fact.content);
        let _ = memoire
            .write(MemoryItem::new(&node_id, &content)
                .with_source(&format!("fatigue-consolidation:{task_id}")))
            .await;
        facts_stored += 1;
    }

    let checkpoint_prompt = format!(
        "From this history, summarize in strict JSON: \
         {{\"resume\":\"...\",\"next_steps\":[\"...\",\"...\"]}} \
         What remains to be accomplished?\n\n{historique}"
    );
    let checkpoint_json = appel_llm_auxiliaire(&checkpoint_prompt, "", config)
        .await
        .unwrap_or_else(|_| "{}".to_string());

    let _ = memoire
        .write(MemoryItem::new(
            format!("tasks.checkpoints.{}", task_id),
            format!("checkpoint:{}", checkpoint_json),
        ).with_source("fatigue-checkpoint"))
        .await;

    Ok(ConsolidationResult { facts_stored, checkpoint: checkpoint_json, task_id: task_id.to_string() })
}

/// Builds the fresh context after consolidation.
pub async fn contexte_apres_consolidation(
    task_id: &str,
    original_task: &str,
    result: &ConsolidationResult,
    memoire: &Arc<dyn MemoireCognitive>,
) -> Vec<serde_json::Value> {
    let checkpoint = memoire
        .read_node(&format!("tasks.checkpoints.{}", task_id))
        .await
        .ok()
        .and_then(|n| n.get("items").and_then(|i| i.as_array())
            .and_then(|a| a.last().and_then(|it| it.get("content").and_then(|c| c.as_str())))
            .map(|s| s.to_string()))
        .unwrap_or_default();

    let resume = checkpoint.splitn(2, ':').nth(1).unwrap_or("").to_string();

    vec![
        serde_json::json!({
            "role": "system",
            "content": format!(
                "=== RESUMING AFTER COGNITIVE CONSOLIDATION ===\n{} facts consolidated into memory.\n\
                 Checkpoint: {}\n\nUse memory_search to recover what you discovered. \
                 Continue the task from the checkpoint.",
                result.facts_stored, resume
            )
        }),
        serde_json::json!({ "role": "user", "content": original_task }),
    ]
}

async fn appel_llm_auxiliaire(
    system_prompt: &str,
    user_content: &str,
    config: &EssaimConfig,
) -> Result<String> {
    let messages = if user_content.is_empty() {
        vec![serde_json::json!({ "role": "user", "content": system_prompt })]
    } else {
        vec![
            serde_json::json!({ "role": "system", "content": system_prompt }),
            serde_json::json!({ "role": "user", "content": user_content }),
        ]
    };
    let mut stream = provider_chat_stream(
        &config.provider,
        config.aux_model.as_deref().unwrap_or(&config.model),
        &messages, 0.0, 1024,
        &config.api_key, config.api_base.as_deref(), &config.ollama_url,
            None,
        ).await?;
    let mut out = String::new();
    while let Some(chunk) = stream.next().await { out.push_str(&chunk.text); }
    Ok(out)
}
