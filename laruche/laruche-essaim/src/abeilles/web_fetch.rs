use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

/// Fetch a web page and return its text content.
pub struct WebFetch;

#[async_trait]
impl Abeille for WebFetch {
    fn nom(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch the content of a web page at the given URL and return it as clean text. \
         Use this to read articles, documentation, or any web page content. Set `render` \
         to true for pages that need JavaScript rendering."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch"
                },
                "render": {
                    "type": "boolean",
                    "description": "Si true, tente un rendu headless Chrome/Edge avant extraction (pages JS)."
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

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Ok(ResultatAbeille::err(
                "URL must start with http:// or https://",
            ));
        }

        if args["render"].as_bool().unwrap_or(false) {
            return match render_url_dom(url, 3).await {
                Ok(html) => Ok(ResultatAbeille::ok(cap_head_tail(
                    html_to_text(&html),
                    6000,
                ))),
                Err(e) => Ok(ResultatAbeille::err(format!(
                    "Render requested but browser rendering failed: {e}"
                ))),
            };
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
        let text = if content_type.contains("html") {
            html_to_text(&body)
        } else {
            body
        };

        Ok(ResultatAbeille::ok(cap_head_tail(text, 6000)))
    }
}

async fn render_url_dom(url: &str, wait_seconds: u64) -> Result<String> {
    let chrome_paths = if cfg!(windows) {
        vec![
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        ]
    } else {
        vec![
            "google-chrome",
            "chromium-browser",
            "chromium",
            "microsoft-edge",
        ]
    };

    let chrome = chrome_paths
        .iter()
        .find(|p| std::path::Path::new(p).exists() || which::which(p).is_ok())
        .ok_or_else(|| {
            anyhow::anyhow!("Chrome/Edge not found; use browser_navigate if available")
        })?;

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(wait_seconds + 15),
        Command::new(chrome)
            .args([
                "--headless=new",
                "--disable-gpu",
                "--no-sandbox",
                "--disable-dev-shm-usage",
                &format!("--virtual-time-budget={}", wait_seconds * 1000),
                "--dump-dom",
                url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await??;

    let html = String::from_utf8_lossy(&output.stdout).to_string();
    if html.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "browser returned empty DOM: {}",
            &stderr[..stderr.len().min(300)]
        );
    }
    Ok(html)
}

fn cap_head_tail(text: String, max_len: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_len {
        return text;
    }
    let tail_len = 1200.min(max_len / 2).min(chars.len());
    let head_len = max_len.saturating_sub(tail_len);
    let head: String = chars[..head_len].iter().collect();
    let tail: String = chars[chars.len() - tail_len..].iter().collect();
    format!(
        "{head}\n\n...(milieu tronque, {} caracteres au total)...\n\n{tail}",
        chars.len()
    )
}

/// Simple HTML to text converter: strips tags, scripts, styles.
fn html_to_text(html: &str) -> String {
    // Régions « boilerplate » à ignorer (nav/menus/pied de page/pubs) — c'est ce
    // qui polluait l'extraction et noyait le vrai contenu. On ne touche PAS à
    // <header> (peut contenir le titre d'article) ni <main>/<article>.
    const BOILERPLATE: &[&str] = &["nav", "footer", "aside", "noscript", "form"];
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut boiler_depth = 0i32;
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
                // Suivi de la profondeur des régions boilerplate (gère l'imbrication).
                let bare = tag_lower
                    .trim_start_matches('/')
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                if BOILERPLATE.contains(&bare) {
                    if tag_lower.starts_with('/') {
                        boiler_depth = (boiler_depth - 1).max(0);
                    } else if !tag_lower.ends_with('/') {
                        boiler_depth += 1;
                    }
                }
                if matches!(
                    tag_lower
                        .trim_start_matches('/')
                        .split_whitespace()
                        .next()
                        .unwrap_or(""),
                    "p" | "div"
                        | "br"
                        | "h1"
                        | "h2"
                        | "h3"
                        | "h4"
                        | "h5"
                        | "h6"
                        | "li"
                        | "tr"
                        | "blockquote"
                        | "hr"
                        | "section"
                        | "article"
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
            _ if !in_script && !in_style && boiler_depth == 0 => {
                result.push(ch);
            }
            _ => {}
        }
    }

    let result = result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
        .replace("&#39;", "'");

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abeille::Abeille;

    #[test]
    fn schema_exposes_render_parameter() {
        let schema = WebFetch.schema();
        assert!(schema["properties"].get("render").is_some());
    }

    #[test]
    fn html_to_text_strips_boilerplate() {
        let html = "<html><body><nav>Accueil Menu Connexion</nav>\
            <article><h1>Titre</h1><p>Vrai contenu de l'article.</p></article>\
            <footer>Mentions legales Cookies</footer></body></html>";
        let txt = html_to_text(html);
        assert!(txt.contains("Vrai contenu de l'article"));
        assert!(txt.contains("Titre"));
        assert!(!txt.contains("Menu"));
        assert!(!txt.contains("Mentions legales"));
    }

    #[test]
    fn cap_head_tail_keeps_tail() {
        let text = format!("{}END", "a".repeat(7000));
        let capped = cap_head_tail(text, 6000);
        assert!(capped.contains("milieu tronque"));
        assert!(capped.ends_with("END"));
    }
}
