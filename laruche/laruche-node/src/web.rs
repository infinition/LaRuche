//! Static web asset serving (the SPA shell, CSS, concatenated JS) and i18n
//! language-file injection. Everything is compiled into the binary via
//! `include_str!`, so there is a single self-contained executable.
//!
//! The language files (`laruche/lang/<code>.json`) are the single source of
//! truth for UI strings, shared with the front-end. `spa_page` picks the active
//! language from the `laruche_lang` cookie and injects it as `window.__I18N__`
//! before `app.js` runs. Adding a language: drop a `lang/<code>.json` file and
//! add a match arm in `lang_data` / `lang_file`.

use axum::extract::Path;
use axum::http::{header, HeaderMap};
use axum::response::{Html, IntoResponse};

const SPA_HTML: &str = include_str!("../../laruche-dashboard/src/templates/spa.html");
// Single language file: { "key": { "en": "...", "fr": "..." } }. Adding a language = add a
// value per key (one column), no key duplication. Served flat per language to the front-end.
const LANG_STRINGS: &str = include_str!("../../lang/strings.json");
const APP_CSS: &str = include_str!("../../laruche-dashboard/src/templates/app.css");
// app.js is split into modules under `templates/js/` (one i18n agent per module). The node
// CONCATENATES them at compile time in dependency ORDER: one `/app.js` served, one binary.
const APP_JS: &str = concat!(
    include_str!("../../laruche-dashboard/src/templates/js/core.js"),
    "\n",
    include_str!("../../laruche-dashboard/src/templates/js/chat.js"),
    "\n",
    include_str!("../../laruche-dashboard/src/templates/js/dashboard.js"),
    "\n",
    include_str!("../../laruche-dashboard/src/templates/js/memory.js"),
    "\n",
    include_str!("../../laruche-dashboard/src/templates/js/missions.js"),
    "\n",
    include_str!("../../laruche-dashboard/src/templates/js/settings.js"),
    "\n",
    include_str!("../../laruche-dashboard/src/templates/js/automations.js"),
    "\n",
    include_str!("../../laruche-dashboard/src/templates/js/capabilities.js"),
    "\n",
    include_str!("../../laruche-dashboard/src/templates/js/boot.js"),
);

/// Picks the UI language from the `laruche_lang` cookie (default "fr").
fn ui_lang(headers: &HeaderMap) -> &'static str {
    let code = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|c| {
            c.split(';')
                .map(|s| s.trim())
                .find_map(|kv| kv.strip_prefix("laruche_lang="))
        })
        .unwrap_or("fr");
    if code == "en" {
        "en"
    } else {
        "fr"
    }
}

type StringTable = std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>;

/// Parsed language strings (key -> { lang -> value }), loaded from strings.json once.
fn all_strings() -> &'static StringTable {
    static S: std::sync::OnceLock<StringTable> = std::sync::OnceLock::new();
    S.get_or_init(|| serde_json::from_str(LANG_STRINGS).unwrap_or_default())
}

/// Flat { key: value } JSON for a language code (per-key fallback to English). Built and cached once.
fn lang_flat_json(code: &str) -> &'static str {
    static EN: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    static FR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    fn build(code: &str) -> String {
        let mut m = serde_json::Map::new();
        for (k, langs) in all_strings() {
            let v = langs
                .get(code)
                .or_else(|| langs.get("en"))
                .cloned()
                .unwrap_or_default();
            m.insert(k.clone(), serde_json::Value::String(v));
        }
        serde_json::Value::Object(m).to_string()
    }
    match code {
        "en" => EN.get_or_init(|| build("en")),
        _ => FR.get_or_init(|| build("fr")),
    }
}

/// GET / (and the SPA client routes): serve the shell with the active language injected.
pub async fn spa_page(headers: HeaderMap) -> Html<String> {
    let lang = ui_lang(&headers);
    // Escape '<' so a translation value can never break out of the inline <script> tag.
    // '<' is a valid escape in both JSON and JS string literals.
    let data = lang_flat_json(lang).replace('<', "\\u003c");
    let inject = format!("<script>window.__LANG__=\"{lang}\";window.__I18N__={data};</script>");
    Html(SPA_HTML.replacen("<!--__LANG_INJECT__-->", &inject, 1))
}

/// GET /lang/<code>.json - serve the flat translation map for a language (tooling/translators).
pub async fn lang_file(Path(file): Path<String>) -> impl IntoResponse {
    let code = file.trim_end_matches(".json");
    let body = match code {
        "en" => lang_flat_json("en"),
        _ => lang_flat_json("fr"),
    };
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        body,
    )
}

/// App CSS (extracted from spa.html). Explicit Content-Type so the browser applies it.
pub async fn app_css() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], APP_CSS)
}

/// App JS (extracted from spa.html). Served before spa.html's small inline init script.
pub async fn app_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        APP_JS,
    )
}
