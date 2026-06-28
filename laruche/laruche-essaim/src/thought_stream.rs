use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ThoughtVisibility {
    Hidden,
    StatusOnly,
    Summaries,
    VerboseTrace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtUpdate {
    pub phase: String,
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ThoughtStreamer {
    visibility: ThoughtVisibility,
    recent: VecDeque<ThoughtUpdate>,
    capacity: usize,
}

impl ThoughtStreamer {
    pub fn new(visibility: ThoughtVisibility) -> Self {
        Self {
            visibility,
            recent: VecDeque::new(),
            capacity: 128,
        }
    }

    pub fn emit(
        &mut self,
        phase: impl Into<String>,
        kind: impl Into<String>,
        text: impl AsRef<str>,
    ) -> Option<ThoughtUpdate> {
        let update = ThoughtUpdate {
            phase: sanitize_fragment(&phase.into()),
            kind: sanitize_fragment(&kind.into()),
            text: sanitize_summary(text.as_ref()),
        };
        if !self.should_emit(&update) {
            return None;
        }
        self.recent.push_back(update.clone());
        while self.recent.len() > self.capacity {
            self.recent.pop_front();
        }
        Some(update)
    }

    fn should_emit(&self, update: &ThoughtUpdate) -> bool {
        match self.visibility {
            ThoughtVisibility::Hidden => false,
            ThoughtVisibility::StatusOnly => update.kind == "status",
            ThoughtVisibility::Summaries => matches!(
                update.kind.as_str(),
                "status" | "observation" | "decision" | "next_action" | "checkpoint"
            ),
            ThoughtVisibility::VerboseTrace => true,
        }
    }
}

impl Default for ThoughtStreamer {
    fn default() -> Self {
        Self::new(ThoughtVisibility::Summaries)
    }
}

pub fn sanitize_summary(input: &str) -> String {
    let mut out = input.replace(['\r', '\n', '\t'], " ");
    for marker in [
        "chain of thought",
        "hidden reasoning",
        "system prompt",
        "private key",
        "secret",
        "raw reflection",
        "api_key",
        "password",
        "token",
        "bearer ",
    ] {
        out = replace_case_insensitive(&out, marker, "[redacted]");
    }
    out.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(220)
        .collect()
}

fn sanitize_fragment(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(40)
        .collect()
}

fn replace_case_insensitive(input: &str, needle: &str, replacement: &str) -> String {
    let lower = input.to_lowercase();
    let needle_lower = needle.to_lowercase();
    let mut output = String::new();
    let mut cursor = 0usize;
    while let Some(pos) = lower[cursor..].find(&needle_lower) {
        let start = cursor + pos;
        output.push_str(&input[cursor..start]);
        output.push_str(replacement);
        cursor = start + needle.len();
    }
    output.push_str(&input[cursor..]);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thought_stream_sanitizes_and_respects_visibility() {
        let mut stream = ThoughtStreamer::new(ThoughtVisibility::StatusOnly);
        assert!(stream
            .emit("hypothesis", "decision", "Check the secret")
            .is_none());

        let update = stream
            .emit("orientation", "status", "Reading the context with secret")
            .unwrap();
        assert_eq!(update.phase, "orientation");
        assert_eq!(update.kind, "status");
        assert!(update.text.contains("[redacted]"));
    }
}
