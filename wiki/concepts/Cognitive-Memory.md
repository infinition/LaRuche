# Cognitive Memory

LaRuche's memory is not a vector store bolted onto a chat log. It is a cognitive map:
named nodes (topics) holding items (facts), stored in SQLite, recalled by hybrid search,
curated over time, and versioned in git.

## Recall

Search fuses two signals:

- **Semantic**: embeddings (default `nomic-embed-text` via Ollama) for meaning-level
  matches.
- **Full-text**: SQLite FTS5 for exact terms, names, paths, error strings.

The working set assembled before each agent run pulls the most relevant facts into
context automatically. Tools also expose explicit recall to the model.

## What keeps the map clean

Memory that only grows becomes noise. Several mechanisms push back:

- **Importance decay**: facts lose weight over time unless something reinforces them.
- **Write-time supersede**: writing a fact that contradicts or updates an existing one
  replaces it instead of piling up next to it. The old fact is superseded, not silently
  destroyed.
- **LLM arbiter**: contradictions in the ambiguity band (similar but not clearly the
  same fact) are settled by a small model call, never destructively.
- **Dream pass**: a periodic consolidation job that surfaces duplicates and merge
  candidates as proposals you approve or reject. It never deletes on its own.

## Hebbian ranking

"Neurons that fire together wire together", applied honestly:

- **Level 1**: recalled facts gain a small amount of weight (access count).
- **Level 2**: the working-set recall is trace-free; after the answer is produced, only
  the facts whose content actually shows up in the answer get reinforced. Mere
  co-occurrence in the context no longer inflates a fact's rank. Recalled noise stays
  noise.

## Git time travel

On a schedule (`LARUCHE_OKF_GIT_SECS`, default 30 minutes, `0` disables), the whole map
is exported as plain markdown (the OKF format) into `memoire-okf/`, a dedicated git
repository with its own history, independent from the code repo. Snapshots that change
nothing produce no commit.

What this buys you:

```bash
cd memoire-okf
git log --oneline          # the agent's learning timeline
git diff HEAD~5 --stat     # what it learned in the last 5 snapshots
git show <sha> -- projets/laruche.md   # a fact's history
```

Rollback is a checkout plus re-import: restore a file from an earlier snapshot and
import it back. Surgical, auditable, and it uses tools you already know.

## Hot-editable system prompts

The agent's own system prompts live in memory as `system.*` entries (identity, behavior,
planning, curator, consolidation). Edit them in the Memory tab and the change applies to
the next run. Restore-to-default is one click.

## Inspecting everything

`memoire.db` is a normal SQLite file. The Memory tab in the dashboard offers full CRUD
with previews of proposed entries. Nothing about what the agent knows is opaque.
