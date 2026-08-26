//! web_fetch: fetch a page and return CLEAN, PAGINATED text.
//!
//! War-machine spec:
//! - readability extraction (article/main selectors, boilerplate stripped);
//! - **pagination**: `offset`/`max_chars` let the model read a LONG page in several
//!   calls instead of losing everything past a tiny cap;
//! - **links extraction** (`include_links`): the model can crawl OUTWARD (find the
//!   download page, the forum thread) instead of being stuck on one page;
//! - **`focus`**: return the passages that answer the question instead of the
//!   first N characters. Blind truncation is not just lossy, it can be a total
//!   miss: on the 145k-char Wikipedia article for mitochondria, zero of the 12
//!   "apoptosis" mentions fall inside the default 12k window;
//! - anti-blocking chain: direct (1 retry) → r.jina.ai → Wayback Machine;
//! - PDFs routed through jina (which renders them as text) instead of binary garbage;
//! - `render=true`: headless Chrome/Edge for JS-only pages.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

/// Fetch a web page and return its text content.
pub struct WebFetch;

const MAX_CHARS_DEFAUT: usize = 12_000;

#[async_trait]
impl Abeille for WebFetch {
    fn nom(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch a web page and return its clean text. Long pages are PAGINATED: the output \
         tells you the total size and the `offset` to pass to read the next chunk. Set \
         `include_links` to true to also get the page's links (to crawl further). Set \
         `render` to true for JavaScript-only pages. PDFs are converted to text."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "The URL to fetch" },
                "offset": { "type": "integer", "description": "Character offset to continue reading a long page (from a previous call's hint). Default 0." },
                "max_chars": { "type": "integer", "description": "Max characters returned (default 12000, max 40000)" },
                "include_links": { "type": "boolean", "description": "Append the page's links (text + URL) to crawl further" },
                "render": { "type": "boolean", "description": "Headless Chrome/Edge render before extraction (JS pages)" },
                "focus": { "type": "string", "description": "What you are looking for on this page (e.g. 'apoptosis', 'pricing tiers'). Returns only the passages that match, in document order, instead of the first N characters. Use it whenever you have a specific question: on a long page, blind truncation often returns none of the answer." }
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
        let url_raw = args["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' argument"))?;
        // Secret substitution (`${NAME}` / `@@NAME`): allows `web_fetch @@webhook_get` without
        // ever exposing the value to the LLM (outbound tool).
        let url_sub = crate::secrets::substituer(url_raw);
        let url = url_sub.as_str();

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Ok(ResultatAbeille::err(
                "URL must start with http:// or https://",
            ));
        }

        let offset = args["offset"].as_u64().unwrap_or(0) as usize;
        let max_chars = (args["max_chars"].as_u64().unwrap_or(MAX_CHARS_DEFAUT as u64) as usize)
            .clamp(1_000, 40_000);
        let avec_liens = args["include_links"].as_bool().unwrap_or(false);
        let focus = args["focus"].as_str().unwrap_or("").to_string();

        if args["render"].as_bool().unwrap_or(false) {
            return match render_url_dom(url, 3).await {
                Ok(html) => {
                    let texte = extraire_lisible(&html);
                    let liens = if avec_liens { rendu_liens(&html, url) } else { String::new() };
                    Ok(ResultatAbeille::ok(format!(
                        "{}{liens}",
                        presenter(&texte, &focus, offset, max_chars)
                    )))
                }
                Err(e) => Ok(ResultatAbeille::err(format!(
                    "Render requested but browser rendering failed: {e}"
                ))),
            };
        }

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_secs(20))
            .build()?;

        // Direct fetch (1 retry on transient network error), then anti-blocking chain.
        let direct = match fetch_direct(&client, url).await {
            Err(e) if e.to_string().starts_with("network") => fetch_direct(&client, url).await,
            autre => autre,
        };
        match direct {
            Ok(Recolte::Pdf) => {
                // r.jina.ai renders PDFs to text; direct bytes would be garbage.
                if let Some(text) = fetch_via_jina(&client, url).await {
                    return Ok(ResultatAbeille::ok(format!(
                        "[PDF converted via r.jina.ai]\n\n{}",
                        presenter(&text, &focus, offset, max_chars)
                    )));
                }
                Ok(ResultatAbeille::err(
                    "PDF detected and jina conversion failed. Download it (shell/file tools) \
                     and use read_extract on the local file.",
                ))
            }
            Ok(Recolte::Html(html)) => {
                let texte = extraire_lisible(&html);
                if texte.trim().is_empty() || looks_like_js_shell(&texte) {
                    // Empty/JS shell: try jina (renders JS) before giving up.
                    if let Some(text) = fetch_via_jina(&client, url).await {
                        return Ok(ResultatAbeille::ok(format!(
                            "[via r.jina.ai - page needed rendering]\n\n{}",
                            presenter(&text, &focus, offset, max_chars)
                        )));
                    }
                    return Ok(ResultatAbeille::err(
                        "Page has no readable content (JS shell). Retry with render=true.",
                    ));
                }
                let liens = if avec_liens { rendu_liens(&html, url) } else { String::new() };
                Ok(ResultatAbeille::ok(format!(
                    "{}{liens}",
                    presenter(&texte, &focus, offset, max_chars)
                )))
            }
            Ok(Recolte::Texte(t)) if !t.trim().is_empty() => {
                Ok(ResultatAbeille::ok(presenter(&t, &focus, offset, max_chars)))
            }
            issue => {
                let motif = match issue {
                    Err(e) => e.to_string(),
                    _ => "empty page".to_string(),
                };
                // Fallback 1: r.jina.ai reader proxy (cleans, renders JS, bypasses simple 403s).
                if let Some(text) = fetch_via_jina(&client, url).await {
                    return Ok(ResultatAbeille::ok(format!(
                        "[via r.jina.ai - direct fetch failed: {motif}]\n\n{}",
                        presenter(&text, &focus, offset, max_chars)
                    )));
                }
                // Fallback 2: Wayback Machine (archived snapshot).
                if let Some(text) = fetch_via_wayback(&client, url).await {
                    return Ok(ResultatAbeille::ok(format!(
                        "[via Wayback Machine - direct fetch failed: {motif}]\n\n{}",
                        presenter(&text, &focus, offset, max_chars)
                    )));
                }
                Ok(ResultatAbeille::err(format!(
                    "Fetch failed (direct: {motif}). r.jina.ai and Wayback fallbacks unsuccessful. \
                     You can retry with render=true (browser rendering)."
                )))
            }
        }
    }
}

