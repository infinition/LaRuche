//! web_search: robust web search (LaRuche spec).
//!
//! Typed output (title/url/snippet), domain filters (allowed XOR blocked), timing.
//! Interchangeable engines: Tavily (if `LARUCHE_TAVILY_KEY`), otherwise the DuckDuckGo HTML scraper
//! (`html.duckduckgo.com/html/`, no key), with a DDG lite fallback. Output format plus a CRITICAL
//! REMINDER to force Markdown-link citations.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;

pub struct WebSearch;

#[derive(Debug, Clone)]
pub(crate) struct SearchResult {
    pub(crate) title: String,
    pub(crate) url: String,
    pub(crate) snippet: String,
}

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

fn domain_of(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("")
        .trim_start_matches("www.")
        .to_lowercase()
}

fn str_vec(v: &serde_json::Value) -> Option<Vec<String>> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty())
}

#[async_trait]
impl Abeille for WebSearch {
    fn nom(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web. Returns a list of title/url/snippet results. Optional filters: `allowed_domains` \
         OR `blocked_domains` (never both). Always cite sources as Markdown links in your response."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query (min 2 characters)" },
                "num_results": { "type": "integer", "description": "Max results returned (default 8, max 15)" },
                "allowed_domains": { "type": "array", "items": { "type": "string" }, "description": "Keep only these domains" },
                "blocked_domains": { "type": "array", "items": { "type": "string" }, "description": "Exclude these domains" }
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
        let query = args["query"].as_str().unwrap_or("").trim().to_string();
        if query.len() < 2 {
            return Ok(ResultatAbeille::err("query too short (min 2 characters)"));
        }
        let allowed = str_vec(&args["allowed_domains"]);
        let blocked = str_vec(&args["blocked_domains"]);
        if allowed.is_some() && blocked.is_some() {
            return Ok(ResultatAbeille::err(
                "Cannot specify both allowed_domains and blocked_domains.",
            ));
        }

        let start = std::time::Instant::now();
        let client = reqwest::Client::builder().user_agent(UA).build()?;

        let mut results = search_web_results(&client, &query)
            .await
            .unwrap_or_default();

        // Domain filters.
        results.retain(|r| {
            let d = domain_of(&r.url);
            if let Some(a) = &allowed {
                if !a.iter().any(|x| d.contains(&x.to_lowercase())) {
                    return false;
                }
            }
            if let Some(b) = &blocked {
                if b.iter().any(|x| d.contains(&x.to_lowercase())) {
                    return false;
                }
            }
            true
        });
        let n = args["num_results"].as_u64().unwrap_or(8).clamp(1, 15) as usize;
        results.truncate(n);

        if results.is_empty() {
            return Ok(ResultatAbeille::ok(format!(
                "No results for: \"{query}\". If the information is unavailable, tell the user honestly."
            )));
        }

        let mut out = format!("Web search results for: \"{query}\"\n\n");
        for (i, r) in results.iter().enumerate() {
            out.push_str(&format!(
                "[{}] Title: {}\n    URL: {}\n    Snippet: {}\n\n",
                i + 1,
                r.title,
                r.url,
                r.snippet
            ));
        }
        out.push_str(&format!(
            "IMPORTANT: cite these sources in your final response as Markdown links ([Title](URL)). \
             Never fabricate links or reuse links not in this list.\n(search completed in {:.2}s)",
            start.elapsed().as_secs_f64()
        ));
        Ok(ResultatAbeille::ok(out))
    }
}

fn decode_ddg(href: &str) -> String {
    if let Some(i) = href.find("uddg=") {
        let enc = href[i + 5..].split('&').next().unwrap_or("");
        return urlencoding::decode(enc)
            .map(|c| c.into_owned())
            .unwrap_or_else(|_| enc.to_string());
    }
    if href.starts_with("//") {
        format!("https:{href}")
    } else {
        href.to_string()
    }
}

