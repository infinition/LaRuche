//! Suggestions unifiees pour LaRuche.
//!
//! Cette crate est volontairement minimale pour la reconciliation workspace. Les
//! moteurs concrets (fichiers, memoire, outils, agents, commandes) viendront se
//! brancher ici sans imposer de dependance runtime externe.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestionKind {
    File,
    MemoryNode,
    Tool,
    Agent,
    Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suggestion {
    pub kind: SuggestionKind,
    pub label: String,
    pub value: String,
    pub detail: Option<String>,
}

impl Suggestion {
    pub fn new(kind: SuggestionKind, label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            kind,
            label: label.into(),
            value: value.into(),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

pub fn filter_prefix(items: &[Suggestion], prefix: &str, limit: usize) -> Vec<Suggestion> {
    let prefix = prefix.trim().to_lowercase();
    let limit = limit.max(1);
    items
        .iter()
        .filter(|item| {
            prefix.is_empty()
                || item.label.to_lowercase().starts_with(&prefix)
                || item.value.to_lowercase().starts_with(&prefix)
        })
        .take(limit)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_prefix_keeps_matching_suggestions() {
        let items = vec![
            Suggestion::new(SuggestionKind::Tool, "file_read", "file_read"),
            Suggestion::new(SuggestionKind::Tool, "web_search", "web_search"),
        ];
        let filtered = filter_prefix(&items, "file", 10);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].label, "file_read");
    }
}
