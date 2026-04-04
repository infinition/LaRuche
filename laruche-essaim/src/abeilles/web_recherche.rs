use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;

/// Search the web using DuckDuckGo HTML (no API key needed).
pub struct WebSearch;

#[async_trait]
impl Abeille for WebSearch {
    fn nom(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information. Returns a list of search results with titles, \
         URLs, and snippets. Use this when you need current information or facts you don't know."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                }
            },
            "required": ["query"]
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
        let query = args["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' argument"))?;

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()?;

        // Use DuckDuckGo HTML-only search (no JS required, no API key)
        let url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        let response = match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            client.get(&url).send(),
        )
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => return Ok(ResultatAbeille::err(format!("HTTP error: {}", e))),
            Err(_) => return Ok(ResultatAbeille::err("Search timed out after 10 seconds")),
        };

        let html = response.text().await.unwrap_or_default();

        // Parse results from DuckDuckGo HTML
        let results = parse_ddg_results(&html);

        if results.is_empty() {
            return Ok(ResultatAbeille::ok(format!(
                "No results found for: {}",
                query
            )));
        }

        let mut output = format!("Search results for: {}\n\n", query);
        for (i, result) in results.iter().enumerate().take(8) {
            output.push_str(&format!(
                "{}. {}\n   {}\n   {}\n\n",
                i + 1,
                result.title,
                result.url,
                result.snippet
            ));
        }

        Ok(ResultatAbeille::ok(output))
    }
}

struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

/// Parse search results from DuckDuckGo HTML response.
/// This is a simple parser — no external HTML parsing dependency needed.
fn parse_ddg_results(html: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();

    // DuckDuckGo HTML results are in <a class="result__a"> tags
    // and snippets in <a class="result__snippet"> tags
    let mut pos = 0;
    while let Some(result_start) = html[pos..].find("class=\"result__a\"") {
        let abs_start = pos + result_start;

        // Extract title and URL from the <a> tag
        let title;
        let url;

        // Find href
        if let Some(href_start) = html[..abs_start].rfind("href=\"") {
            let href_begin = href_start + 6;
            if let Some(href_end) = html[href_begin..].find('"') {
                let raw_url = &html[href_begin..href_begin + href_end];
                // DuckDuckGo wraps URLs — extract the actual URL
                url = extract_ddg_url(raw_url);
            } else {
                pos = abs_start + 20;
                continue;
            }
        } else {
            pos = abs_start + 20;
            continue;
        }

        // Find title text (between > and </a>)
        if let Some(tag_end) = html[abs_start..].find('>') {
            let text_start = abs_start + tag_end + 1;
            if let Some(close) = html[text_start..].find("</a>") {
                title = strip_html_tags(&html[text_start..text_start + close]);
            } else {
                pos = abs_start + 20;
                continue;
            }
        } else {
            pos = abs_start + 20;
            continue;
        }

        // Find snippet
        let snippet = if let Some(snip_start) = html[abs_start..].find("class=\"result__snippet\"") {
            let snip_abs = abs_start + snip_start;
            if let Some(tag_end) = html[snip_abs..].find('>') {
                let text_start = snip_abs + tag_end + 1;
                if let Some(close) = html[text_start..].find("</a>") {
                    strip_html_tags(&html[text_start..text_start + close])
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if !title.trim().is_empty() && !url.is_empty() {
            results.push(SearchResult {
                title: title.trim().to_string(),
                url,
                snippet: snippet.trim().to_string(),
            });
        }

        pos = abs_start + 20;

        if results.len() >= 10 {
            break;
        }
    }

    results
}

fn extract_ddg_url(raw: &str) -> String {
    // DuckDuckGo HTML wraps URLs like: //duckduckgo.com/l/?uddg=https%3A%2F%2F...
    if let Some(uddg_start) = raw.find("uddg=") {
        let encoded = &raw[uddg_start + 5..];
        let encoded = encoded.split('&').next().unwrap_or(encoded);
        urlencoding::decode(encoded)
            .unwrap_or_else(|_| encoded.into())
            .into_owned()
    } else if raw.starts_with("http") {
        raw.to_string()
    } else {
        raw.to_string()
    }
}

fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    // Decode common HTML entities
    result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
}
