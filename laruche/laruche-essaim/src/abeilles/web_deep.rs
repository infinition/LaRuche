//! Deep web search: web_search typed results + fetch/cleanup of top pages.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use crate::abeilles::web_recherche::search_web_results;
use anyhow::Result;
use async_trait::async_trait;
use scraper::{Html, Selector};

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

        // Step 1: Search with the robust web_search engine, then keep web_deep cleanup.
        let urls: Vec<(String, String, String)> = match search_web_results(&client, query).await {
            Ok(results) => results
                .into_iter()
                .filter_map(|result| {
                    clean_result_url(&result.url).map(|u| (result.title, u, result.snippet))
                })
                .collect(),
            Err(e) => return Ok(ResultatAbeille::err(format!("Search failed: {}", e))),
        };

        if urls.is_empty() {
            return Ok(ResultatAbeille::ok(format!(
                "No results found for: {}",
                query
            )));
        }

        let mut output = format!("# Deep Search: {}\n\n", query);
        output.push_str(&format!(
            "Found {} results. Fetching top {}...\n\n",
            urls.len(),
            num.min(urls.len())
        ));

        // Step 2: Fetch top N results
        for (i, (title, url, snippet)) in urls.iter().take(num).enumerate() {
            output.push_str(&format!(
                "---\n## {}. {}\n**URL:** {}\n**Snippet:** {}\n\n",
                i + 1,
                title,
                url,
                snippet
            ));

            match client.get(url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let ctype = resp
                            .headers()
                            .get(reqwest::header::CONTENT_TYPE)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("")
                            .to_lowercase();
                        if !ctype.is_empty()
                            && !ctype.contains("html")
                            && !ctype.contains("text/plain")
                        {
                            output.push_str("(non-HTML content ignored)\n\n");
                            continue;
                        }
                        let html = resp.text().await.unwrap_or_default();
                        let text = readability_text(&html);
                        if looks_like_js_shell(&text) {
                            output.push_str(
                                "(JavaScript shell page detected: insufficient readable content)\n",
                            );
                            continue;
                        }

                        let chars: Vec<char> = text.chars().collect();
                        if chars.len() > 2000 {
                            let head: String = chars[..1500].iter().collect();
                            let tail: String = chars[chars.len() - 400..].iter().collect();
                            output.push_str(&head);
                            output.push_str("\n...(middle truncated)...\n");
                            output.push_str(&tail);
                        } else {
                            output.push_str(&text);
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

        if output.len() > 8000 {
            output.truncate(8000);
            output.push_str("\n\n...(total output truncated)");
        }

        Ok(ResultatAbeille::ok(output))
    }
}

/// Clean a DuckDuckGo result URL: decode the `uddg=` redirect, normalize the
/// protocol, reject assets (css/js/img...) and internal DDG links.
fn clean_result_url(raw: &str) -> Option<String> {
    let mut url = raw.trim().to_string();
    if let Some(idx) = url.find("uddg=") {
        let enc = url[idx + 5..].split('&').next().unwrap_or("");
        if let Ok(dec) = urlencoding::decode(enc) {
            url = dec.into_owned();
        }
    }
    if url.starts_with("//") {
        url = format!("https:{url}");
    }
    if !url.starts_with("http") {
        return None;
    }
    let lower = url.to_lowercase();
    if lower.contains("duckduckgo.com") {
        return None;
    }
    const ASSETS: &[&str] = &[
        ".css", ".js", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico", ".woff", ".woff2", ".mp4",
        ".webp", ".pdf", ".zip",
    ];
    let path = lower.split(['?', '#']).next().unwrap_or(&lower);
    if ASSETS.iter().any(|ext| path.ends_with(ext)) {
        return None;
    }
    Some(url)
}

fn readability_text(html: &str) -> String {
    let doc = Html::parse_document(html);
    let mut best = String::new();
    for selector in [
        "article",
        "main",
        "[role=\"main\"]",
        ".entry-content",
        ".post-content",
        ".article-content",
        ".content",
        "#content",
    ] {
        let Ok(sel) = Selector::parse(selector) else {
            continue;
        };
        for node in doc.select(&sel) {
            let text = html_to_text(&node.html());
            if text.chars().count() > best.chars().count() {
                best = text;
            }
        }
    }
    if best.chars().count() >= 500 {
        best
    } else {
        html_to_text(html)
    }
}

fn html_to_text(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut skip_stack: Vec<String> = Vec::new();
    let mut tag_buf = String::new();

    for ch in html.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag_buf.clear();
            }
            '>' => {
                in_tag = false;
                let t = tag_buf.to_lowercase();
                let tag_name = t
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_start_matches('/')
                    .to_string();
                if t.starts_with('/') {
                    if skip_stack.last().is_some_and(|open| open == &tag_name) {
                        skip_stack.pop();
                    }
                } else if matches!(
                    tag_name.as_str(),
                    "script"
                        | "style"
                        | "nav"
                        | "footer"
                        | "header"
                        | "aside"
                        | "form"
                        | "svg"
                        | "noscript"
                ) {
                    skip_stack.push(tag_name);
                } else if skip_stack.is_empty()
                    && matches!(
                        t.split_whitespace()
                            .next()
                            .unwrap_or("")
                            .trim_start_matches('/'),
                        "p" | "div" | "br" | "h1" | "h2" | "h3" | "h4" | "li" | "tr"
                    )
                {
                    result.push('\n');
                }
            }
            _ if in_tag => {
                tag_buf.push(ch);
            }
            _ if skip_stack.is_empty() => {
                result.push(ch);
            }
            _ => {}
        }
    }

    result
        .replace("&amp;", "&")
        .replace("&nbsp;", " ")
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn looks_like_js_shell(text: &str) -> bool {
    let lower = text.to_lowercase();
    let short = text.chars().count() < 500;
    short
        && (lower.contains("enable javascript")
            || lower.contains("requires javascript")
            || lower.contains("please enable js")
            || lower.contains("app shell"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readability_prefers_article_and_ignores_nav_footer() {
        let html = format!(
            "<html><body><nav>menu repeated repeated</nav><article><h1>Titre</h1><p>{}</p></article><footer>legal legal legal</footer></body></html>",
            "contenu utile ".repeat(80)
        );
        let text = readability_text(&html);
        assert!(text.contains("contenu utile"));
        assert!(!text.contains("menu repeated"));
        assert!(!text.contains("legal legal"));
    }

    #[test]
    fn detects_js_shells() {
        assert!(looks_like_js_shell(
            "Please enable JavaScript to use this app"
        ));
        assert!(!looks_like_js_shell(&"article ".repeat(200)));
    }
}
