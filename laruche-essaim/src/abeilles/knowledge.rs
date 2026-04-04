//! Knowledge base abeilles — add/search/list knowledge entries (RAG).

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use crate::rag::KnowledgeBase;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Add information to the knowledge base.
pub struct KnowledgeAdd {
    pub kb: Arc<RwLock<KnowledgeBase>>,
}

#[async_trait]
impl Abeille for KnowledgeAdd {
    fn nom(&self) -> &str { "knowledge_add" }
    fn description(&self) -> &str {
        "Add information to the persistent knowledge base. The information will be stored \
         with an embedding and can be retrieved later via semantic search. \
         Use this to remember important facts, user preferences, or any information \
         that should persist across conversations."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "The information to remember" },
                "source": { "type": "string", "description": "Optional source (e.g., 'user said', 'web search', 'file: x.txt')" }
            },
            "required": ["text"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger { NiveauDanger::Safe }

    async fn executer(&self, args: serde_json::Value, _ctx: &ContextExecution) -> Result<ResultatAbeille> {
        let text = args["text"].as_str().ok_or_else(|| anyhow::anyhow!("Missing 'text'"))?;
        let source = args["source"].as_str();

        let mut kb = self.kb.write().await;
        match kb.add(text, source).await {
            Ok(id) => Ok(ResultatAbeille::ok(format!(
                "Stored in knowledge base (id: {}, total: {} entries)",
                id, kb.len()
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!(
                "Failed to store: {}. Make sure an embedding model is available in Ollama (e.g., nomic-embed-text).",
                e
            ))),
        }
    }
}

/// Search the knowledge base for relevant information.
pub struct KnowledgeSearch {
    pub kb: Arc<RwLock<KnowledgeBase>>,
}

#[async_trait]
impl Abeille for KnowledgeSearch {
    fn nom(&self) -> &str { "knowledge_search" }
    fn description(&self) -> &str {
        "Search the knowledge base for relevant information using semantic search. \
         Returns the most relevant stored entries. Use this to recall previously stored \
         information, user preferences, or facts from earlier conversations."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "The search query" },
                "top_k": { "type": "integer", "description": "Number of results (default: 5)" }
            },
            "required": ["query"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger { NiveauDanger::Safe }

    async fn executer(&self, args: serde_json::Value, _ctx: &ContextExecution) -> Result<ResultatAbeille> {
        let query = args["query"].as_str().ok_or_else(|| anyhow::anyhow!("Missing 'query'"))?;
        let top_k = args["top_k"].as_u64().unwrap_or(5) as usize;

        let kb = self.kb.read().await;
        if kb.len() == 0 {
            return Ok(ResultatAbeille::ok("Knowledge base is empty. Use knowledge_add to store information first."));
        }

        match kb.search(query, top_k).await {
            Ok(results) => {
                if results.is_empty() {
                    return Ok(ResultatAbeille::ok("No relevant results found."));
                }
                let mut output = format!("Found {} result(s):\n\n", results.len());
                for (i, (score, entry)) in results.iter().enumerate() {
                    let source = entry.source.as_deref().unwrap_or("unknown");
                    output.push_str(&format!(
                        "{}. [score: {:.2}] (source: {})\n{}\n\n",
                        i + 1, score, source,
                        &entry.text[..entry.text.len().min(500)]
                    ));
                }
                Ok(ResultatAbeille::ok(output))
            }
            Err(e) => Ok(ResultatAbeille::err(format!("Search failed: {}", e))),
        }
    }
}
