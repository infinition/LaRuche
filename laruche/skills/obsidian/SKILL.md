---
type: skill
name: obsidian
title: Obsidian Vault
version: "1.1.0"
license: MIT
description: Read, search, create, and edit Obsidian vault notes.
platforms: [linux, macos, windows]
tools: [file_read, file_write, file_list, file_edit, file_search, shell_exec]
scripts: []
dependencies: []
metadata:
  laruche:
    category: productivity
    tags: [obsidian, notes, markdown, wikilinks]
    homepage: https://obsidian.md
---

# Obsidian Vault

Filesystem-first Obsidian vault operations: read, list, search, create, and edit notes, using LaRuche native tools.

## Vault path

The vault root is resolved from `${OBSIDIAN_VAULT_PATH}` (set this as a LaRuche secret or env var). Fallback: `~/Documents/Obsidian Vault`.

**Critical:** `file_read`, `file_write`, `file_list` do not expand shell variables — always resolve to a concrete absolute path first.

```
shell_exec: echo "${OBSIDIAN_VAULT_PATH}"
```

If empty, check fallback:
```
shell_exec: [ -d "$HOME/Documents/Obsidian Vault" ] && echo "$HOME/Documents/Obsidian Vault"
```

Paths may contain spaces — always quote them in shell commands.

## Read a note

```
file_read: <vault_path>/Notes/MyNote.md
```

## List notes

All `.md` files in the vault:
```
file_list: <vault_path>  # recursive, filter *.md
```

Subfolder only:
```
file_list: <vault_path>/Subfolder
```

## Search

**By filename:**
```
shell_exec: find "<vault_path>" -name "*keyword*.md"
```

**By content (files):**
```
shell_exec: grep -rl "search term" "<vault_path>" --include="*.md"
```

**By content (with line numbers):**
```
shell_exec: grep -rn "search term" "<vault_path>" --include="*.md"
```

Or use `file_search` with a keyword to let LaRuche scan indexed content.

## Create a note

Compose the full markdown content, then write it:
```
file_write: <vault_path>/Notes/NewNote.md
content: |
  # Title

  Body text. [[LinkedNote]]
```

Use `[[Note Name]]` wikilinks to connect related notes (base filename, no `.md` extension). For aliased display: `[[Note Name|Alias]]`.

## Edit a note

For targeted edits (replace a block, insert a section):
```
file_edit: <vault_path>/Notes/MyNote.md
```
Use `file_edit` to avoid overwriting the full file when only a section changes.

For full rewrites:
1. Read current content: `file_read: <vault_path>/Notes/MyNote.md`
2. Compose the updated content.
3. Write back: `file_write: <vault_path>/Notes/MyNote.md`

**Pitfall:** `file_write` overwrites entirely — always read first to avoid data loss.

For simple appends:
```
shell_exec: printf '\n## New Section\n\nContent\n' >> "<vault_path>/Notes/MyNote.md"
```

## Failure handling

- If `file_write` errors on a missing directory, create it first: `shell_exec: mkdir -p "<vault_path>/Subfolder"`
- If the vault path resolves empty, prompt the user to set `OBSIDIAN_VAULT_PATH` in LaRuche secrets.
- Prefer `file_edit` over read+write for incremental changes to reduce the risk of content loss.