/// What a direct fetch yielded.
enum Recolte {
    Html(String),
    Texte(String),
    Pdf,
}

/// Direct fetch: raw HTML/text, or an error (network or non-2xx status).
async fn fetch_direct(client: &reqwest::Client, url: &str) -> Result<Recolte> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("network: {e}"))?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("HTTP {}", response.status()));
    }
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    if content_type.contains("pdf") {
        return Ok(Recolte::Pdf);
    }
    let body = response.text().await.unwrap_or_default();
    Ok(if content_type.contains("html") || body.trim_start().starts_with('<') {
        Recolte::Html(body)
    } else {
        Recolte::Texte(body)
    })
}

/// Fallback via r.jina.ai: reader proxy that already returns clean text/markdown
/// (with links inline). Shared with web_deep_search.
pub(crate) async fn fetch_via_jina(client: &reqwest::Client, url: &str) -> Option<String> {
    let proxied = format!("https://r.jina.ai/{url}");
    let resp = client
        .get(&proxied)
        .timeout(std::time::Duration::from_secs(25))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let text = resp.text().await.ok()?;
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

/// Fallback via the Wayback Machine (archive.org): closest snapshot.
async fn fetch_via_wayback(client: &reqwest::Client, url: &str) -> Option<String> {
    let api = format!(
        "https://archive.org/wayback/available?url={}",
        urlencoding::encode(url)
    );
    let v: serde_json::Value = client.get(&api).send().await.ok()?.json().await.ok()?;
    let snap = v["archived_snapshots"]["closest"]["url"].as_str()?;
    let html = client.get(snap).send().await.ok()?.text().await.ok()?;
    let text = extraire_lisible(&html);
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
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

/// Single presentation path: focus if asked, otherwise plain pagination.
///
/// Blind truncation can return NONE of what was asked for. Measured on the
/// Wikipedia article for mitochondria (145k chars): of 12 "apoptosis" mentions,
/// zero fall inside the default 12k window. The agent spends 3k tokens on the
/// lead section and infobox, sees nothing about apoptosis, and concludes the
/// page does not cover it. `focus` returns the matching passages instead.
pub(crate) fn presenter(texte: &str, focus: &str, offset: usize, max_chars: usize) -> String {
    if focus.trim().is_empty() {
        return paginer(texte, offset, max_chars);
    }
    match cibler(texte, focus, max_chars) {
        Some(cible) => cible,
        // Never silently return an empty result: say the focus missed and show
        // the page linearly, so the model can decide rather than guess.
        None => format!(
            "[focus=\"{focus}\" matched nothing on this page. Showing it from the top instead.]\n\n{}",
            paginer(texte, offset, max_chars)
        ),
    }
}

/// Keep the passages that match `focus`, in document order, within budget.
///
/// Returns `None` when nothing matches, so the caller can be honest about it.
fn cibler(texte: &str, focus: &str, max_chars: usize) -> Option<String> {
    let termes = termes_focus(focus);
    if termes.is_empty() {
        return None;
    }

    let blocs = decouper_en_blocs(texte);
    let mut notes: Vec<(usize, usize)> = blocs
        .iter()
        .enumerate()
        .map(|(i, b)| (i, score_bloc(b, &termes)))
        .filter(|(_, s)| *s > 0)
        .collect();
    if notes.is_empty() {
        return None;
    }

    // Best blocks first to spend the budget on them, then restore document
    // order so the extract still reads like the page.
    notes.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut retenus: Vec<usize> = Vec::new();
    let mut budget = max_chars;
    for (index, _) in notes.iter() {
        let cout = blocs[*index].chars().count();
        if cout > budget {
            continue;
        }
        budget -= cout;
        retenus.push(*index);
    }
    if retenus.is_empty() {
        return None;
    }
    retenus.sort_unstable();

    let mut sortie = String::new();
    let mut precedent: Option<usize> = None;
    for index in &retenus {
        // A gap must be visible: the model has to know the text is not contiguous.
        if precedent.is_some_and(|p| *index > p + 1) {
            sortie.push_str("\n\n[...]\n\n");
        } else if precedent.is_some() {
            sortie.push_str("\n\n");
        }
        sortie.push_str(blocs[*index].trim());
        precedent = Some(*index);
    }

    let total: usize = texte.chars().count();
    sortie.push_str(&format!(
        "\n\n[focus=\"{focus}\": kept {} of {} passages ({} of {total} chars). Gaps marked [...]. \
         Drop `focus` to read the page linearly.]",
        retenus.len(),
        blocs.len(),
        sortie.chars().count()
    ));
    Some(sortie)
}

/// Focus terms: lowercase, deduplicated, stopwords dropped.
fn termes_focus(focus: &str) -> Vec<String> {
    const VIDES: &[&str] = &[
        "the", "a", "an", "of", "and", "or", "in", "on", "for", "to", "is", "are", "what",
        "how", "le", "la", "les", "de", "des", "du", "et", "ou", "un", "une", "dans", "sur",
        "quoi", "comment", "quel", "quelle",
    ];
    let mut termes: Vec<String> = focus
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .map(|t| t.trim().to_lowercase())
        .filter(|t| t.len() >= 2 && !VIDES.contains(&t.as_str()))
        .collect();
    termes.sort();
    termes.dedup();
    termes
}

/// Split into passages: real paragraphs when the text has them, otherwise
/// sentence groups, because readability extraction often yields one long run.
fn decouper_en_blocs(texte: &str) -> Vec<String> {
    /// Target size of a synthetic block, in characters.
    const TAILLE_BLOC: usize = 400;

    let paragraphes: Vec<String> = texte
        .split("\n\n")
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    if paragraphes.len() >= 3 {
        return paragraphes;
    }

    let mut blocs = Vec::new();
    let mut courant = String::new();
    for phrase in texte.split_inclusive(['.', '!', '?', '\n']) {
        courant.push_str(phrase);
        if courant.chars().count() >= TAILLE_BLOC {
            blocs.push(std::mem::take(&mut courant));
        }
    }
    if !courant.trim().is_empty() {
        blocs.push(courant);
    }
    blocs
}

/// Relevance of one passage to the focus terms.
///
/// Distinct terms weigh far more than repetition: a passage covering two of the
/// asked-about terms answers better than one repeating a single term ten times.
fn score_bloc(bloc: &str, termes: &[String]) -> usize {
    let bas = bloc.to_lowercase();
    let mut distincts = 0;
    let mut occurrences = 0;
    for terme in termes {
        let n = bas.matches(terme.as_str()).count();
        if n > 0 {
            distincts += 1;
            occurrences += n;
        }
    }
    if distincts == 0 {
        return 0;
    }
    distincts * 10 + occurrences.min(10)
}

/// Char-safe pagination window over the extracted text. Tells the model the total
/// size and the exact `offset` for the next chunk - a long page is READ, not lost.
pub(crate) fn paginer(texte: &str, offset: usize, max_chars: usize) -> String {
    let total = texte.chars().count();
    if offset >= total && total > 0 {
        return format!(
            "(offset {offset} is past the end: the page has {total} characters total)"
        );
    }
    let fenetre: String = texte.chars().skip(offset).take(max_chars).collect();
    let fin = offset + fenetre.chars().count();
    if fin < total {
        format!(
            "{fenetre}\n\n[... page continues: {total} chars total, showing {offset}..{fin}. \
             Call web_fetch again with offset={fin} to read more. ...]"
        )
    } else if offset > 0 {
        format!("{fenetre}\n\n[end of page: {total} chars total]")
    } else {
        fenetre
    }
}

/// JS shell page: nothing readable without a renderer.
pub(crate) fn looks_like_js_shell(text: &str) -> bool {
    let lower = text.to_lowercase();
    let short = text.chars().count() < 500;
    short
        && (lower.contains("enable javascript")
            || lower.contains("requires javascript")
            || lower.contains("please enable js")
            || lower.contains("app shell"))
}

/// Readability extraction: prefer the densest article/main region (scraper
/// selectors), fall back to whole-page tag stripping. Shared with web_deep_search.
pub(crate) fn extraire_lisible(html: &str) -> String {
    use scraper::{Html, Selector};
    let doc = Html::parse_document(html);
    let mut best = String::new();
    let mut meilleur_score = i64::MIN;
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
            let brut = node.html();
            let text = html_to_text(&brut);
            // LINK DENSITY (standard readability metric): pick the container with the
            // most PROSE per link, not merely the largest one. Site navigation is a
            // big block of tiny anchors and used to win on raw length alone (measured:
            // a Nexus Mods page returned "My games / Your favourited games..." instead
            // of the mod list). Links are NOT stripped - a page of download links is
            // exactly what a scout is after; only the choice of container changes.
            let liens = brut.matches("<a ").count() as i64;
            let score = text.chars().count() as i64 - 40 * liens;
            if score > meilleur_score {
                meilleur_score = score;
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

/// Extracts the page's links (anchor text + ABSOLUTE url), deduplicated, capped.
/// This is what lets a scout CRAWL: find the download page, the next forum page...
pub(crate) fn extraire_liens(html: &str, base: &str) -> Vec<(String, String)> {
    use scraper::{Html, Selector};
    let doc = Html::parse_document(html);
    let Ok(sel) = Selector::parse("a[href]") else {
        return Vec::new();
    };
    let base_url = reqwest::Url::parse(base).ok();
    let mut vus = std::collections::HashSet::new();
    let mut liens = Vec::new();
    for a in doc.select(&sel) {
        let Some(href) = a.value().attr("href") else { continue };
        // Resolve relative hrefs against the page URL.
        let abs = match reqwest::Url::parse(href) {
            Ok(u) => u.to_string(),
            Err(_) => match &base_url {
                Some(b) => match b.join(href) {
                    Ok(u) => u.to_string(),
                    Err(_) => continue,
                },
                None => continue,
            },
        };
        if !abs.starts_with("http") {
            continue;
        }
        let texte: String = a.text().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");
        let texte: String = texte.chars().take(80).collect();
        if texte.is_empty() {
            continue;
        }
        let cle = abs.split('#').next().unwrap_or(&abs).to_string();
        if vus.insert(cle) {
            liens.push((texte, abs));
            if liens.len() >= 40 {
                break;
            }
        }
    }
    liens
}

/// Links section appended to the output when `include_links=true`.
fn rendu_liens(html: &str, base: &str) -> String {
    let liens = extraire_liens(html, base);
    if liens.is_empty() {
        return String::new();
    }
    let mut out = format!("\n\n## Links on this page ({})\n", liens.len());
    for (texte, url) in liens {
        // Pipe, not a dash: link texts very often contain their own dashes,
        // and the model has to tell the label from the URL at a glance.
        out.push_str(&format!("- {texte} | {url}\n"));
    }
    out
}

/// Simple HTML to text converter: strips tags, scripts, styles, boilerplate.
pub(crate) fn html_to_text(html: &str) -> String {
    // Boilerplate regions to skip (nav/menus/footer/ads): this is what polluted
    // extraction and drowned out the real content. We do NOT touch <header>
    // (may contain the article title) or <main>/<article>.
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
                // Track boilerplate region depth (handles nesting).
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
    fn schema_exposes_new_parameters() {
        let schema = WebFetch.schema();
        for p in ["render", "offset", "max_chars", "include_links"] {
            assert!(schema["properties"].get(p).is_some(), "missing {p}");
        }
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

    /// The measured failure: on a long page, blind truncation can return none
    /// of what was asked for.
    #[test]
    fn focus_ramene_le_passage_que_la_troncature_perd() {
        let page = format!(
            "{}

Apoptosis is triggered by cytochrome c release.",
            "Lead section filler. ".repeat(400)
        );
        // Linear reading at the default budget never reaches the passage.
        assert!(!paginer(&page, 0, 2_000).contains("Apoptosis"));
        let cible = presenter(&page, "apoptosis", 0, 2_000);
        assert!(cible.contains("cytochrome c"), "focus lost the answer");
        assert!(cible.contains("kept 1 of"), "focus report missing");
    }

    #[test]
    fn focus_qui_ne_matche_rien_le_dit_et_ne_rend_pas_le_vide() {
        let page = "Some page about mitochondria.".to_string();
        let sortie = presenter(&page, "quantum chromodynamics", 0, 2_000);
        assert!(sortie.contains("matched nothing"));
        // The page still comes through: a miss must not cost the content.
        assert!(sortie.contains("mitochondria"));
    }

    #[test]
    fn focus_marque_les_trous_et_garde_lordre_du_document() {
        let page = "alpha ATP one.

filler filler.

beta ATP two.

omega end.";
        let sortie = presenter(page, "ATP", 0, 4_000);
        let i_alpha = sortie.find("alpha").expect("first match kept");
        let i_beta = sortie.find("beta").expect("second match kept");
        assert!(i_alpha < i_beta, "document order broken");
        assert!(sortie.contains("[...]"), "gap not marked");
        assert!(!sortie.contains("filler"), "non-matching block kept");
    }

    #[test]
    fn score_bloc_prefere_la_couverture_a_la_repetition() {
        let termes = vec!["atp".to_string(), "synthase".to_string()];
        let deux = score_bloc("ATP synthase sits in the membrane", &termes);
        let un = score_bloc("ATP ATP ATP ATP ATP ATP", &termes);
        assert!(deux > un, "repetition should not beat coverage");
    }

    #[test]
    fn termes_focus_ecarte_les_mots_vides() {
        assert_eq!(termes_focus("what is the role of apoptosis"), vec!["apoptosis", "role"]);
        assert!(termes_focus("the of and").is_empty());
    }

    #[test]
    fn paginer_fenetre_et_offset() {
        let texte: String = "abcdefghij".repeat(100); // 1000 chars
        // first window announces the continuation offset
        let p0 = paginer(&texte, 0, 400);
        assert!(p0.contains("offset=400"));
        assert!(p0.contains("1000 chars total"));
        // middle window
        let p1 = paginer(&texte, 400, 400);
        assert!(p1.contains("offset=800"));
        // last window marks the end
        let p2 = paginer(&texte, 800, 400);
        assert!(p2.contains("end of page"));
        // short text: returned as-is
        assert_eq!(paginer("court", 0, 400), "court");
        // char-safe with multibyte (no panic)
        let acc = "éàü".repeat(500);
        let _ = paginer(&acc, 100, 200);
    }

    #[test]
    fn extraire_liens_absolus_dedupliques() {
        let html = r##"<html><body>
            <a href="/download/save.zip">Download savegame</a>
            <a href="https://example.org/page2">Page 2</a>
            <a href="/download/save.zip">Download savegame (dup)</a>
            <a href="#anchor">ignore</a>
        </body></html>"##;
        let liens = extraire_liens(html, "https://example.org/base/");
        assert_eq!(liens.len(), 3, "{liens:?}"); // dup collapsed, anchor resolves to base
        assert!(liens.iter().any(|(t, u)| t.contains("Download")
            && u == "https://example.org/download/save.zip"));
    }

    #[test]
    fn readability_prefere_l_article() {
        let html = format!(
            "<html><body><nav>menu</nav><article><p>{}</p></article><footer>legal</footer></body></html>",
            "contenu utile ".repeat(80)
        );
        let t = extraire_lisible(&html);
        assert!(t.contains("contenu utile"));
        assert!(!t.contains("menu"));
    }

    #[test]
    fn readability_ecarte_la_navigation_dense_en_liens() {
        // `main` = site chrome: LONGER than the article but made of tiny anchors
        // (the real Nexus Mods case). Link density must make the article win.
        let nav_links = (0..60)
            .map(|i| format!("<a href=\"/x{i}\">Mod category {i}</a>"))
            .collect::<String>();
        let html = format!(
            "<html><body><main>My games. Your favourited games will be displayed here. {nav_links}</main>\
             <article><p>{}</p></article></body></html>",
            "sauvegarde .dsparty documentee ".repeat(30)
        );
        let t = extraire_lisible(&html);
        assert!(t.contains("sauvegarde .dsparty"), "the article wins: {t:.120}");
        assert!(!t.contains("Your favourited games"), "nav chrome discarded");
    }

    #[test]
    fn readability_garde_une_page_de_liens_utiles() {
        // No prose competitor: a page that IS a list of download links must survive
        // intact - stripping links would destroy exactly what a scout is after.
        let liens = (0..12)
            .map(|i| format!("<a href=\"/dl{i}\">savegame_pack_{i}.dsparty</a> "))
            .collect::<String>();
        let html = format!("<html><body><main>Downloads: {liens}</main></body></html>");
        let t = extraire_lisible(&html);
        assert!(t.contains("savegame_pack_0.dsparty"), "download links kept: {t:.160}");
    }
}
