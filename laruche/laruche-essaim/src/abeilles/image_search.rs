//! Recherche d'images publiques, sans dorks ni URL devinee.
//!
//! Wikimedia Commons expose une API stable, sans cle, qui renvoie a la fois
//! l'URL originale et une miniature redimensionnee. Le resultat transporte le
//! marqueur media compris par le chat afin d'etre affiche dans la galerie.

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
        "Recherche des images publiques sur Wikimedia Commons et les affiche directement dans le chat. Utilise cet outil pour une image sur le web; n'utilise pas de Google dorks et n'invente jamais d'URL d'image."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Description de l'image recherchee" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 8, "description": "Nombre de resultats a afficher (defaut 4)" }
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
                "La recherche d'image doit contenir au moins 2 caracteres.",
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
                "Recherche d'image indisponible (Wikimedia Commons HTTP {}).",
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
                    .unwrap_or("Image Wikimedia")
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
                    "caption": format!("Source : Wikimedia Commons — {source}")
                }));
                if items.len() >= limit as usize {
                    break;
                }
            }
        }
        if items.is_empty() {
            return Ok(ResultatAbeille::ok(format!(
                "Aucune image publique trouvee sur Wikimedia Commons pour : {query}."
            )));
        }
        Ok(ResultatAbeille::ok(format!(
            "<laruche-media>{}</laruche-media>\n{} image(s) publique(s) trouvee(s) pour : {query}. Reponds brievement et n'utilise pas de dork.",
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
