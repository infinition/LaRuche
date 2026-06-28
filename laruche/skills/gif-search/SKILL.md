---
type: skill
name: gif-search
description: "Search and download GIFs from Tenor v2 API via curl + jq."
version: 1.2.0
license: MIT
platforms: [linux, macos, windows]
prerequisites:
  env_vars: [TENOR_API_KEY]
  commands: [curl, jq]
tools: [shell_exec]
metadata:
  laruche:
    tags: [GIF, Media, Search, Tenor, API]
---

# GIF Search (Tenor API)

Search and download GIFs via the Tenor v2 API using `curl` + `jq`. Run all commands with `shell_exec`. The secret `TENOR_API_KEY` is injected automatically from the LaRuche vault - no manual export needed.

Get a free API key at https://developers.google.com/tenor/guides/quickstart.

## Search - get URLs

```bash
# Full-quality GIFs
curl -s "https://tenor.googleapis.com/v2/search?q=thumbs+up&limit=5&key=${TENOR_API_KEY}" \
  | jq -r '.results[].media_formats.gif.url'

# Lightweight preview GIFs (preferred for chat/markdown)
curl -s "https://tenor.googleapis.com/v2/search?q=nice+work&limit=3&key=${TENOR_API_KEY}" \
  | jq -r '.results[].media_formats.tinygif.url'
```

## Download top result

```bash
URL=$(curl -s "https://tenor.googleapis.com/v2/search?q=celebration&limit=1&key=${TENOR_API_KEY}" \
  | jq -r '.results[0].media_formats.gif.url')
curl -sL "$URL" -o celebration.gif
```

## Full metadata per result

```bash
curl -s "https://tenor.googleapis.com/v2/search?q=cat&limit=3&key=${TENOR_API_KEY}" \
  | jq '.results[] | {title, url: .media_formats.gif.url, preview: .media_formats.tinygif.url, dims: .media_formats.gif.dims}'
```

## API parameters

| Parameter       | Description |
|-----------------|-------------|
| `q`             | Search query (URL-encode spaces as `+`) |
| `limit`         | Results 1–50 (default 20) |
| `key`           | `${TENOR_API_KEY}` |
| `media_filter`  | Formats to return: `gif`, `tinygif`, `mp4`, `tinymp4`, `webm` |
| `contentfilter` | Safety: `off`, `low`, `medium`, `high` |
| `locale`        | Language: `en_US`, `es`, `fr`, … |

## Media formats

| Format    | Use case |
|-----------|----------|
| `gif`     | Full quality GIF |
| `tinygif` | Small preview GIF |
| `mp4`     | Video (smaller than GIF) |
| `tinymp4` | Small video |
| `webm`    | WebM video |
| `nanogif` | Tiny thumbnail |

## Notes

- URL-encode queries: spaces → `+`, special chars → `%XX`.
- Embed GIFs directly in markdown: `![alt](url)`.
- If `${TENOR_API_KEY}` is missing or invalid, the API returns 403 - verify the secret is set in the LaRuche vault.
- Prefer `tinygif` over `gif` for embedding in chat to save bandwidth.
