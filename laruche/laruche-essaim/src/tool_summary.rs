use anyhow::Result;
use serde::{Deserialize, Serialize};

pub const DEFAULT_TOOL_SUMMARY_THRESHOLD: usize = 8_000;

pub trait ToolSummaryClient {
    fn summarize(&self, prompt: &str) -> Result<String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSummary {
    pub summarized: bool,
    pub text: String,
}

pub fn resumer_output<C: ToolSummaryClient>(
    aux_client: &C,
    output: &str,
    threshold: usize,
) -> Result<ToolSummary> {
    if output.chars().count() <= threshold {
        return Ok(ToolSummary {
            summarized: false,
            text: output.to_string(),
        });
    }

    let prompt = construire_prompt_resume(output);
    let summary = aux_client.summarize(&prompt)?.trim().to_string();
    Ok(ToolSummary {
        summarized: true,
        text: if summary.is_empty() {
            resume_extractif(output)
        } else {
            summary
        },
    })
}

pub fn construire_prompt_resume(output: &str) -> String {
    let preview = head_tail(output, 2_000);
    format!(
        "Summarize this tool result, preserving facts, paths, errors and useful next actions.\n\nResult:\n{preview}\n\nSummary:"
    )
}

pub fn resume_extractif(output: &str) -> String {
    format!("[Extractive summary]\n{}", head_tail(output, 1_200))
}

fn head_tail(output: &str, max_chars: usize) -> String {
    let chars: Vec<char> = output.chars().collect();
    if chars.len() <= max_chars {
        return output.to_string();
    }
    let head_len = max_chars / 2;
    let tail_len = max_chars - head_len;
    let head: String = chars.iter().take(head_len).collect();
    let tail: String = chars.iter().skip(chars.len() - tail_len).collect();
    format!(
        "{head}\n...\n[{} chars omitted]\n...\n{tail}",
        chars.len() - max_chars
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct MockClient {
        calls: Cell<usize>,
        response: String,
    }

    impl ToolSummaryClient for MockClient {
        fn summarize(&self, _prompt: &str) -> Result<String> {
            self.calls.set(self.calls.get() + 1);
            Ok(self.response.clone())
        }
    }

    #[test]
    fn petit_output_ne_declenche_pas_client() {
        let client = MockClient {
            calls: Cell::new(0),
            response: "unused".into(),
        };

        let summary = resumer_output(&client, "petit", 10).unwrap();

        assert!(!summary.summarized);
        assert_eq!(summary.text, "petit");
        assert_eq!(client.calls.get(), 0);
    }

    #[test]
    fn gros_output_appelle_client_auxiliaire() {
        let client = MockClient {
            calls: Cell::new(0),
            response: "Lu gros journal et trouve erreur X".into(),
        };

        let summary = resumer_output(&client, &"x".repeat(100), 10).unwrap();

        assert!(summary.summarized);
        assert_eq!(summary.text, "Lu gros journal et trouve erreur X");
        assert_eq!(client.calls.get(), 1);
    }

    #[test]
    fn resume_vide_retombe_sur_extractif() {
        let client = MockClient {
            calls: Cell::new(0),
            response: "   ".into(),
        };

        let summary = resumer_output(&client, &"abcdef".repeat(100), 10).unwrap();

        assert!(summary.summarized);
        assert!(summary.text.contains("Extractive summary"));
    }
}
