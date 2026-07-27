---
type: skill
name: blogwatcher
description: Track RSS/Atom feeds and blogs via blogwatcher-cli.
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

```
shell_exec: blogwatcher-cli scan
shell_exec: blogwatcher-cli articles
```

Then read the article list and act on it. Do NOT pipe it into memory wholesale:
`memory_write` takes `node_id` and `content`, it stores ONE fact per call, and a node id
is dotted and snake_case (`veille.blogs`), never a slash path. A dump of every unread
headline poisons every later `memory_search` with noise nobody can prune. Write the
conclusion instead, one item at a time:

```
memory_write: node_id=veille.blogs, content=<the one thing worth remembering>
```

For the raw list, `file_write` it to disk and keep the path. For passages you may want to
quote later, `knowledge_add`, which is the store built for text.

## Traps

- **`scan` is what fetches.** `articles` only reads the local database, so a listing
  without a preceding `scan` shows yesterday's world and looks like nothing was published.
- **The Docker form needs a volume.** Without `-v`, the database lives in the container
  and every article is unread again after a restart.
- **`read-all` is not undoable.** Confirm with the user before running it, and prefer
  `read-all --blog "<name>"` over the unscoped form.
- **Auto-discovery fails silently on some sites.** A blog added with no feed found stays
  in the list and never produces an article. If a blog never yields anything, re-add it
  with an explicit `--feed-url`, or with `--scrape-selector`.

## Failure modes

**`blogwatcher-cli: command not found`.** Not installed, or the Go bin directory is not
on PATH. Install with one of the commands above, then confirm with
`blogwatcher-cli blogs`.

**`scan` finds nothing for a blog that clearly publishes.** No feed was discovered.
Fetch the site and look for a `<link rel="alternate" type="application/rss+xml">` tag,
then re-add the blog with `--feed-url`. If the site has no feed at all, use
`--scrape-selector` with a CSS selector matching the article links.

**Every article comes back unread after a restart.** The database moved. Set
`BLOGWATCHER_DB` to a fixed absolute path, and use it consistently.

Run `blogwatcher-cli <command> --help` to see all flags.
