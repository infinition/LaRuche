//! Public image search, without dorks or guessed URLs.
//!
//! Wikimedia Commons exposes a stable, key-free API that returns both the
//! original URL and a resized thumbnail. The result carries the media marker
//! understood by the chat so it can be displayed in the gallery.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

const COMMONS_API: &str = "https://commons.wikimedia.org/w/api.php";
const MAX_RESULTS: u64 = 8;

pub struct ImageSearch;

#[async_trait]
impl Abeille for ImageSearch {
    fn nom(&self) -> &str {
        "image_search"
    }

    fn description(&self) -> &str {
        "Search Wikimedia Commons for public images and display them in chat. Use this tool whenever you need a real image; never guess or fabricate image URLs."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Image search terms" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 8, "description": "Number of results to return (default 4)" }
            },
            "required": ["query"],
            "additionalProperties": false
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(&self, args: Value, _ctx: &ContextExecution) -> Result<ResultatAbeille> {
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if query.chars().count() < 2 {
            return Ok(ResultatAbeille::err(
                "Image query must be at least 2 characters.",
            ));
        }
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(4)
            .clamp(1, MAX_RESULTS);
        let client = reqwest::Client::builder()
            .user_agent("LaRuche/0.2 image_search (Wikimedia Commons)")
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        let response = client
            .get(COMMONS_API)
            .query(&[
                ("action", "query"),
                ("generator", "search"),
                ("gsrnamespace", "6"),
                ("gsrsearch", query),
                ("gsrlimit", &limit.to_string()),
                ("prop", "imageinfo"),
                ("iiprop", "url"),
                ("iiurlwidth", "1200"),
                ("format", "json"),
                ("origin", "*"),
            ])
            .send()
            .await?;
        if !response.status().is_success() {
            return Ok(ResultatAbeille::err(format!(
                "Image search unavailable (Wikimedia Commons HTTP {}).",
                response.status()
            )));
        }
        let body: Value = response.json().await?;
        let mut items = Vec::new();
        if let Some(pages) = body.pointer("/query/pages").and_then(Value::as_object) {
            for page in pages.values() {
                let Some(info) = page.pointer("/imageinfo/0") else {
                    continue;
                };
                let url = info
                    .get("thumburl")
                    .or_else(|| info.get("url"))
                    .and_then(Value::as_str)
                    .filter(|url| url.starts_with("https://"));
                let Some(url) = url else {
                    continue;
                };
                let title = page
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("Wikimedia Image")
                    .trim_start_matches("File:")
                    .trim_start_matches("Fichier:");
                let source = info
                    .get("descriptionurl")
                    .and_then(Value::as_str)
                    .unwrap_or(COMMONS_API);
                items.push(json!({
                    "url": url,
                    "kind": "image",
                    "title": title,
                    "caption": format!("Source: Wikimedia Commons - {source}")
                }));
                if items.len() >= limit as usize {
                    break;
                }
            }
        }
        if items.is_empty() {
            return Ok(ResultatAbeille::ok(format!(
                "No public images found on Wikimedia Commons for: {query}."
            )));
        }
        Ok(ResultatAbeille::ok(format!(
            "<laruche-media>{}</laruche-media>\n{} public image(s) found for: {query}. Reply briefly; do not use dorks or invent URLs.",
            serde_json::to_string(&items)?,
            items.len()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abeille::Abeille;

    #[test]
    fn schema_limits_results() {
        let schema = ImageSearch.schema();
        assert_eq!(schema["properties"]["limit"]["maximum"], MAX_RESULTS);
        assert_eq!(ImageSearch.niveau_danger(), NiveauDanger::Safe);
    }
}
