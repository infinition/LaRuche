---
type: skill
name: obsidian
description: Read, search, create, and edit Obsidian vault notes.
---

# Obsidian Vault

Filesystem-first Obsidian vault operations: read, list, search, create, and edit notes, using LaRuche native tools.

## Vault path

The vault root is resolved from `${OBSIDIAN_VAULT_PATH}` (set this as a LaRuche secret or env var). Fallback: `~/Documents/Obsidian Vault`.

**Critical:** `file_read`, `file_write`, `file_list` and `file_search` do not expand
shell variables, and a relative path resolves against the server's working directory,
not the vault. Resolve to a concrete ABSOLUTE path first, once, and reuse it.

```
shell_exec: echo "${OBSIDIAN_VAULT_PATH}"
```

If empty, check fallback:
```
shell_exec: [ -d "$HOME/Documents/Obsidian Vault" ] && echo "$HOME/Documents/Obsidian Vault"
```

Paths may contain spaces - always quote them in shell commands.

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

Or use `file_search`, which needs `path` and `pattern` and searches FILENAMES by glob.
Add `content` to search inside the files instead, and `max_depth` to bound the walk:

```
file_search: path=<vault_path>, pattern=*.md, content=search term
```

There is no index behind it; it walks the tree. On a large vault, bound it with
`max_depth` or point `path` at a subfolder.

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

**Pitfall:** `file_write` overwrites entirely - always read first to avoid data loss.

For simple appends:
```
shell_exec: printf '\n## New Section\n\nContent\n' >> "<vault_path>/Notes/MyNote.md"
```

## Traps

- **`file_write` overwrites the whole note.** There is no append mode. For a change to
  part of a note, use `file_edit` with `path`, `old_string` and `new_string`; it fails
  loudly if `old_string` is not unique, which is the protection you want.
- **A wikilink is a filename, not a path.** `[[Note Name]]` resolves by base filename
  anywhere in the vault. Renaming a note breaks every link to it, and nothing warns you.
- **Obsidian keeps its own cache.** A note written on disk while the app is open may not
  appear until it re-scans. Say the file was written; do not claim the user can see it.
- **Never touch `.obsidian/`.** That folder is the vault's configuration and plugin
  state. Writing into it can break the user's setup in ways they will not connect to you.
- **Frontmatter is load-bearing.** Many vaults key dataview queries and templates off
  YAML frontmatter. When rewriting a note, preserve the block between the leading `---`
  markers exactly.

## Failure modes

**`file_write` errors on a missing directory.** Create it first:
`shell_exec` with `mkdir -p "<vault_path>/Subfolder"`. Quote it; vault paths contain
spaces more often than not.

**The vault path resolves empty.** `OBSIDIAN_VAULT_PATH` is not set and the fallback does
not exist. Ask the user for the vault location, and offer to store it as a LaRuche secret
so the next session does not ask again.

**`file_edit` reports the string was not found.** The note on disk differs from what you
remember, usually by a line break or a non-breaking space. `file_read` it again and copy
`old_string` from that output rather than from memory.

**A note was written but the user cannot find it.** It went to an absolute path outside
the vault, or into a subfolder they do not sync. Report the exact path you wrote.
