use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;

/// Fetch a web page and return its text content.
pub struct WebFetch;

#[async_trait]
impl Abeille for WebFetch {
    fn nom(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch the content of a web page at the given URL and return it as clean text. \
         Use this to read articles, documentation, or any web page content."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch"
                }
            },
            "required": ["url"]
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
        let url = args["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' argument"))?;

        // Validate URL
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Ok(ResultatAbeille::err(
                "URL must start with http:// or https://",
            ));
        }

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        let response = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => return Ok(ResultatAbeille::err(format!("Failed to fetch: {}", e))),
        };

        if !response.status().is_success() {
            return Ok(ResultatAbeille::err(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = response.text().await.unwrap_or_default();

        // Convert HTML to plain text
        let text = if content_type.contains("html") {
            html_to_text(&body)
        } else {
            body
        };

        // Truncate to avoid context explosion
        let max_len = 6000;
        let truncated: String = text.chars().take(max_len).collect();
        let output = if text.len() > max_len {
            format!("{}\n\n... (content truncated, {} chars total)", truncated, text.len())
        } else {
            truncated
        };

        Ok(ResultatAbeille::ok(output))
    }
}

/// Simple HTML to text converter — strips tags, scripts, styles.
fn html_to_text(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut tag_name = String::new();
    let mut collecting_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => {
                in_tag = true;
                collecting_tag = true;
                tag_name.clear();
            }
            '>' => {
                in_tag = false;
                collecting_tag = false;
                let tag_lower = tag_name.to_lowercase();
                if tag_lower.starts_with("script") {
                    in_script = true;
                } else if tag_lower.starts_with("/script") {
                    in_script = false;
                } else if tag_lower.starts_with("style") {
                    in_style = true;
                } else if tag_lower.starts_with("/style") {
                    in_style = false;
                }
                // Add line breaks for block elements
                if matches!(
                    tag_lower.trim_start_matches('/').split_whitespace().next().unwrap_or(""),
                    "p" | "div" | "br" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
                        | "li" | "tr" | "blockquote" | "hr" | "section" | "article"
                ) {
                    result.push('\n');
                }
            }
            _ if in_tag => {
                if collecting_tag && (ch.is_alphanumeric() || ch == '/') {
                    tag_name.push(ch);
                } else {
                    collecting_tag = false;
                }
            }
            _ if !in_script && !in_style => {
                result.push(ch);
            }
            _ => {}
        }
    }

    // Decode HTML entities
    let result = result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
        .replace("&#39;", "'");

    // Collapse whitespace: multiple blank lines → single, trim lines
    let mut cleaned = String::new();
    let mut blank_count = 0;
    for line in result.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                cleaned.push('\n');
            }
        } else {
            blank_count = 0;
            cleaned.push_str(trimmed);
            cleaned.push('\n');
        }
    }

    cleaned.trim().to_string()
}
