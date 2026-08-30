//! Parsing of LLM output: tool calls (`<tool_call>` tags, attribute form, raw JSON
//! fallback) and `<plan>` blocks.

use crate::evenements::PlanItem;
use serde::{Deserialize, Serialize};

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

    while let Some(start) = text[search_from..].find("<tool_call") {
        let after_tag = search_from + start + "<tool_call".len();
        let rest = &text[after_tag..];
        if let Some(body) = rest.strip_prefix('>') {
            // Canonical form: <tool_call>{"name":...,"arguments":{...}}</tool_call>
            if let Some(end) = body.find("</tool_call>") {
                let json_str = body[..end].trim();
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
                search_from = after_tag + 1 + end + "</tool_call>".len();
                continue;
            }
            break;
        }
        // Attribute form emitted by some local models (observed with gemma):
        //   <tool_call name="memory_search" arguments={"query": "missions", "limit": 10}>
        if rest.starts_with(|c: char| c.is_whitespace()) {
            if let Some((call, consumed)) = parse_tool_call_attributs(rest) {
                calls.push(call);
                search_from = after_tag + consumed;
                continue;
            }
        }
        // Unrecognized shape after the opener: move past it and keep scanning.
        search_from = after_tag;
    }

    calls
}

/// Parse the attribute form that follows `<tool_call` (whitespace included):
/// `name="X" arguments={...}>` with `args=` accepted, quotes optional, and an
/// optional stray `</tool_call>` right after. Returns the call and how many
/// bytes of the input were consumed.
fn parse_tool_call_attributs(rest: &str) -> Option<(ToolCall, usize)> {
    let name_pos = rest.find("name=")?;
    let after_name = &rest[name_pos + "name=".len()..];
    let first = after_name.chars().next()?;
    let (name, name_attr_len) = if first == '"' || first == '\'' {
        let end = after_name[1..].find(first)?;
        (after_name[1..1 + end].to_string(), end + 2)
    } else {
        let n: String = after_name
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '>' && *c != '/')
            .collect();
        let l = n.len();
        (n, l)
    };
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }

    let v_start = rest
        .find("arguments=")
        .map(|i| i + "arguments=".len())
        .or_else(|| rest.find("args=").map(|i| i + "args=".len()));
    let (args, mut end) = match v_start {
        Some(v) => {
            let (js, je) = plage_objet_json(&rest[v..])?;
            let obj = &rest[v + js..v + je];
            (serde_json::from_str::<serde_json::Value>(obj).ok()?, v + je)
        }
        None => (
            serde_json::json!({}),
            name_pos + "name=".len() + name_attr_len,
        ),
    };
    // Consume through the tag's closing '>' and an optional stray closing tag.
    if let Some(gt) = rest[end..].find('>') {
        end += gt + 1;
    }
    let apres = rest[end..].trim_start();
    if let Some(sans) = apres.strip_prefix("</tool_call>") {
        end = rest.len() - sans.len();
    }
    Some((
        ToolCall {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            args,
        },
        end,
    ))
}

/// Locate the first brace-balanced JSON object in `s` (string-aware, so a `}`
/// or `>` inside a string value does not end the scan). Returns its byte range.
fn plage_objet_json(s: &str) -> Option<(usize, usize)> {
    let start = s.find('{')?;
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if esc {
                esc = false;
            } else if b == b'\\' {
                esc = true;
            } else if b == b'"' {
                in_str = false;
            }
        } else {
            match b {
                b'"' => in_str = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((start, i + 1));
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Defensive fallback: try to parse raw JSON when the model did not use
/// the `<tool_call>` tags. deepseek-v4-flash and gemma4:e4b sometimes emit
/// `{"name":"...","arguments":{...}}` directly without tags.
fn try_parse_as_tool_call(json: &str) -> Option<ToolCall> {
    serde_json::from_str::<ToolCallRaw>(json)
        .ok()
        .map(|r| ToolCall {
            id: uuid::Uuid::new_v4().to_string(),
            name: r.name,
            args: r.arguments,
        })
}

pub(crate) fn parse_tool_calls_json_brut(text: &str) -> Vec<ToolCall> {
    let trimmed = text.trim();

    // Format 1: ```json\n{...}\n``` block
    if trimmed.starts_with("```") {
        let without_fence = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        if let Some(call) = try_parse_as_tool_call(without_fence) {
            return vec![call];
        }
    }

    // Format 2: raw {"name":"...","arguments":{...}}
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        if let Some(call) = try_parse_as_tool_call(trimmed) {
            return vec![call];
        }
    }

    // Format 3 : JSON array [{...}, {...}]
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        if let Ok(calls) = serde_json::from_str::<Vec<ToolCallRaw>>(trimmed) {
            return calls
                .into_iter()
                .map(|r| ToolCall {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: r.name,
                    args: r.arguments,
                })
                .collect();
        }
    }

    // Format 4: any JSON within the text (best-effort extraction)
    let mut calls = Vec::new();
    let mut search_from = 0;
    while let Some(start) = text[search_from..].find('{') {
        let abs_start = search_from + start;
        // Find the matching closing `}` (basic counting)
        let mut depth = 0u32;
        let mut end = abs_start;
        for (i, ch) in text[abs_start..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        end = abs_start + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        if depth != 0 {
            break; // malformed JSON
        }
        let candidate = &text[abs_start..end];
        if let Some(call) = try_parse_as_tool_call(candidate) {
            // Avoid duplicates
            if !calls.iter().any(|c: &ToolCall| c.name == call.name) {
                calls.push(call);
            }
        }
        search_from = end;
    }

    calls
}

#[derive(Debug, Deserialize)]
struct ToolCallRaw {
    #[serde(alias = "tool", alias = "function", alias = "function_name")]
    name: String,
    #[serde(
        default,
        alias = "arguments",
        alias = "args",
        alias = "parameters",
        alias = "input"
    )]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tool_call_style_attributs_gemma() {
        // Exact shape observed in chat: the model emits an XML-attribute call
        // instead of the canonical JSON body, which used to leak as raw text.
        let calls = parse_tool_calls(
            r#"<tool_call name="memory_search" arguments={"query": "missions", "limit": 10}>"#,
        );
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "memory_search");
        assert_eq!(calls[0].args["query"], "missions");
        assert_eq!(calls[0].args["limit"], 10);
    }

    #[test]
    fn parse_tool_call_attributs_sans_arguments_puis_canonique() {
        let calls = parse_tool_calls(concat!(
            "avant <tool_call name='cron_list'> milieu ",
            r#"<tool_call>{"name":"web_fetch","arguments":{"url":"https://a.b"}}</tool_call>"#,
        ));
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "cron_list");
        assert_eq!(calls[0].args, serde_json::json!({}));
        assert_eq!(calls[1].name, "web_fetch");
        assert_eq!(calls[1].args["url"], "https://a.b");
    }

    #[test]
    fn parse_tool_call_attributs_accolades_dans_les_chaines() {
        // A '>' or '}' inside a string value must not truncate the JSON scan.
        let calls = parse_tool_calls(
            r#"<tool_call name="file_write" args={"path": "a.md", "content": "x > y et {z}"}></tool_call>"#,
        );
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "file_write");
        assert_eq!(calls[0].args["content"], "x > y et {z}");
    }
}
