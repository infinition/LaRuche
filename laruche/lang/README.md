# Language file

Single source of truth for all web UI strings: one file, one column per language.

`strings.json`:

```json
{
  "common.save": { "en": "Save", "fr": "Enregistrer" },
  "common.cancel": { "en": "Cancel", "fr": "Annuler" }
}
```

## How it works

The node server reads the `laruche_lang` cookie (default `fr`), builds a flat
`{ key: value }` map for that language from `strings.json`, and injects it into
the page as `window.__I18N__` before `app.js` runs. In the browser,
`LaRuche.i18n.t('key')` does a flat lookup; `applyStatic()` translates the
static shell via `data-i18n*` attributes. The flat map per language is also
served at `/lang/<code>.json` for tooling. The same file is read server-side by
`laruche_essaim::i18n` for user-facing Rust strings.

## Add a language

1. For each key in `strings.json`, add a value under the new code, e.g.
   `"de": "..."`. No need to duplicate the keys: just add one column.
   Keep the `{placeholders}` (e.g. `{status}`, `{n}`) intact.
2. Keep LaRuche brand terms identical in every language: LaRuche, ruche, essaim,
   Miel, butinage, nectar, Source, escale, eclaireuse, curateur, vigie, boussole,
   jauge, carnet, recolte.
3. Add the code to `normalize_lang` / the `match` arms in
   `laruche-node/src/web.rs` and `laruche-essaim/src/i18n.rs`, then rebuild.
