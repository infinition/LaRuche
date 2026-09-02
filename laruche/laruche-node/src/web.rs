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
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse};
use axum::Json;

const SPA_HTML: &str = include_str!("../../laruche-dashboard/src/templates/spa.html");
// Single language file: { "key": { "en": "...", "fr": "..." } }. Adding a language = add a
// value per key (one column), no key duplication. Served flat per language to the front-end.
const LANG_STRINGS: &str = include_str!("../../lang/strings.json");
const APP_CSS: &str = include_str!("../../laruche-dashboard/src/templates/app.css");
// PWA assets (installable web app: add-to-home-screen on iPhone/Android, offline shell).
const MANIFEST_JSON: &str = include_str!("../../laruche-dashboard/src/templates/manifest.json");
const ICON_SVG: &str = include_str!("../../laruche-dashboard/src/templates/icon.svg");
// Bitmaps derives du meme SVG (cargo run -p laruche-icones). include_bytes! et non
// include_str!: ce sont des octets binaires, pas de l'UTF-8.
const ICON_PNG_192: &[u8] = include_bytes!("../../laruche-dashboard/src/templates/icones/icon-192.png");
const ICON_PNG_512: &[u8] = include_bytes!("../../laruche-dashboard/src/templates/icones/icon-512.png");
const SW_JS: &str = include_str!("../../laruche-dashboard/src/templates/sw.js");
// app.js is split into modules under `templates/js/` (one i18n agent per module). The node
// CONCATENATES them at compile time in dependency ORDER: one `/app.js` served, one binary.
const APP_JS: &str = concat!(
    // EN PREMIER, avant core.js: il peint le theme avant le premier rendu, depuis
    // le cache local. Plus bas dans la liste, l'interface clignoterait dans
    // l'ancien theme le temps que le module se charge.
    include_str!("../../laruche-dashboard/src/templates/js/themes.js"),
    "
",
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
    // After chat.js: it decorates rows chat.js creates, and calls back into LaRuche.Chat.
    include_str!("../../laruche-dashboard/src/templates/js/reactions.js"),
    "\n",
    include_str!("../../laruche-dashboard/src/templates/js/lareine-appel.js"),
    "\n",
    include_str!("../../laruche-dashboard/src/templates/js/watchers-graph.js"),
    "\n",
    // Apres chat.js: la table ronde partage sa zone de saisie et bascule son
    // conteneur.
    include_str!("../../laruche-dashboard/src/templates/js/tableronde.js"),
    "\n",
    // After settings.js and chat.js: the welcome modal calls into both
    // (Settings.ouvrirSection, Chat.sendMessage) as soon as it is opened.
    include_str!("../../laruche-dashboard/src/templates/js/accueil.js"),
    "\n",
    include_str!("../../laruche-dashboard/src/templates/js/boot.js"),
);

/// Picks the UI language from the `laruche_lang` cookie (default "fr").
/// Le fichier de langue du foyer.
///
/// Le choix ne vivait que dans un cookie et dans le `localStorage` du navigateur,
/// c'est-a-dire nulle part que le RESTE du programme puisse lire. L'ecran de
/// demarrage de l'application de bureau est une fenetre a part, servie depuis une
/// autre origine: il n'a acces ni a l'un ni a l'autre, et annoncait donc son
/// « Demarrage de LaRuche » en francais quelle que soit la langue reglee dans
/// l'interface. Un fichier, comme `themes/actif.txt` pour le theme, se lit depuis
/// n'importe ou et sans serveur.
pub(crate) fn langue_du_foyer() -> Option<String> {
    let c = std::fs::read_to_string("langue.txt").ok()?;
    let c = c.trim().to_string();
    (c == "en" || c == "fr").then_some(c)
}

/// POST /api/lang {"code":"en"} - enregistre la langue dans le foyer.
pub async fn api_lang_set(Json(body): Json<serde_json::Value>) -> impl IntoResponse {
    let code = body["code"].as_str().unwrap_or("");
    if code != "en" && code != "fr" {
        return (StatusCode::BAD_REQUEST, "code de langue inconnu").into_response();
    }
    match std::fs::write("langue.txt", code) {
        Ok(()) => (StatusCode::OK, "ok").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

fn ui_lang(headers: &HeaderMap) -> &'static str {
    // Le cookie d'abord: c'est le choix de CE navigateur, et il doit primer sur le
    // reglage general quand on ouvre l'interface depuis un autre appareil.
    let depuis_cookie = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|c| {
            c.split(';')
                .map(|s| s.trim())
                .find_map(|kv| kv.strip_prefix("laruche_lang="))
        })
        .map(|s| s.to_string());
    let code = depuis_cookie
        .or_else(langue_du_foyer)
        .unwrap_or_else(|| "fr".to_string());
    let code = code.as_str();
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
    // Le fichier suit la langue SERVIE, pas seulement les changements de langue.
    //
    // Il n'etait ecrit que par le bouton de bascule: quelqu'un qui avait deja
    // regle l'anglais avant que ce fichier n'existe n'en declenchait jamais
    // l'ecriture, et l'ecran de demarrage restait en francais sans qu'aucun geste
    // ne puisse le corriger, sauf a passer en francais puis revenir a l'anglais.
    // Ecrire ici rattrape ce cas des le premier chargement de l'interface.
    if langue_du_foyer().as_deref() != Some(lang) {
        let _ = std::fs::write("langue.txt", lang);
    }
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

/// PWA manifest (installable web app).
pub async fn manifest() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/manifest+json; charset=utf-8")],
        MANIFEST_JSON,
    )
}

/// App icon (home-screen / favicon).
pub async fn icon_svg() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")], ICON_SVG)
}

/// Icones PWA en bitmap. Windows et Android refusent un SVG pour l'icone installee:
/// avec le seul `icon.svg` au manifeste, « Installer LaRuche » aboutissait a une
/// vignette generique dans la barre des taches. Generees depuis ce meme SVG par
/// `cargo run -p laruche-icones`.
pub async fn icon_png_192() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/png")], ICON_PNG_192)
}

pub async fn icon_png_512() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/png")], ICON_PNG_512)
}

/// Service worker (offline shell + installability).
pub async fn service_worker() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        SW_JS,
    )
}

/// App JS (extracted from spa.html). Served before spa.html's small inline init script.
pub async fn app_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        APP_JS,
    )
}

// Vendored third-party libraries, served LOCALLY (local-first: the UI must render
// markdown offline) instead of a CDN, and with no external runtime dependency.
const VENDOR_MARKED: &str =
    include_str!("../../laruche-dashboard/src/templates/vendor/marked.min.js");
const VENDOR_PURIFY: &str =
    include_str!("../../laruche-dashboard/src/templates/vendor/purify.min.js");
const VENDOR_HLJS: &str =
    include_str!("../../laruche-dashboard/src/templates/vendor/highlight.min.js");

/// Serves a vendored JS library by name (`marked` | `purify` | `highlight`).
pub async fn vendor_js(Path(name): Path<String>) -> impl IntoResponse {
    let body = match name.trim_end_matches(".min.js").trim_end_matches(".js") {
        "marked" => VENDOR_MARKED,
        "purify" | "dompurify" => VENDOR_PURIFY,
        "highlight" | "highlight.min" => VENDOR_HLJS,
        _ => "",
    };
    let code = if body.is_empty() {
        axum::http::StatusCode::NOT_FOUND
    } else {
        axum::http::StatusCode::OK
    };
    (
        code,
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        body,
    )
}
