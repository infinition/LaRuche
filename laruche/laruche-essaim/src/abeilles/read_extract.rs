use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

const MAX_OUTPUT_CHARS: usize = 12_000;
const HEAD_CHARS: usize = 8_000;
const TAIL_CHARS: usize = 3_000;

pub struct ReadExtract;

#[async_trait]
impl Abeille for ReadExtract {
    fn nom(&self) -> &str {
        "read_extract"
    }

    fn description(&self) -> &str {
        "Extract text from a PDF, .txt, or .md file. Returns head+tail output when content exceeds the limit."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the PDF, TXT, or Markdown file to read" }
            },
            "required": ["path"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;
        let path = Path::new(path_str);
        if !path.exists() {
            return Ok(ResultatAbeille::err(format!("File not found: {path_str}")));
        }
        if !path.is_file() {
            return Ok(ResultatAbeille::err(format!("Not a file: {path_str}")));
        }

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let text = match ext.as_str() {
            "pdf" => match pdf_extract::extract_text(path) {
                Ok(text) => text,
                Err(e) => {
                    return Ok(ResultatAbeille::err(format!(
                        "PDF extraction failed: {e}"
                    )))
                }
            },
            "txt" | "md" | "markdown" => match std::fs::read_to_string(path) {
                Ok(text) => text,
                Err(e) => return Ok(ResultatAbeille::err(format!("File read failed: {e}"))),
            },
            _ => {
                return Ok(ResultatAbeille::err(format!(
                    "Unsupported format for read_extract: .{ext} (expected: pdf, txt, md)"
                )))
            }
        };

        let text = normalize_text(&text);
        if text.trim().is_empty() {
            return Ok(ResultatAbeille::ok(format!(
                "No text extracted from {path_str}."
            )));
        }
        Ok(ResultatAbeille::ok(cap_head_tail(&text)))
    }
}

fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

fn cap_head_tail(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= MAX_OUTPUT_CHARS {
        return text.to_string();
    }
    let head: String = chars[..HEAD_CHARS.min(chars.len())].iter().collect();
    let tail_start = chars.len().saturating_sub(TAIL_CHARS);
    let tail: String = chars[tail_start..].iter().collect();
    format!(
        "{head}\n\n...(middle truncated: {} chars omitted)...\n\n{tail}",
        chars.len().saturating_sub(HEAD_CHARS + TAIL_CHARS)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_head_tail_keeps_beginning_and_end() {
        let text = format!("{}MIDDLE{}", "A".repeat(9_000), "Z".repeat(4_000));
        let capped = cap_head_tail(&text);
        assert!(capped.starts_with('A'));
        assert!(capped.contains("middle truncated"));
        assert!(capped.ends_with('Z'));
        assert!(!capped.contains("MIDDLE"));
    }
}
