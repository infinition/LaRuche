//! Presentation de medias dans le chat.
//!
//! L'outil ne telecharge rien : il decrit de maniere sure les medias que le
//! navigateur doit rendre. Les chemins locaux restent limites au dossier de
//! travail actif; le dashboard les sert ensuite via son endpoint local.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;

const MAX_MEDIA_ITEMS: usize = 8;

pub struct MediaPresent;

#[async_trait]
impl Abeille for MediaPresent {
    fn nom(&self) -> &str {
        "media_present"
    }

    fn description(&self) -> &str {
        "Display one or more media items (image, PDF, video, audio) directly below the response. Accepts http(s) URLs and local paths inside the working directory. Call after finding or creating a media file the user needs to see."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": MAX_MEDIA_ITEMS,
                    "description": "Media items to display in the conversation.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "url": { "type": "string", "description": "http(s) URL or absolute/relative local path" },
                            "kind": { "type": "string", "enum": ["image", "pdf", "video", "audio", "auto"], "description": "Media type; auto-detected from file extension when omitted" },
                            "title": { "type": "string", "description": "Short caption shown in the chat" },
                            "caption": { "type": "string", "description": "Optional context shown below the media" }
                        },
                        "required": ["url"]
                    }
                }
            },
            "required": ["items"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(&self, args: Value, ctx: &ContextExecution) -> Result<ResultatAbeille> {
        let Some(items) = args.get("items").and_then(Value::as_array) else {
            return Ok(ResultatAbeille::err(
                "'items' must be a list of media objects.",
            ));
        };
        if items.is_empty() || items.len() > MAX_MEDIA_ITEMS {
            return Ok(ResultatAbeille::err(format!(
                "Provide between 1 and {MAX_MEDIA_ITEMS} media items."
            )));
        }

        let mut normalized = Vec::with_capacity(items.len());
        for item in items {
            let Some(raw_url) = item.get("url").and_then(Value::as_str).map(str::trim) else {
                return Ok(ResultatAbeille::err("Each media item must include a URL."));
            };
            if raw_url.is_empty() {
                return Ok(ResultatAbeille::err(
                    "Media URL cannot be empty.",
                ));
            }
            let (url, local) = normalize_media_url(raw_url, ctx)?;
            let kind = normalize_kind(item.get("kind").and_then(Value::as_str), &url);
            let title = item
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| default_title(&url));
            let caption = item
                .get("caption")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty());
            normalized.push(json!({
                "url": url,
                "kind": kind,
                "title": title,
                "caption": caption,
                "local": local,
            }));
        }

        // Marqueur compact : l'UI l'intercepte dans l'evenement outil et ne
        // l'affiche jamais comme du texte au modele ou a l'utilisateur.
        Ok(ResultatAbeille::ok(format!(
            "<laruche-media>{}</laruche-media>\n{} media item(s) ready to display.",
            serde_json::to_string(&normalized)?,
            normalized.len()
        )))
    }
}

fn normalize_media_url(raw: &str, ctx: &ContextExecution) -> Result<(String, bool)> {
    if raw.starts_with("https://") || raw.starts_with("http://") {
        return Ok((raw.to_string(), false));
    }
    if raw.starts_with("file://") {
        anyhow::bail!("Use a local path, not a file:// URL.");
    }
    let candidate = Path::new(raw);
    let path = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        ctx.working_dir.join(candidate)
    };
    let canonical = std::fs::canonicalize(&path)
        .map_err(|_| anyhow::anyhow!("Local media not found: {}", path.display()))?;
    if !canonical.is_file() {
        anyhow::bail!(
            "Local media must be a file: {}",
            canonical.display()
        );
    }
    let root = std::fs::canonicalize(&ctx.working_dir).unwrap_or_else(|_| ctx.working_dir.clone());
    if !canonical.starts_with(&root) {
        anyhow::bail!(
            "Local media must be inside the working directory: {}",
            root.display()
        );
    }
    Ok((canonical.to_string_lossy().to_string(), true))
}

fn normalize_kind(requested: Option<&str>, url: &str) -> &'static str {
    match requested.unwrap_or("auto").to_ascii_lowercase().as_str() {
        "image" => "image",
        "pdf" => "pdf",
        "video" => "video",
        "audio" => "audio",
        _ => match url
            .split('?')
            .next()
            .unwrap_or(url)
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "avif" => "image",
            "pdf" => "pdf",
            "mp4" | "webm" | "mov" | "m4v" | "ogv" => "video",
            "mp3" | "wav" | "ogg" | "m4a" | "aac" | "flac" => "audio",
            _ => "image",
        },
    }
}

fn default_title(url: &str) -> &str {
    url.rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("Media")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_media_extensions() {
        assert_eq!(normalize_kind(None, "https://x.test/a.mp4"), "video");
        assert_eq!(normalize_kind(None, "photo.webp"), "image");
        assert_eq!(normalize_kind(None, "report.pdf"), "pdf");
    }

    #[test]
    fn explicit_kind_wins() {
        assert_eq!(normalize_kind(Some("video"), "image.jpg"), "video");
    }
}
