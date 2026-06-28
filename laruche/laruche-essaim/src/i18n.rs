//! Server-side i18n. Reads the same flat language files as the web UI
//! (laruche/lang/<code>.json) so there is a single source of truth across the
//! whole project. Only user-facing strings (channel messages, status shown to
//! the user, error messages, feed labels) are translated; logs and LLM prompts
//! stay in English. Adding a language: drop a laruche/lang/<code>.json file and
//! add a match arm in `catalog` / `normalize_lang`.

use std::collections::HashMap;
use std::sync::OnceLock;

// Single language file: { "key": { "en": "...", "fr": "..." } }.
const LANG_STRINGS: &str = include_str!("../../lang/strings.json");

/// Flat { key -> value } catalog for a language, built once (per-key fallback to English).
fn catalog(lang: &str) -> &'static HashMap<String, String> {
    static EN: OnceLock<HashMap<String, String>> = OnceLock::new();
    static FR: OnceLock<HashMap<String, String>> = OnceLock::new();
    fn build(code: &str) -> HashMap<String, String> {
        let table: HashMap<String, HashMap<String, String>> =
            serde_json::from_str(LANG_STRINGS).unwrap_or_default();
        table
            .into_iter()
            .map(|(k, langs)| {
                let v = langs
                    .get(code)
                    .or_else(|| langs.get("en"))
                    .cloned()
                    .unwrap_or_default();
                (k, v)
            })
            .collect()
    }
    match lang {
        "en" => EN.get_or_init(|| build("en")),
        _ => FR.get_or_init(|| build("fr")),
    }
}

/// Normalize an arbitrary language tag (e.g. "en-US", "fr_FR") to a supported
/// code. Unsupported tags fall back to the LaRuche default, "fr".
pub fn normalize_lang(tag: &str) -> &'static str {
    let t = tag.trim().to_lowercase();
    if t.starts_with("en") {
        "en"
    } else {
        "fr"
    }
}

/// Translate `key` for `lang`, substituting `{name}` placeholders from `vars`.
/// Falls back to the key itself when missing, so a gap is visible rather than blank.
pub fn t(key: &str, lang: &str, vars: &[(&str, &str)]) -> String {
    let lang = normalize_lang(lang);
    let mut s = catalog(lang)
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_string());
    for (k, v) in vars {
        s = s.replace(&format!("{{{}}}", k), v);
    }
    s
}

/// Convenience for the no-placeholder case.
pub fn tr(key: &str, lang: &str) -> String {
    t(key, lang, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_key_per_lang() {
        // 'common.save' exists in both language files.
        assert_eq!(t("common.save", "fr", &[]), "Enregistrer");
        assert_eq!(t("common.save", "en", &[]), "Save");
    }

    #[test]
    fn falls_back_to_key_when_missing() {
        assert_eq!(t("nope.nope", "fr", &[]), "nope.nope");
    }

    #[test]
    fn substitutes_placeholders() {
        // dashboard.inferError = "Erreur {status}" (fr) / "Error {status}" (en).
        assert_eq!(
            t("dashboard.inferError", "en", &[("status", "500")]),
            "Error 500"
        );
    }

    #[test]
    fn normalizes_tags() {
        assert_eq!(normalize_lang("en-US"), "en");
        assert_eq!(normalize_lang("fr-FR"), "fr");
        assert_eq!(normalize_lang("de"), "fr"); // unsupported -> default
    }
}
