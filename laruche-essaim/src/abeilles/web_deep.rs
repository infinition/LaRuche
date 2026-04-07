//! Deep web search — searches the web then auto-fetches top results for detailed content.
//! Combines web_search + web_fetch in one tool for comprehensive research.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;

/// Deep web search: search + auto-fetch top results for full content.
pub struct WebDeepSearch;

#[async_trait]
impl Abeille for WebDeepSearch {
    fn nom(&self) -> &str {
        "web_deep_search"
    }

    fn description(&self) -> &str {
        "Perform a deep web search: first searches the web, then automatically fetches \
         and extracts content from the top 3 results. Returns both search snippets AND \
         full page content. Use this for thorough research when you need detailed information, \
         not just snippets."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of results to fetch in detail (default: 3, max: 5)"
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
        let num = args["num_results"].as_u64().unwrap_or(3).min(5) as usize;

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        // Step 1: Search
        let search_url = format!(
            "https://lite.duckduckgo.com/lite/?q={}",
            urlencoding::encode(query)
        );

        let search_resp = match client.get(&search_url).send().await {
            Ok(r) => r.text().await.unwrap_or_default(),
            Err(e) => return Ok(ResultatAbeille::err(format!("Search failed: {}", e))),
        };

        // Parse results
        let urls = parse_search_urls(&search_resp);

        if urls.is_empty() {
            return Ok(ResultatAbeille::ok(format!(
                "No results found for: {}",
                query
            )));
        }

        let mut output = format!("# Deep Search: {}\n\n", query);
        output.push_str(&format!("Found {} results. Fetching top {}...\n\n", urls.len(), num.min(urls.len())));

        // Step 2: Fetch top N results
        for (i, (title, url)) in urls.iter().take(num).enumerate() {
            output.push_str(&format!("---\n## {}. {}\n**URL:** {}\n\n", i + 1, title, url));

            match client.get(url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let html = resp.text().await.unwrap_or_default();
                        let text = html_to_text(&html);
                        // Truncate each page content
                        let truncated: String = text.chars().take(2000).collect();
                        output.push_str(&truncated);
                        if text.len() > 2000 {
                            output.push_str("\n...(truncated)");
                        }
                    } else {
                        output.push_str(&format!("(HTTP {})", resp.status()));
                    }
                }
                Err(e) => {
                    output.push_str(&format!("(Failed to fetch: {})", e));
                }
            }
            output.push_str("\n\n");
        }

        // Truncate total output
        if output.len() > 8000 {
            output.truncate(8000);
            output.push_str("\n\n...(total output truncated)");
        }

        Ok(ResultatAbeille::ok(output))
    }
}

fn parse_search_urls(html: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let mut pos = 0;

    while results.len() < 10 {
        let Some(link_start) = html[pos..].find("rel=\"nofollow\"") else { break; };
        let abs_link = pos + link_start;

        // Get href
        let href_start = html[..abs_link].rfind("href=\"").map(|i| i + 6);
        let url_str = if let Some(hs) = href_start {
            if let Some(he) = html[hs..].find('"') { html[hs..hs + he].to_string() } else { String::new() }
        } else { String::new() };

        // Get title
        let title = if let Some(tag_end) = html[abs_link..].find('>') {
            let text_start = abs_link + tag_end + 1;
            if let Some(close) = html[text_start..].find("</a>") {
                strip_tags(&html[text_start..text_start + close])
            } else { String::new() }
        } else { String::new() };

        // Extract real URL from DDG redirect
        let final_url = if url_str.contains("uddg=") {
            let uddg_start = url_str.find("uddg=").unwrap() + 5;
            let encoded = url_str[uddg_start..].split('&').next().unwrap_or("");
            urlencoding::decode(encoded).unwrap_or_else(|_| encoded.into()).into_owned()
        } else if url_str.starts_with("//") {
            format!("https:{}", url_str)
        } else {
            url_str.clone()
        };

        if !title.trim().is_empty() && final_url.starts_with("http") {
            results.push((title.trim().to_string(), final_url));
        }

        pos = abs_link + 50;
    }

    results
}

fn strip_tags(html: &str) -> String {
    let mut r = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch { '<' => in_tag = true, '>' => in_tag = false, _ if !in_tag => r.push(ch), _ => {} }
    }
    r.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&nbsp;", " ")
}

fn html_to_text(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut tag_buf = String::new();

    for ch in html.chars() {
        match ch {
            '<' => { in_tag = true; tag_buf.clear(); }
            '>' => {
                in_tag = false;
                let t = tag_buf.to_lowercase();
                if t.starts_with("script") { in_script = true; }
                else if t.starts_with("/script") { in_script = false; }
                else if t.starts_with("style") { in_style = true; }
                else if t.starts_with("/style") { in_style = false; }
                else if matches!(t.split_whitespace().next().unwrap_or("").trim_start_matches('/'),
                    "p"|"div"|"br"|"h1"|"h2"|"h3"|"h4"|"li"|"tr") { result.push('\n'); }
            }
            _ if in_tag => { tag_buf.push(ch); }
            _ if !in_script && !in_style => { result.push(ch); }
            _ => {}
        }
    }

    result.replace("&amp;", "&").replace("&nbsp;", " ")
        .lines().map(|l| l.trim()).filter(|l| !l.is_empty())
        .collect::<Vec<_>>().join("\n")
}
