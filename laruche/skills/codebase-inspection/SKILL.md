---
type: skill
name: codebase-inspection
description: >-
  Measure a codebase: lines of code, language mix, comment ratio, and symbol lookups.
prerequisites:
  commands: [pygount]
---

# Codebase inspection

Analyze repositories for lines of code, language breakdown, file counts, and code-vs-comment ratios using `pygount`. Run all commands via `shell_exec`.

## Reading code, not just counting it

Counting tells you the shape of a repository. To understand a specific symbol, use `lsp`,
which asks the real language server instead of guessing from text.

`lsp` requires all four arguments: `operation`, `file`, `line` and `character`. There is
no default position, and an approximate one returns an answer about the wrong symbol.

- `goToDefinition`: where this symbol is actually defined.
- `findReferences`: every call site, which is what you need before changing a signature.
- `hover`: the resolved type and documentation.

Locate the symbol first with `file_search` or `grep`, take the exact line and column from
that result, then call `lsp`. Supported servers are rust-analyzer and
typescript-language-server; on a project in another language, fall back to text search.

## Install

```bash
pip install --break-system-packages pygount 2>/dev/null || pip install pygount
```

## 1. Basic Summary (Start Here)

```bash
cd /path/to/repo && pygount --format=summary \
  --folders-to-skip=".git,node_modules,venv,.venv,__pycache__,.cache,dist,build,.next,.tox,.eggs,vendor,third_party" \
  .
```

**CRITICAL:** Always pass `--folders-to-skip`. Without it, pygount crawls dependency/build directories and can hang for minutes on large repos.

Project-type additions to `--folders-to-skip`:
- **Python:** `.mypy_cache,.tox,.eggs`
- **JS/TS:** `.turbo,coverage`

## 2. Filter by Language

```bash
# Python only
pygount --suffix=py --format=summary .

# Python + YAML
pygount --suffix=py,yaml,yml --format=summary .
```

Use `--suffix` on large monorepos to avoid scanning irrelevant file types.

## 3. File-by-File Output

```bash
# Per-file breakdown (default format, no --format flag)
pygount --folders-to-skip=".git,node_modules,venv" .

# Top 20 files by code lines (Linux/macOS)
pygount --folders-to-skip=".git,node_modules,venv" . | sort -t$'\t' -k1 -nr | head -20
```

On Windows, skip the `sort` pipe - use JSON output and post-process instead (see §4).

## 4. JSON Output (Programmatic / Windows-safe)

```bash
pygount --format=json --folders-to-skip=".git,node_modules,venv" . > loc_report.json
```

Then use `execute_code` (Python) to parse and aggregate, or `file_write` to save and pass downstream. Example aggregation:

```python
import json, collections
data = json.load(open("loc_report.json"))
by_lang = collections.defaultdict(lambda: {"files": 0, "code": 0, "comment": 0})
for entry in data:
    lang = entry.get("language", "__unknown__")
    by_lang[lang]["files"] += 1
    by_lang[lang]["code"] += entry.get("code", 0)
    by_lang[lang]["comment"] += entry.get("documentation", 0)
for lang, stats in sorted(by_lang.items(), key=lambda x: -x[1]["code"])[:10]:
    print(f"{lang}: {stats['code']} code, {stats['comment']} comment ({stats['files']} files)")
```

## 5. Interpreting the Summary Table

| Column   | Meaning                                      |
|----------|----------------------------------------------|
| Language | Detected language                            |
| Files    | File count                                   |
| Code     | Executable/declarative lines                 |
| Comment  | Comment/doc lines                            |
| %        | Share of total                               |

Special pseudo-languages: `__empty__`, `__binary__`, `__generated__`, `__duplicate__`, `__unknown__`.

## Pitfalls

1. **Missing `--folders-to-skip`** - crawls node_modules/venv and hangs. Always include it.
2. **Markdown = 0 code lines** - pygount classifies all Markdown as comments. Expected behavior.
3. **JSON files show low counts** - pygount is conservative; use `wc -l` for raw line counts.
4. **Large monorepos** - target with `--suffix` to avoid scanning everything.
5. **pygount not found** - run the install command above; confirm with `pygount --version`.
