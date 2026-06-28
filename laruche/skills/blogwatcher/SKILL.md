---
type: skill
name: blogwatcher
description: "Track RSS/Atom feeds and blogs via blogwatcher-cli."
version: 2.0.0
author: JulienTant (fork of Hyaxia/blogwatcher)
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec]
metadata:
  laruche:
    tags: [RSS, Blogs, Feed-Reader, Monitoring]
    homepage: https://github.com/JulienTant/blogwatcher-cli
prerequisites:
  commands: [blogwatcher-cli]
---

# Blogwatcher

Track blog and RSS/Atom feed updates with `blogwatcher-cli`. Supports automatic feed discovery, HTML scraping fallback, OPML import, and read/unread article management. Run all commands via `shell_exec`.

## Installation

**Go (recommended):**
```bash
go install github.com/JulienTant/blogwatcher-cli/cmd/blogwatcher-cli@latest
```

**Binary (replace `linux_amd64` with your target: `linux_arm64`, `darwin_arm64`, `darwin_amd64`):**
```bash
curl -sL https://github.com/JulienTant/blogwatcher-cli/releases/latest/download/blogwatcher-cli_linux_amd64.tar.gz \
  | tar xz -C /usr/local/bin blogwatcher-cli
```

**Docker (persist the DB or it resets on container restart):**
```bash
docker run --rm -v blogwatcher-cli:/data \
  -e BLOGWATCHER_DB=/data/blogwatcher-cli.db \
  ghcr.io/julientant/blogwatcher-cli scan
```

Default DB path: `~/.blogwatcher-cli/blogwatcher-cli.db`

All releases: https://github.com/JulienTant/blogwatcher-cli/releases

## Managing Blogs

```bash
blogwatcher-cli add "My Blog" https://example.com                                   # auto-discover feed
blogwatcher-cli add "My Blog" https://example.com --feed-url https://example.com/feed.xml
blogwatcher-cli add "My Blog" https://example.com --scrape-selector "article h2 a"  # HTML scraping fallback
blogwatcher-cli blogs                     # list tracked blogs
blogwatcher-cli remove "My Blog" --yes
blogwatcher-cli import subscriptions.opml # bulk import (Feedly, Inoreader, NewsBlur, etc.)
```

## Scanning and Reading

```bash
blogwatcher-cli scan                           # scan all blogs
blogwatcher-cli scan "My Blog"                 # scan one blog
blogwatcher-cli articles                       # list unread articles
blogwatcher-cli articles --all
blogwatcher-cli articles --blog "My Blog"
blogwatcher-cli articles --category "Engineering"
blogwatcher-cli read 1                         # mark article read by ID
blogwatcher-cli unread 1
blogwatcher-cli read-all
blogwatcher-cli read-all --blog "My Blog" --yes
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `BLOGWATCHER_DB` | `~/.blogwatcher-cli/blogwatcher-cli.db` | SQLite database path |
| `BLOGWATCHER_WORKERS` | 8 | Concurrent scan workers |
| `BLOGWATCHER_SILENT` | - | Only output "scan done" on scan |
| `BLOGWATCHER_YES` | - | Skip confirmation prompts |
| `BLOGWATCHER_CATEGORY` | - | Default article filter by category |

All flags can also be set via matching `BLOGWATCHER_*` env vars.

## LaRuche Usage Pattern

```python
# Scan and surface unread articles into memory
shell_exec("blogwatcher-cli scan")
articles = shell_exec("blogwatcher-cli articles")
memory_write("blogwatcher/unread", articles)
```

Run `blogwatcher-cli <command> --help` to see all flags.
