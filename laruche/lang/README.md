# Language files

Single source of truth for all web UI strings. One flat JSON file per language:
`key -> translated string`.

- `en.json` - English
- `fr.json` - French (LaRuche default)

## How it works

The node server reads the `laruche_lang` cookie (default `fr`), embeds the
matching file (`include_str!`) and injects it into the page as
`window.__I18N__` before `app.js` runs. In the browser, `LaRuche.i18n.t('key')`
does a flat lookup. The language toggle sets the cookie and reloads, so the
server injects the new language. Files are also served at `/lang/<code>.json`.

## Add a language

1. Copy `en.json` to `<code>.json` (e.g. `de.json`) and translate the values.
   Keep the `{placeholders}` (e.g. `{status}`, `{n}`) intact.
2. Keep LaRuche brand terms identical in every language: LaRuche, ruche, essaim,
   Miel, butinage, nectar, Source, escale, eclaireuse, curateur, vigie, boussole,
   jauge, carnet, recolte.
3. Wire it in `laruche-node/src/main.rs`: add `const LANG_DE` and a match arm in
   `lang_data` / `lang_file`, then rebuild.

## Regenerate from code

These files were generated from the inline dictionaries still present in the JS
modules (kept as a runtime fallback). The inline dictionaries can be re-exported
with the extraction script if they change. The JSON files are authoritative at
runtime.
