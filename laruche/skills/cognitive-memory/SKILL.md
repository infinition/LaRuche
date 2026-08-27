---
type: skill
name: cognitive-memory
description: Store, find and curate lasting facts in the cognitive memory map.
---

# Cognitive memory

Everything you learn dies at the end of the turn unless you write it down. This skill
covers the whole memory surface: where to put a fact, how to find one again, and how to
keep the map from rotting into a landfill.

## Two stores, and they are not interchangeable

Getting this wrong is the single most common mistake.

| | `memory_*` | a file on disk |
|---|---|---|
| Holds | facts ABOUT the user and the work: decisions, preferences, project state | reference TEXT you may need to quote: docs, articles, transcripts |
| Shape | a map of nodes, each holding short items | whatever the document already is |
| Write | `memory_write` | `file_write`, then `memory_write` the path and one line saying what it is |
| Read | `memory_search` | `memory_search` finds the note, `file_read` or `read_extract` opens the document |
| Ask yourself | "will I need this to act correctly next week?" | "will I need to look this passage up?" |

"The user prefers Rust and refuses em dashes" is `memory_write`. A 40 page datasheet you
may need to quote is a file, plus one remembered line pointing at it. Never dump a
document into `memory_write`: it poisons every future search with noise no one can prune.

**There is no separate knowledge store.** A flat RAG index existed once, `knowledge_add`
and `knowledge_search`, and it was removed on purpose: it duplicated the cognitive map and
split every recall across two systems that never agreed. Those two tools no longer exist.
The pairing above, a file for the text and a remembered pointer to it, replaces them.

## Node ids

A node id is dotted and snake_case. The parent is created automatically.

```
projects.laruche          decisions.licence_mpl        people.marc
preferences.redaction     infra.serveur_maison
```

Rules that prevent an unusable map:

- **Lowercase, snake_case, dots as the only separator.** `Projects.LaRuche` and
  `projects-laruche` create two more nodes next to the right one.
- **Two levels is usually enough.** `projects.laruche.build.ci.cache` splits one subject
  across five nodes and nothing is ever found again.
- **Reuse before you create.** Call `memory_suggest_nodes` with the subject first. It
  returns existing nodes that fit. Only create when nothing does.

## Procedure: recording a fact

1. `memory_search` with the subject. If the fact is already there and merely outdated,
   this is an update, not a write: go to `memory_update_item` with the returned
   `item_id`. Writing again leaves both versions and the model will later read the stale
   one.
2. `memory_suggest_nodes` with the subject, to pick an existing node.
3. `memory_write` with `node_id` and `content`.
   - One fact per call. "Uses Rust, lives in Lyon, hates em dashes" is three writes,
     because they will be corrected, moved and deleted independently.
   - Write it so it survives without the conversation. "He agreed" is worthless in a
     month. "The user chose MPL-2.0 for LaRuche on 2026-07-26, to keep the core copyleft
     while allowing proprietary integrations" is not.
   - **Absolute dates only.** Never "yesterday", "next week", "in two days". Resolve
     against the current date before writing.
   - `source` is optional and worth filling: where the fact came from.
4. Re-read with `memory_read_node` on the node you just wrote to. Confirm the item is
   there and says what you meant.

## Procedure: orienting yourself before a task

1. `memory_search` with the task subject. This is the default and should be your first
   call on any non-trivial request.
2. If the subject is a known node, `memory_read_node` returns all of it, which is
   cheaper and more complete than repeated searching.
3. `memory_grep` with `pattern` when you need a literal string, an exact path, a token, an
   id. `memory_search` is semantic and can miss an exact match; `memory_grep` cannot.
   Optional `limit` caps the result count.
4. `memory_tree` takes NO arguments. It prints the shape of the whole map, node by node,
   without contents. Use it to find where something should live, not to read. There is no
   way to ask it for one subtree: read the output and pick.

## Procedure: curating the map

Run this when memory feels noisy, or when the user asks you to tidy it.

1. `memory_doctor`, no arguments. Read-only. Reports counters, the heaviest nodes,
   duplicates and overloads. It changes nothing, so it is always safe to call.
2. For each overloaded node it names, `memory_consolidate` on that `node_id`. It merges
   the items into a minimal, lossless summary. Read the node afterwards with
   `memory_read_node` and confirm nothing important was flattened away.
3. For items filed in the wrong place, `memory_create_node` the right node if needed,
   then `memory_move_item` with `item_id` and the destination `node_id`. Always pass
   `reason`; a move with no reason is indistinguishable from a mistake later.
4. `memory_delete` only for a fact that is WRONG or now meaningless. A fact that is
   merely old is history: keep it, and add the current one. Deleting is not tidying.

## Maintaining the nodes themselves

- `memory_update_node` with `node_id`, plus `label`, `one_liner` or `importance`. The
  `one_liner` is what a reader sees when browsing the map, so it must say what the node
  holds, not repeat its id. Fix a node whose contents drifted away from its label instead
  of creating a second one beside it.
- `memory_delete_node` with `node_id`. Destructive: it takes the node and what it holds.
  Read it with `memory_read_node` first, and move anything worth keeping with
  `memory_move_item`. Never delete a node to "clean up" without reading it.
- `memory_stats` takes no arguments and shows volume: how many nodes, how many items,
  where the weight is. Use it to decide whether curation is even needed before running a
  full `memory_doctor`.
- `memory_mutations` shows the recent change history of the whole map: what was written,
  updated or removed, most recent first. Its only argument is `limit`; it cannot be
  filtered to one node, so raise `limit` and read for the node you care about. This is how
  you answer "why does memory say this?" and how you spot a fact that keeps being
  rewritten because two sources disagree.

## Proposed items

Some items land in a pending queue instead of the map, awaiting review.

1. `memory_list_proposed` lists them.
2. `memory_review` with `item_id` and `action` set to `accept` or `reject`.

Do not leave the queue growing. If proposals accumulate, nothing is being learned.

## Traps

- **`memory_write` needs both `node_id` and `content`.** Omitting `node_id` fails; there
  is no default node and inventing `misc` or `general` defeats the whole map.
- **`item_id` is not `node_id`.** `memory_update_item`, `memory_delete`,
  `memory_move_item` and `memory_review` all take an ITEM id, obtained from
  `memory_search` or `memory_read_node`. Passing a node id there fails or, worse, matches
  nothing and looks like a no-op.
- **Searching returns nothing does not mean the fact is absent.** Try `memory_grep` with
  a literal fragment before concluding it was never stored.
- **Do not write secrets.** API keys, tokens, passwords go in the secrets vault, never in
  a memory item. Memory items are read back into the prompt, which means into the
  provider request and into logs.

## Failure modes

**Every search returns the same three irrelevant items.** The map has too few nodes and
everything is piled into them. Run `memory_doctor`, then `memory_consolidate` the heavy
nodes and `memory_create_node` proper ones.

**The same fact appears three times with different wording.** Earlier turns wrote instead
of updating. Keep the most complete item, `memory_delete` the others with a reason, and
from now on search before writing.

**A write reports success but the fact is not found afterwards.** You searched with a
different vocabulary than you wrote. Confirm with `memory_read_node` on the exact
`node_id` you used. If it is there, the write worked and the search term was the problem.

**The user says you forgot something they told you.** It was never written. Write it now,
then check whether the surrounding subject deserves a node of its own.
