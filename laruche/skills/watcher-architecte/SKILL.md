---
type: skill
name: watcher-architecte
version: watcher-architecte-v1
description: Compile a natural-language monitoring wish into a deterministic watcher rules tree (watcher_create with regles) - zero LLM cost at runtime for everything mechanical
tools: watcher_create, watcher_list, watcher_delete
---

# Watcher architect

## When to use it
Whenever the user asks to be warned, notified or to have something done WHEN a
condition becomes true: a file appears or is deleted, a site goes down or comes
back, a page changes, a log line matches, on given days or hours, after a given
duration. You COMPILE the wish once into a `regles` tree; the watcher then runs
mechanically for free. Never put day/hour/duration/pattern logic in the legacy
`condition` text field: that costs an LLM call per event.

## The rules tree (`regles` argument of watcher_create)
JSON object with an `op` tag. Combinators: `et`, `ou` (with `"regles": [...]`),
`non` (with `"regle": {...}`).

Deterministic leaves (free, evaluated at every poll):
| op | args | true when |
|---|---|---|
| `jour_semaine` | `jours: ["mar","jeu"]` (fr or en, full or 3-letter) | today is one of them |
| `heure_entre` | `de: "22:00", a: "06:00"` (overnight ok) | local time in window |
| `plage_date` | `du: "2026-07-01", au: "2026-07-31"` | local date in range |
| `apparu` / `supprime` / `modifie` | - | the watched FILE appeared / was deleted / changed |
| `contenu_change` | - | the page text or the log got new content |
| `est_down` | - | the URL is unreachable or 5xx |
| `down_depuis_min` | `minutes: 10` | down for at least N minutes |
| `retour_en_ligne` | - | the URL just came back up |
| `contient` | `motif: "ERROR"` | fresh content contains it (case-insensitive) |
| `taille_depasse_mo` | `mo: 100` | the watched file is at least N MB |
| `status_http` | `codes: [500, 503]` | last HTTP status is one of them |

Semantic leaf (the ONLY one that costs an LLM call, asked AFTER the
deterministic prefix already passed - short-circuit):
| `llm_check` | `question: "..."` | the LLM gate answers YES |

Re-fire semantics: a STATE leaf (`down_depuis_min`, `est_down`, `contient` on a
page) stays true while the situation lasts, so the watcher re-fires every
`cooldown_secs` automatically. An EVENT leaf (`apparu`, `retour_en_ligne`)
is true only on the poll where it happens: single fire.

## Compilation examples

"previens-moi quand ala.txt apparait sur le bureau"
```json
{"watcher_type":"file","target":"C:\\Users\\infinition\\Desktop\\ala.txt",
 "regles":{"op":"ou","regles":[{"op":"apparu"},{"op":"supprime"}]}}
```
(single fires, zero LLM)

"si on est mardi ou jeudi et que le site local est down depuis 10 minutes,
message telegram toutes les 20 minutes"
```json
{"watcher_type":"url","target":"http://127.0.0.1:8080","interval_secs":60,
 "cooldown_secs":1200,
 "regles":{"op":"et","regles":[
   {"op":"jour_semaine","jours":["mar","jeu"]},
   {"op":"down_depuis_min","minutes":10}]}}
```
(re-fires every 20 min while down, zero LLM)

"le mardi, si le changelog mentionne une faille de securite"
```json
{"watcher_type":"url","target":"https://exemple.com/changelog",
 "regles":{"op":"et","regles":[
   {"op":"jour_semaine","jours":["mar"]},
   {"op":"contenu_change"},
   {"op":"llm_check","question":"the new changelog content mentions a security vulnerability or fix"}]}}
```
(the LLM is consulted only on Tuesdays AND only when the page actually changed)

"alerte si ERROR apparait dans le log, mais pas la nuit"
```json
{"watcher_type":"log","target":"C:\\logs\\app.log",
 "regles":{"op":"et","regles":[
   {"op":"contient","motif":"ERROR"},
   {"op":"non","regle":{"op":"heure_entre","de":"22:00","a":"07:00"}}]}}
```

## Rules of thumb
- BEFORE creating: `watcher_list`. If a watcher already covers the target,
  UPDATE or DELETE it instead of creating a twin (the tool refuses duplicates
  on the same target unless you pass `force: true` with a reason).
- Target the ABSOLUTE FILE path, not its folder, for appear/delete watches.
  The tool refuses relative paths and directories: a relative path resolves
  against the server's working directory, and a folder fires on every
  unrelated change inside it.
- Push everything mechanical (days, hours, durations, patterns, sizes, statuses)
  into deterministic leaves; keep `llm_check` for judgment calls only, and put it
  LAST inside an `et` so the cheap leaves filter first.
- Set `interval_secs` to how fast the user needs to KNOW, `cooldown_secs` to how
  often they want to be REMINDED.
- After creating, restate the compiled tree to the user in one readable line so
  they can confirm the logic.