/// Engine shared by `web_search` and `web_deep_search`:
/// Tavily if available, otherwise the rich DDG HTML, then a DDG lite fallback.
pub(crate) async fn search_web_results(
    client: &reqwest::Client,
    query: &str,
) -> Result<Vec<SearchResult>> {
    // 1) Dedicated API if configured (quality > volume, and it spares the scrapers).
    //    Tavily and Brave are built for agents: clean results, operators handled.
    if let Ok(key) = std::env::var("LARUCHE_TAVILY_KEY") {
        let r = search_tavily(client, query, &key).await.unwrap_or_default();
        if !r.is_empty() {
            return Ok(r);
        }
    }
    if let Ok(key) = std::env::var("LARUCHE_BRAVE_KEY") {
        let r = search_brave(client, query, &key).await.unwrap_or_default();
        if !r.is_empty() {
            return Ok(r);
        }
    }
    if let Ok(url) = std::env::var("LARUCHE_SEARXNG_URL") {
        let r = search_searxng(client, query, &url).await.unwrap_or_default();
        if !r.is_empty() {
            return Ok(r);
        }
    }

    // 2) Otherwise: query the free scrapers IN PARALLEL and merge
    //    (instead of "first non-empty wins"). Better coverage, and
    //    resilience when an engine is rate-limited/blocked (the cause of "No results").
    let (yahoo, ddg, lite) = tokio::join!(
        search_yahoo_html(client, query),
        search_ddg_html(client, query),
        search_ddg_lite(client, query),
    );
    let mut fusion: Vec<SearchResult> = Vec::new();
    fusionner(&mut fusion, yahoo.unwrap_or_default());
    fusionner(&mut fusion, ddg.unwrap_or_default());
    fusionner(&mut fusion, lite.unwrap_or_default());
    Ok(fusion)
}

/// URL deduplication key: domain + path, without `www.`, query, fragment, or trailing slash.
fn cle_url(url: &str) -> String {
    let sans_proto = url.split("://").nth(1).unwrap_or(url);
    let sans_q = sans_proto.split(['?', '#']).next().unwrap_or(sans_proto);
    sans_q
        .trim_start_matches("www.")
        .trim_end_matches('/')
        .to_lowercase()
}

/// Adds to `acc` the results not already present (deduplicated by [`cle_url`]).
fn fusionner(acc: &mut Vec<SearchResult>, nouveaux: Vec<SearchResult>) {
    for r in nouveaux {
        let cle = cle_url(&r.url);
        if cle.is_empty() || !r.url.starts_with("http") {
            continue;
        }
        if !acc.iter().any(|x| cle_url(&x.url) == cle) {
            acc.push(r);
        }
    }
}

