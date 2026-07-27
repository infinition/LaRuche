---
type: skill
name: watcher-design
description: >-
  Turn a monitoring wish in plain language into a deterministic watcher rule tree.
---

# Watcher architect

The user says when they want to be told something. You turn that sentence into a rule
tree, once. The watcher then evaluates it mechanically at every poll, for free, forever.

Use this whenever someone asks to be warned, notified, or to have something run WHEN a
condition becomes true: a file appears or vanishes, a service goes down or comes back, a
page changes, a log line matches, on certain days, within certain hours, after a certain
delay.

Never express days, hours, durations or patterns in the legacy `condition` text field.
That field wakes a model on every single event. The `regles` tree costs nothing.

## The three traps, first, because they are what goes wrong

**1. The op must match `watcher_type`, or it is false at every poll.**

| watcher_type | What it observes | Ops it can satisfy |
|---|---|---|
| `log` | new lines appended to a growing file | `contient`, `contenu_change` |
| `file` | the lifecycle of a path. Carries NO text. | `apparu`, `supprime`, `modifie`, `taille_depasse_mo` |
| `url` | reachability and page text | `est_down`, `down_depuis_min`, `retour_en_ligne`, `status_http`, `contient`, `contenu_change` |

"Tell me when a line of app.log contains ERROR" is a **log** watcher. Written as `file`,
`contient` reads an observation that carries no text, so it is false forever and the
watcher never fires while reporting itself as active.

**2. `heure_entre` bounds are STRINGS and the end is EXCLUSIVE.**
`a: "23:00"` stops at 22:59. To cover 23:55, write `a: "23:56"`. `"22:00"` is the
canonical form; a bare `"22"` and `"8h30"` are accepted too.

**3. `apparu` means "the file appeared", not "a new line appeared".**
On a log watcher, new lines are what `contient` reads. Combining `apparu` with a log
watcher yields a rule that can never be true.

## The tree

One JSON object tagged by `op`, nesting through `regles` arrays.

Combinators: `et {regles: [...]}`, `ou {regles: [...]}`, `non {regle: {...}}`.
The child array is named `regles`. Not `clauses`, not `conditions`.

Deterministic leaves, evaluated at every poll at zero cost:

| op | arguments | true when |
|---|---|---|
| `jour_semaine` | `jours: ["mar","jeu"]` (fr or en, short or full) | today is one of them |
| `heure_entre` | `de: "08:00", a: "22:00"` (overnight windows work) | local time is inside, end excluded |
| `plage_date` | `du: "2026-07-01", au: "2026-07-31"` | local date is inside, both included |
| `apparu` / `supprime` / `modifie` | none | the watched FILE appeared, was deleted, changed |
| `contenu_change` | none | the page text or the log gained content |
| `est_down` | none | the URL is unreachable or answers 5xx |
| `down_depuis_min` | `minutes: 10` | it has been down for at least that long |
| `retour_en_ligne` | none | the URL just came back |
| `contient` | `motif: "ERROR"` | fresh content contains it, case-insensitive |
| `taille_depasse_mo` | `mo: 100` | the watched file reached that size |
| `status_http` | `codes: [500, 503]` | the last status is one of them |

One semantic leaf, the only one that costs a model call, and only after the
deterministic part already passed:

| `llm_check` | `question: "..."` | the gate answers yes |

Put it LAST inside an `et`, so the free leaves filter first.

## Firing

A STATE leaf (`est_down`, `down_depuis_min`, `contient` on a page) stays true while the
situation lasts, so the watcher re-fires every `cooldown_secs` on its own. An EVENT leaf
(`apparu`, `retour_en_ligne`) is true only on the poll where it happens: one fire.

Set `interval_secs` to how fast the user needs to KNOW. Set `cooldown_secs` to how often
they want to be REMINDED.

## Procedure

1. Run `watcher_list`. If one already covers this target, update or delete it rather
   than create a twin. The tool refuses duplicates on the same target.
2. Determine `watcher_type` from what is actually observed, using the table above. Get
   this wrong and nothing else matters.
3. Write the tree. Every mechanical condition goes into a deterministic leaf.
4. Use an ABSOLUTE path for a file or log target. Relative paths resolve against the
   server's working directory, not the user's folder, and the tool refuses them. A
   folder is refused too: it would fire on every unrelated change inside it.
5. Call `watcher_create`. If it answers "Rule accepted by the parser but it can never
   fire", read the reason: the window is unreadable or empty, a pattern is blank, a
   threshold is zero. Fix and retry.
6. Restate the compiled tree to the user in one readable line, so they can confirm the
   logic you inferred.

## Removing one, and checking a file by hand

`watcher_delete` with `id` removes a watcher. Get the id from `watcher_list`; there is no
deletion by name or by target. Run `watcher_list` again afterwards to confirm it is gone,
because a watcher that survives keeps firing long after everyone forgot it exists.

`file_watch` with `path` and `since` is NOT a watcher. It is a one-shot question: was
this file modified after that timestamp? It answers now and monitors nothing. Use it to
check a single file inside a task. When the user wants to be TOLD when something happens,
they want a watcher.

## Examples

Warn me when `ala.txt` appears on the desktop, or disappears.

```json
{"watcher_type":"file","target":"C:\\Users\\me\\Desktop\\ala.txt",
 "regles":{"op":"ou","regles":[{"op":"apparu"},{"op":"supprime"}]}}
```

Ping me on Telegram if a line of release.log contains ERROR, but not at night.

```json
{"watcher_type":"log","target":"C:\\logs\\release.log",
 "regles":{"op":"et","regles":[
   {"op":"contient","motif":"ERROR"},
   {"op":"heure_entre","de":"08:00","a":"23:56"}]}}
```

On Tuesday or Thursday, if the local site has been down 10 minutes, remind me every 20.

```json
{"watcher_type":"url","target":"http://127.0.0.1:8080","interval_secs":60,
 "cooldown_secs":1200,
 "regles":{"op":"et","regles":[
   {"op":"jour_semaine","jours":["mar","jeu"]},
   {"op":"down_depuis_min","minutes":10}]}}
```

On Tuesdays, if the changelog mentions a security fix. The model is consulted only on
Tuesdays, and only when the page actually changed.

```json
{"watcher_type":"url","target":"https://example.com/changelog",
 "regles":{"op":"et","regles":[
   {"op":"jour_semaine","jours":["mar"]},
   {"op":"contenu_change"},
   {"op":"llm_check","question":"the new changelog content mentions a security vulnerability or fix"}]}}
```

## Failure modes

**Created, shown as active, never fires.** Almost always trap 1 or trap 2. Check the op
against `watcher_type`, then check the hour window: an unreadable one makes the whole
tree false at every hour.

**"missing field `op`" or "missing field `regles`".** The tree is not tagged, or the
child array is named `clauses`. Every node carries `op`; children live under `regles`.

**Fires far too often.** A state leaf with no cooldown. Set `cooldown_secs`.

**Fires once and never again** when it should repeat. An event leaf where a state leaf
was meant: `apparu` instead of `contient`, `retour_en_ligne` instead of `est_down`.
