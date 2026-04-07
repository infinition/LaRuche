use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;

/// Search the web using Brave Search (no API key needed).
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
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .build()?;

        // Try Brave Search first, fallback to DuckDuckGo Lite
        let results = search_brave(&client, query).await
            .or_else(|_| Ok::<Vec<SearchResult>, anyhow::Error>(vec![]))
            .unwrap_or_default();

        let results = if results.is_empty() {
            search_ddg_lite(&client, query).await.unwrap_or_default()
        } else {
            results
        };

        if results.is_empty() {
            return Ok(ResultatAbeille::ok(format!(
                "No results found for: {}\nTry a different query or use web_fetch with a specific URL.",
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

/// Search using Brave Search HTML scraping.
async fn search_brave(client: &reqwest::Client, query: &str) -> Result<Vec<SearchResult>> {
    let url = format!(
        "https://search.brave.com/search?q={}&source=web",
        urlencoding::encode(query)
    );

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        client.get(&url).send(),
    ).await??;

    let html = response.text().await?;
    let mut results = Vec::new();

    // Brave uses data-pos attributes and specific class patterns
    // Parse <a> tags within result snippets
    let mut pos = 0;
    while results.len() < 10 {
        // Find snippet blocks
        let snippet_marker = "snippet-description";
        let Some(snippet_start) = html[pos..].find(snippet_marker) else { break; };
        let abs_snippet = pos + snippet_start;

        // Look backwards for the title/URL
        let search_back = if abs_snippet > 500 { abs_snippet - 500 } else { 0 };
        let block = &html[search_back..abs_snippet + 500.min(html.len() - abs_snippet)];

        // Extract title from <a class="heading-..."> or <span class="title">
        let title = extract_between(block, "heading-serpresult", "</a>")
            .or_else(|| extract_between(block, "title", "</span>"))
            .map(|s| strip_html_tags(&s))
            .unwrap_or_default();

        // Extract URL from href
        let url_str = extract_href(block).unwrap_or_default();

        // Extract snippet
        let snippet = extract_after_tag(&html[abs_snippet..], ">", "<")
            .map(|s| strip_html_tags(&s))
            .unwrap_or_default();

        if !title.is_empty() && !url_str.is_empty() && url_str.starts_with("http") {
            results.push(SearchResult {
                title: title.trim().to_string(),
                url: url_str,
                snippet: snippet.trim().to_string(),
            });
        }

        pos = abs_snippet + 100;
    }

    Ok(results)
}

/// Fallback: DuckDuckGo Lite (simpler HTML, more reliable than full DDG)
async fn search_ddg_lite(client: &reqwest::Client, query: &str) -> Result<Vec<SearchResult>> {
    let url = format!(
        "https://lite.duckduckgo.com/lite/?q={}",
        urlencoding::encode(query)
    );

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        client.get(&url).send(),
    ).await??;

    let html = response.text().await?;
    let mut results = Vec::new();

    // DDG Lite has a simple table structure
    // Links are in <a rel="nofollow" href="...">title</a>
    // Snippets are in <td class="result-snippet">
    let mut pos = 0;
    while results.len() < 10 {
        let Some(link_start) = html[pos..].find("rel=\"nofollow\"") else { break; };
        let abs_link = pos + link_start;

        // Get href
        let href_start = html[..abs_link].rfind("href=\"").map(|i| i + 6);
        let url_str = if let Some(hs) = href_start {
            if let Some(he) = html[hs..].find('"') {
                html[hs..hs + he].to_string()
            } else { String::new() }
        } else { String::new() };

        // Get title (text between > and </a>)
        let title = if let Some(tag_end) = html[abs_link..].find('>') {
            let text_start = abs_link + tag_end + 1;
            if let Some(close) = html[text_start..].find("</a>") {
                strip_html_tags(&html[text_start..text_start + close])
            } else { String::new() }
        } else { String::new() };

        // Get snippet (next result-snippet td)
        let snippet = if let Some(snip_start) = html[abs_link..].find("result-snippet") {
            let snip_abs = abs_link + snip_start;
            if let Some(td_end) = html[snip_abs..].find('>') {
                let text_start = snip_abs + td_end + 1;
                if let Some(close) = html[text_start..].find("</td>") {
                    strip_html_tags(&html[text_start..text_start + close])
                } else { String::new() }
            } else { String::new() }
        } else { String::new() };

        // Extract real URL from DDG redirect (//duckduckgo.com/l/?uddg=ENCODED_URL)
        let final_url = if url_str.contains("uddg=") {
            let uddg_start = url_str.find("uddg=").unwrap() + 5;
            let encoded = url_str[uddg_start..].split('&').next().unwrap_or("");
            urlencoding::decode(encoded).unwrap_or_else(|_| encoded.into()).into_owned()
        } else if url_str.starts_with("//") {
            format!("https:{}", url_str)
        } else {
            url_str.clone()
        };

        if !title.trim().is_empty() && (final_url.starts_with("http") || url_str.contains("uddg=")) {
            results.push(SearchResult {
                title: title.trim().to_string(),
                url: final_url,
                snippet: snippet.trim().to_string(),
            });
        }

        pos = abs_link + 50;
    }

    Ok(results)
}

fn extract_between(html: &str, class_marker: &str, end_tag: &str) -> Option<String> {
    let start = html.find(class_marker)?;
    let after = &html[start..];
    let tag_end = after.find('>')?;
    let text_start = start + tag_end + 1;
    let close = html[text_start..].find(end_tag)?;
    Some(html[text_start..text_start + close].to_string())
}

fn extract_after_tag(html: &str, open: &str, close: &str) -> Option<String> {
    let start = html.find(open)? + open.len();
    let end = html[start..].find(close)?;
    Some(html[start..start + end].to_string())
}

fn extract_href(html: &str) -> Option<String> {
    let start = html.find("href=\"")? + 6;
    let end = html[start..].find('"')?;
    Some(html[start..start + end].to_string())
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
    result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
}