/// Brave Search API (key `LARUCHE_BRAVE_KEY`): reliable, agent-oriented, generous free tier.
async fn search_brave(
    client: &reqwest::Client,
    query: &str,
    key: &str,
) -> Result<Vec<SearchResult>> {
    let url = format!(
        "https://api.search.brave.com/res/v1/web/search?q={}&count=10",
        urlencoding::encode(query)
    );
    let resp = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        client
            .get(&url)
            .header("X-Subscription-Token", key)
            .header("Accept", "application/json")
            .send(),
    )
    .await??;
    let v: serde_json::Value = resp.json().await?;
    let out = v["web"]["results"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|r| SearchResult {
                    title: r["title"].as_str().unwrap_or("").to_string(),
                    url: r["url"].as_str().unwrap_or("").to_string(),
                    snippet: r["description"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(out)
}

fn decode_yahoo_url(href: &str) -> String {
    if let Some(i) = href.find("/RU=") {
        let rest = &href[i + 4..];
        let enc = rest.split('/').next().unwrap_or(rest);
        return urlencoding::decode(enc)
            .map(|c| c.into_owned())
            .unwrap_or_else(|_| enc.to_string());
    }
    href.to_string()
}

async fn search_searxng(
    client: &reqwest::Client,
    query: &str,
    base_url: &str,
) -> Result<Vec<SearchResult>> {
    let url = format!(
        "{}/search?q={}&format=json",
        base_url.trim_end_matches('/'),
        urlencoding::encode(query)
    );
    let resp =
        tokio::time::timeout(std::time::Duration::from_secs(10), client.get(&url).send()).await??;
    let v: serde_json::Value = resp.json().await?;
    let out = v["results"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|r| SearchResult {
                    title: r["title"].as_str().unwrap_or("").to_string(),
                    url: r["url"].as_str().unwrap_or("").to_string(),
                    snippet: r["content"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(out)
}

async fn search_yahoo_html(client: &reqwest::Client, query: &str) -> Result<Vec<SearchResult>> {
    let url = format!(
        "https://search.yahoo.com/search?p={}",
        urlencoding::encode(query)
    );
    let resp =
        tokio::time::timeout(std::time::Duration::from_secs(10), client.get(&url).send()).await??;
    let html = resp.text().await?;

    let doc = scraper::Html::parse_document(&html);
    let result_sel = scraper::Selector::parse("div.algo, div.algo-sr").unwrap();
    let title_sel = scraper::Selector::parse("h3.title").unwrap();
    let a_sel = scraper::Selector::parse("a").unwrap();
    let snip_sel = scraper::Selector::parse("div.compText").unwrap();

    let mut out = Vec::new();
    for el in doc.select(&result_sel) {
        let a = el.select(&a_sel).next();
        let title = el
            .select(&title_sel)
            .next()
            .map(|t| t.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        let href = a.and_then(|a| a.value().attr("href")).unwrap_or("");

        let snippet = el
            .select(&snip_sel)
            .next()
            .map(|s| s.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let final_url = decode_yahoo_url(href);

        if !title.is_empty()
            && final_url.starts_with("http")
            && !final_url.contains("search.yahoo.com")
        {
            out.push(SearchResult {
                title,
                url: final_url,
                snippet,
            });
        }
        if out.len() >= 10 {
            break;
        }
    }
    Ok(out)
}

/// DuckDuckGo HTML scraper (no key): rich endpoint `html.duckduckgo.com/html/`.
async fn search_ddg_html(client: &reqwest::Client, query: &str) -> Result<Vec<SearchResult>> {
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );
    let resp =
        tokio::time::timeout(std::time::Duration::from_secs(10), client.get(&url).send()).await??;
    let html = resp.text().await?;

    let doc = scraper::Html::parse_document(&html);
    let result_sel = scraper::Selector::parse(".result").unwrap();
    let a_sel = scraper::Selector::parse(".result__a").unwrap();
    let snip_sel = scraper::Selector::parse(".result__snippet").unwrap();

    let mut out = Vec::new();
    for el in doc.select(&result_sel) {
        let a = el.select(&a_sel).next();
        let title = a
            .map(|a| a.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        let href = a.and_then(|a| a.value().attr("href")).unwrap_or("");
        let url = decode_ddg(href);
        let snippet = el
            .select(&snip_sel)
            .next()
            .map(|s| s.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        if !title.is_empty() && url.starts_with("http") {
            out.push(SearchResult {
                title,
                url,
                snippet,
            });
        }
        if out.len() >= 10 {
            break;
        }
    }
    Ok(out)
}

/// Tavily (optional): dense, LLM-optimized snippets.
async fn search_tavily(
    client: &reqwest::Client,
    query: &str,
    key: &str,
) -> Result<Vec<SearchResult>> {
    let resp = client
        .post("https://api.tavily.com/search")
        .json(&serde_json::json!({ "api_key": key, "query": query, "max_results": 8 }))
        .send()
        .await?;
    let v: serde_json::Value = resp.json().await?;
    let out = v["results"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|r| SearchResult {
                    title: r["title"].as_str().unwrap_or("").to_string(),
                    url: r["url"].as_str().unwrap_or("").to_string(),
                    snippet: r["content"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(out)
}

/// Fallback: DuckDuckGo lite (simple parsing, no scraper).
async fn search_ddg_lite(client: &reqwest::Client, query: &str) -> Result<Vec<SearchResult>> {
    let url = format!(
        "https://lite.duckduckgo.com/lite/?q={}",
        urlencoding::encode(query)
    );
    let resp =
        tokio::time::timeout(std::time::Duration::from_secs(10), client.get(&url).send()).await??;
    let html = resp.text().await?;
    let mut results = Vec::new();
    let mut pos = 0;
    while results.len() < 10 {
        let Some(link_start) = html[pos..].find("rel=\"nofollow\"") else {
            break;
        };
        let abs_link = pos + link_start;
        let href_start = html[..abs_link].rfind("href=\"").map(|i| i + 6);
        let url_str = href_start
            .and_then(|hs| html[hs..].find('"').map(|he| html[hs..hs + he].to_string()))
            .unwrap_or_default();
        let title = html[abs_link..]
            .find('>')
            .map(|te| abs_link + te + 1)
            .and_then(|ts| {
                html[ts..]
                    .find("</a>")
                    .map(|c| strip_html_tags(&html[ts..ts + c]))
            })
            .unwrap_or_default();
        let snippet = html[abs_link..]
            .find("result-snippet")
            .map(|s| abs_link + s)
            .and_then(|sa| html[sa..].find('>').map(|te| sa + te + 1))
            .and_then(|ts| {
                html[ts..]
                    .find("</td>")
                    .map(|c| strip_html_tags(&html[ts..ts + c]))
            })
            .unwrap_or_default();
        let final_url = decode_ddg(&url_str);
        if !title.trim().is_empty() && (final_url.starts_with("http") || url_str.contains("uddg="))
        {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_extraction() {
        assert_eq!(
            domain_of("https://www.meteofrance.com/x"),
            "meteofrance.com"
        );
        assert_eq!(domain_of("http://example.org"), "example.org");
    }

    #[tokio::test]
    async fn rejects_both_filters() {
        let t = WebSearch;
        let r = t
            .executer(
                serde_json::json!({"query":"rust","allowed_domains":["a.com"],"blocked_domains":["b.com"]}),
                &ContextExecution::default(),
            )
            .await
            .unwrap();
        assert!(!r.success);
    }

    #[tokio::test]
    async fn rejects_short_query() {
        let t = WebSearch;
        let r = t
            .executer(
                serde_json::json!({"query":"x"}),
                &ContextExecution::default(),
            )
            .await
            .unwrap();
        assert!(!r.success);
    }
}
