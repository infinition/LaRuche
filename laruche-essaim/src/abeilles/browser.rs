//! Browser control abeille — navigates web pages, takes screenshots, extracts content.
//!
//! Uses headless Chrome via shell commands (no CDP dependency needed).
//! For full CDP integration, a separate browser service would be needed.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

/// Navigate to a URL and get a screenshot + page content using headless Chrome.
pub struct BrowserNavigate;

#[async_trait]
impl Abeille for BrowserNavigate {
    fn nom(&self) -> &str {
        "browser_navigate"
    }

    fn description(&self) -> &str {
        "Navigate to a URL using a headless browser and return the page content as text. \
         This is more powerful than web_fetch as it executes JavaScript and renders the page \
         like a real browser. Use for pages that require JS rendering."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to navigate to"
                },
                "wait_seconds": {
                    "type": "integer",
                    "description": "Seconds to wait for page load (default: 3)"
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
        let wait = args["wait_seconds"].as_u64().unwrap_or(3);

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Ok(ResultatAbeille::err("URL must start with http:// or https://"));
        }

        // Try to find Chrome/Edge executable
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
            vec!["google-chrome", "chromium-browser", "chromium", "microsoft-edge"]
        };

        let chrome = chrome_paths.iter().find(|p| {
            std::path::Path::new(p).exists() || which::which(p).is_ok()
        });

        let Some(chrome_path) = chrome else {
            // Fallback: use web_fetch style approach
            return Ok(ResultatAbeille::err(
                "Chrome/Edge not found. Install Chrome or use web_fetch instead."
            ));
        };

        // Use Chrome headless to dump DOM
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(wait + 15),
            Command::new(chrome_path)
                .args([
                    "--headless=new",
                    "--disable-gpu",
                    "--no-sandbox",
                    "--disable-dev-shm-usage",
                    &format!("--virtual-time-budget={}", wait * 1000),
                    "--dump-dom",
                    url,
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let html = String::from_utf8_lossy(&output.stdout);
                if html.is_empty() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Ok(ResultatAbeille::err(format!(
                        "Chrome returned empty output. stderr: {}",
                        &stderr[..stderr.len().min(500)]
                    )));
                }

                // Simple HTML to text conversion
                let text = html_to_text_simple(&html);
                let max_len = 6000;
                let truncated: String = text.chars().take(max_len).collect();
                let output = if text.len() > max_len {
                    format!("{}\n\n...(truncated, {} chars total)", truncated, text.len())
                } else {
                    truncated
                };

                Ok(ResultatAbeille::ok(output))
            }
            Ok(Err(e)) => Ok(ResultatAbeille::err(format!("Chrome error: {}", e))),
            Err(_) => Ok(ResultatAbeille::err("Browser timed out")),
        }
    }
}

/// Take a screenshot of a URL using headless Chrome.
pub struct BrowserScreenshot;

#[async_trait]
impl Abeille for BrowserScreenshot {
    fn nom(&self) -> &str {
        "browser_screenshot"
    }

    fn description(&self) -> &str {
        "Take a screenshot of a web page. Returns the path to the saved screenshot file. \
         Useful for visual inspection or sharing with users."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "The URL to screenshot" },
                "output_path": { "type": "string", "description": "Path to save the screenshot (default: screenshot.png)" }
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
        let url = args["url"].as_str().ok_or_else(|| anyhow::anyhow!("Missing 'url'"))?;
        let output_path = args["output_path"]
            .as_str()
            .unwrap_or("screenshot.png")
            .to_string();

        let chrome_paths = if cfg!(windows) {
            vec![
                r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
                r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
            ]
        } else {
            vec!["google-chrome", "chromium-browser"]
        };

        let chrome = chrome_paths.iter().find(|p| {
            std::path::Path::new(p).exists() || which::which(p).is_ok()
        });

        let Some(chrome_path) = chrome else {
            return Ok(ResultatAbeille::err("Chrome/Edge not found."));
        };

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            Command::new(chrome_path)
                .args([
                    "--headless=new",
                    "--disable-gpu",
                    "--no-sandbox",
                    "--window-size=1280,720",
                    &format!("--screenshot={}", output_path),
                    url,
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                if std::path::Path::new(&output_path).exists() {
                    let size = std::fs::metadata(&output_path)
                        .map(|m| m.len())
                        .unwrap_or(0);
                    Ok(ResultatAbeille::ok(format!(
                        "Screenshot saved to: {} ({} KB)",
                        output_path,
                        size / 1024
                    )))
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Ok(ResultatAbeille::err(format!(
                        "Screenshot failed: {}",
                        &stderr[..stderr.len().min(300)]
                    )))
                }
            }
            Ok(Err(e)) => Ok(ResultatAbeille::err(format!("Chrome error: {}", e))),
            Err(_) => Ok(ResultatAbeille::err("Screenshot timed out")),
        }
    }
}

fn html_to_text_simple(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut tag_buf = String::new();

    for ch in html.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag_buf.clear();
            }
            '>' => {
                in_tag = false;
                let tag = tag_buf.to_lowercase();
                if tag.starts_with("script") { in_script = true; }
                else if tag.starts_with("/script") { in_script = false; }
                else if tag.starts_with("style") { in_style = true; }
                else if tag.starts_with("/style") { in_style = false; }
                else if matches!(tag.split_whitespace().next().unwrap_or("").trim_start_matches('/'),
                    "p" | "div" | "br" | "h1" | "h2" | "h3" | "h4" | "li" | "tr") {
                    result.push('\n');
                }
            }
            _ if in_tag => { tag_buf.push(ch); }
            _ if !in_script && !in_style => { result.push(ch); }
            _ => {}
        }
    }

    result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
