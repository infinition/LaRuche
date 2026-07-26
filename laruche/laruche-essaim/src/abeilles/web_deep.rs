//! Deep web search: web_search typed results + PARALLEL fetch/cleanup of top pages.
//!
//! War-machine spec: the top pages are fetched CONCURRENTLY (wall-clock = slowest
//! page, not the sum), each failed/JS-shell page falls back to r.jina.ai before
//! being skipped, and every truncation is char-safe (the old byte-based
//! `String::truncate` PANICKED on multi-byte characters - French accents).

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use crate::abeilles::web_fetch::{extraire_lisible, fetch_via_jina, looks_like_js_shell};
use crate::abeilles::web_recherche::search_web_results;
use anyhow::Result;
use async_trait::async_trait;

/// Deep web search: search + auto-fetch top results for full content.
pub struct WebDeepSearch;

/// Per-page and total char budgets (head+tail keeps intros AND conclusions).
const PAGE_TETE: usize = 2_600;
const PAGE_QUEUE: usize = 600;
const TOTAL_MAX: usize = 16_000;

#[async_trait]
impl Abeille for WebDeepSearch {
    fn nom(&self) -> &str {
        "web_deep_search"
    }

    fn description(&self) -> &str {
        "Perform a deep web search: searches the web, then fetches the top results IN \
         PARALLEL and extracts their content. Returns search snippets AND full page \
         content. Use this for thorough research when you need details, not just snippets."
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
                    "description": "Number of results to fetch in detail (default: 3, max: 6)"
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
        let num = args["num_results"].as_u64().unwrap_or(3).clamp(1, 6) as usize;

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        // Step 1: search with the robust web_search engine.
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
            // Actionable coaching instead of a dead end: diagnose the keyword soup
            // and hand back a tightened query (the prompt rule alone was ignored).
            return Ok(ResultatAbeille::ok(
                crate::abeilles::web_recherche::conseil_recherche_vide(query),
            ));
        }

        let mut output = format!("# Deep Search: {}\n\n", query);
        output.push_str(&format!(
            "Found {} results. Fetching top {} in parallel...\n\n",
            urls.len(),
            num.min(urls.len())
        ));

        // Step 2: fetch the top N CONCURRENTLY (each with its own fallback chain).
        let futs = urls.iter().take(num).enumerate().map(|(i, (title, url, snippet))| {
            let client = client.clone();
            async move {
                let corps = recolter_page(&client, url).await;
                let mut section = format!(
                    "---\n## {}. {}\n**URL:** {}\n**Snippet:** {}\n\n",
                    i + 1,
                    title,
                    url,
                    snippet
                );
                section.push_str(&corps);
                section.push_str("\n\n");
                section
            }
        });
        for section in futures_util::future::join_all(futs).await {
            output.push_str(&section);
        }

        // Char-safe total cap (byte-based truncate panicked mid-UTF-8).
        if output.chars().count() > TOTAL_MAX {
            output = output.chars().take(TOTAL_MAX).collect();
            output.push_str("\n\n...(total output truncated - re-run with a narrower query or fetch a specific URL with web_fetch)");
        }

        Ok(ResultatAbeille::ok(output))
    }
}

/// Fetches one page's readable content: direct → readability; on failure, HTTP error
/// or JS shell → r.jina.ai fallback; head+tail char-safe capping.
async fn recolter_page(client: &reqwest::Client, url: &str) -> String {
    let direct = match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => {
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
                Err("(non-HTML content)".to_string())
            } else {
                let html = resp.text().await.unwrap_or_default();
                let text = extraire_lisible(&html);
                if text.trim().is_empty() || looks_like_js_shell(&text) {
                    Err("(JS shell / empty)".to_string())
                } else {
                    Ok(text)
                }
            }
        }
        Ok(resp) => Err(format!("(HTTP {})", resp.status())),
        Err(e) => Err(format!("(fetch failed: {e})")),
    };
    match direct {
        Ok(text) => tete_queue(&text),
        Err(motif) => {
            // Second chance: jina renders JS and bypasses simple blocks.
            if let Some(text) = fetch_via_jina(client, url).await {
                format!("[via r.jina.ai - direct: {motif}]\n{}", tete_queue(&text))
            } else {
                motif
            }
        }
    }
}

/// Head + tail char-safe extract of a page body.
fn tete_queue(text: &str) -> String {
    let n = text.chars().count();
    if n <= PAGE_TETE + PAGE_QUEUE {
        return text.to_string();
    }
    let head: String = text.chars().take(PAGE_TETE).collect();
    let tail: String = text.chars().skip(n - PAGE_QUEUE).collect();
    format!("{head}\n...(middle truncated, {n} chars total - use web_fetch with offset to read it all)...\n{tail}")
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
        ".webp", ".zip",
    ];
    let path = lower.split(['?', '#']).next().unwrap_or(&lower);
    if ASSETS.iter().any(|ext| path.ends_with(ext)) {
        return None;
    }
    Some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tete_queue_est_char_safe() {
        // The old byte-based truncate panicked here (multi-byte at the boundary).
        let text = "é".repeat(PAGE_TETE + PAGE_QUEUE + 100);
        let capped = tete_queue(&text);
        assert!(capped.contains("middle truncated"));
        // short text untouched
        assert_eq!(tete_queue("court"), "court");
    }

    #[test]
    fn clean_url_decode_et_filtre() {
        assert_eq!(
            clean_result_url("//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.org%2Fpage"),
            Some("https://example.org/page".to_string())
        );
        assert_eq!(clean_result_url("https://site.com/style.css"), None);
        assert_eq!(clean_result_url("javascript:void(0)"), None);
        // PDFs are no longer rejected: web_fetch converts them via jina.
        assert!(clean_result_url("https://site.com/manual.pdf").is_some());
    }
}
