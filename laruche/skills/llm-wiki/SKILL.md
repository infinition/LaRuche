---
type: skill
name: llm-wiki
description: "Build and query a Karpathy-style interlinked markdown knowledge base."
version: 2.1.0
author: LaRuche
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec, execute_code, file_write, file_read, file_list, file_edit, read_extract, web_fetch, memory_write]
metadata:
  laruche:
    tags: [wiki, knowledge-base, research, notes, markdown, rag-alternative]
    category: research
    related_skills: [obsidian]
---

# Karpathy's LLM Wiki

Build and maintain a persistent, compounding knowledge base as interlinked markdown files.
Based on [Andrej Karpathy's LLM Wiki pattern](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f).

Unlike RAG (which rediscovers knowledge per query), the wiki compiles knowledge once, keeps it current, and pre-builds cross-references and contradiction flags.

**Division of labor:** Human curates sources and directs analysis. Agent summarizes, cross-references, files, and maintains consistency.

## When This Skill Activates

- User asks to create, build, or start a wiki / knowledge base
- User asks to ingest, add, or process a source into their wiki
- User asks a question and an existing wiki is present at the configured path
- User asks to lint, audit, or health-check their wiki

## Wiki Location

```bash
WIKI="${WIKI_PATH:-$HOME/wiki}"
```

Set `WIKI_PATH` in environment (via LaRuche secrets vault as `${WIKI_PATH}`, or default `~/wiki`). The wiki is a plain directory of markdown files - Obsidian, VS Code, or any editor works.

## Architecture

```
wiki/
├── SCHEMA.md           # Conventions, structure rules, domain config, tag taxonomy
├── index.md            # Sectioned catalog: one line per page (wikilink + summary)
├── log.md              # Chronological action log (append-only, rotate at 500 entries)
├── raw/                # Layer 1 - immutable source material (never modify)
│   ├── articles/       # Web articles, clippings
│   ├── papers/         # PDFs, arxiv papers
│   ├── transcripts/    # Notes, interviews
│   └── assets/         # Images referenced by sources
├── entities/           # Layer 2 - entity pages (people, orgs, products, models)
├── concepts/           # Layer 2 - concept/topic pages
├── comparisons/        # Layer 2 - side-by-side analyses
└── queries/            # Layer 2 - filed query results worth keeping
```

**Layer 1 - Raw:** Agent reads, never writes. **Layer 2 - Wiki:** Agent-owned, cross-referenced pages. **SCHEMA.md:** Governs all agent behavior.

## CRITICAL: Orient Before Every Session

When a wiki already exists, do this **before any other action**:

```bash
WIKI="${WIKI_PATH:-$HOME/wiki}"
# read via file_read:
file_read "$WIKI/SCHEMA.md"   # conventions, tag taxonomy, domain scope
file_read "$WIKI/index.md"    # existing pages
file_read "$WIKI/log.md"      # tail ~30 entries for recent activity
```

For large wikis (100+ pages), also run `file_list` for the topic before creating anything. Skipping orientation causes duplicate pages and missed cross-references.

## Initializing a New Wiki

1. Resolve path from `${WIKI_PATH}` or ask user; default `~/wiki`
2. Create directory structure: `shell_exec "mkdir -p $WIKI/raw/articles $WIKI/raw/papers $WIKI/raw/transcripts $WIKI/raw/assets $WIKI/entities $WIKI/concepts $WIKI/comparisons $WIKI/queries"`
3. Ask user the domain - be specific
4. Write `SCHEMA.md` (domain, conventions, frontmatter spec, tag taxonomy, page thresholds)
5. Write `index.md` (sectioned header: Entities / Concepts / Comparisons / Queries)
6. Write `log.md` (`## [YYYY-MM-DD] create | Wiki initialized`)
7. Confirm and suggest first sources to ingest

### Key SCHEMA.md sections to include

- **Domain** - what the wiki covers
- **Conventions** - filename format (`lowercase-hyphens.md`), min 2 outbound `[[wikilinks]]` per page, bump `updated` on every edit
- **Frontmatter spec:**
  ```yaml
  ---
  title: Page Title
  created: YYYY-MM-DD
  updated: YYYY-MM-DD
  type: entity | concept | comparison | query
  tags: [taxonomy tags only]
  sources: [raw/articles/source-name.md]
  confidence: high | medium | low
  contested: true                   # set when unresolved contradictions exist
  contradictions: [other-slug]      # pages this one conflicts with
  ---
  ```
- **Raw source frontmatter:** `source_url`, `ingested`, `sha256` of body for drift detection
- **Tag taxonomy** - 10–20 top-level tags; new tags must be added here before use
- **Page thresholds** - create when entity appears in 2+ sources or is central to one; don't create for passing mentions; split pages >200 lines; archive superseded pages to `_archive/`
- **Update policy** - newer source generally supersedes; flag contradictions with dates and sources

## Core Operations

### 1. Ingest

① **Capture raw source:**
   - URL → `read_extract "<url>"` (or `web_fetch`) → `file_write "$WIKI/raw/articles/slug-YYYY.md"`
   - PDF → `read_extract "<url>"` → `file_write "$WIKI/raw/papers/slug-YYYY.md"`
   - Pasted text → `file_write` to appropriate `raw/` subdirectory
   - Add raw frontmatter: `source_url`, `ingested`, `sha256` of body
   - On re-ingest: recompute sha256 via `execute_code`, skip if unchanged, flag drift if changed

② **Discuss takeaways** with user (skip in automated/cron contexts)

③ **Check existing pages** - scan `index.md`, then `file_list "$WIKI"` for matching slugs. This is the difference between a compounding wiki and a pile of duplicates.

④ **Write or update wiki pages:**
   - New entities/concepts: create only if they meet Page Thresholds in SCHEMA.md
   - Existing pages: `file_edit` to add info, update facts, bump `updated` date; follow Update Policy on conflict
   - Cross-reference: every page must link to ≥2 others via `[[wikilinks]]`; check back-links exist
   - Tags: taxonomy only (SCHEMA.md)
   - Provenance: on pages synthesizing 3+ sources, append `^[raw/articles/source.md]` to claim paragraphs

⑤ **Update navigation:**
   - Add new pages to `index.md` alphabetically under the correct section; update page count and date
   - Append to log: `## [YYYY-MM-DD] ingest | Source Title` + list of files created/updated

⑥ **Report** every file created or updated to the user. One source can legitimately touch 5–15 pages.

### 2. Query

① `file_read "$WIKI/index.md"` - identify relevant pages
② For wikis >100 pages: `file_search` or scan file contents for key terms via `execute_code`
③ `file_read` each relevant page
④ Synthesize answer, cite sources: "Based on [[page-a]] and [[page-b]]…"
⑤ File the answer if it's a substantial synthesis: `file_write "$WIKI/queries/slug.md"`. Skip trivial lookups.
⑥ Append to `log.md`

### 3. Lint

Run these checks; report grouped by severity (broken links → orphans → source drift → contested → stale → style):

① **Orphan pages** - pages with zero inbound `[[wikilinks]]`:
   ```python
   # execute_code
   import os, re
   from collections import defaultdict
   wiki = os.environ.get("WIKI_PATH", os.path.expanduser("~/wiki"))
   pages = {}
   inbound = defaultdict(int)
   for root, _, files in os.walk(wiki):
       for f in files:
           if f.endswith(".md") and not root.startswith(os.path.join(wiki, "raw")):
               path = os.path.join(root, f)
               content = open(path).read()
               pages[f[:-3]] = path
               for link in re.findall(r'\[\[([^\]]+)\]\]', content):
                   inbound[link.split('|')[0]] += 1
   orphans = [s for s in pages if inbound[s] == 0]
   print(f"Orphans ({len(orphans)}): {orphans}")
   ```

② **Broken wikilinks** - `[[links]]` pointing to non-existent files (extend above script)

③ **Index completeness** - every wiki page must appear in `index.md`; diff filesystem vs index entries via `execute_code`

④ **Frontmatter validation** - all required fields present; tags in taxonomy

⑤ **Stale content** - pages with `updated` >90 days older than most recent source mentioning same entity

⑥ **Contradictions** - surface all pages with `contested: true` or `contradictions:` frontmatter

⑦ **Quality signals** - list `confidence: low` pages and single-source pages lacking a confidence field

⑧ **Source drift** - recompute sha256 for each `raw/` file via `execute_code`; flag mismatches (raw/ should be immutable)

⑨ **Page size** - flag pages >200 lines as split candidates

⑩ **Tag audit** - list tags in use, flag any absent from SCHEMA.md taxonomy

⑪ **Log rotation** - if `log.md` exceeds 500 entries, rotate: rename to `log-YYYY.md`, start fresh

⑫ Append: `## [YYYY-MM-DD] lint | N issues found`

## Searching

```bash
# Content search - use file_search or execute_code glob+grep
# By tag - execute_code: scan .md frontmatter for tag field
# Recent activity - file_read "$WIKI/log.md"
```

## Bulk Ingest

1. Read all sources first
2. Identify all entities/concepts across all sources
3. Check existing pages for all in one pass
4. Create/update pages in one pass
5. Update `index.md` once at the end
6. Write a single log entry for the batch

## Archiving

1. `file_write "$WIKI/_archive/<type>/old-page.md"` (preserve path structure)
2. `file_edit "$WIKI/index.md"` to remove the entry
3. Update linking pages: replace `[[wikilink]]` with plain text + "(archived)" via `file_edit`
4. Log the action

## Obsidian Integration

The wiki directory is an Obsidian vault out of the box: `[[wikilinks]]`, Graph View, YAML frontmatter for Dataview. Set attachment folder to `raw/assets/`. If using the obsidian skill, set `OBSIDIAN_VAULT_PATH` to the same path as the wiki.

## Pitfalls

- **Never modify `raw/`** - sources are immutable; corrections go in wiki pages
- **Always orient first** - SCHEMA + index + log tail before any operation; skipping causes duplicates
- **Always update `index.md` and `log.md`** - these are the navigational backbone
- **Don't create pages for passing mentions** - follow Page Thresholds
- **Don't create pages without cross-references** - min 2 outbound wikilinks
- **Frontmatter is required** - it powers search, filtering, and staleness detection
- **Tags from taxonomy only** - freeform tags decay into noise; add new tags to SCHEMA.md first
- **Split pages >200 lines** - keep pages scannable
- **Ask before mass-updating** - if an ingest would touch 10+ existing pages, confirm scope first
- **Handle contradictions explicitly** - don't silently overwrite; note both claims with dates, mark frontmatter

## Related Tools

[llm-wiki-compiler](https://github.com/atomicmemory/llm-wiki-compiler) - Node.js CLI that batch-compiles a source directory into a concept wiki (Karpathy-inspired, Obsidian-compatible). Use for scheduled/CLI batch pipelines; use this skill for agent-in-the-loop curation.
