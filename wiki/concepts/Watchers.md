# Watchers

Watchers are LaRuche's event reflexes: standing conditions that trigger actions when
something happens on your machine or on your network. The design goal was ruthless:
watching must cost nothing while nothing is happening.

## Compiled rules, not polling prompts

You describe the watcher in plain language:

> "If it is Tuesday or Thursday and my local site has been down for 10 minutes, send me
> a Telegram message every 20 minutes until it comes back."

The agent (through its `watcher-architecte` skill) compiles that once into a
deterministic predicate tree. At runtime the scheduler evaluates the tree; no model is
involved unless the tree explicitly asks for one.

## The rules DSL

Rules compose with `et` (and), `ou` (or), `non` (not) over these leaves:

| Rule | Fires when |
|---|---|
| `jour_semaine` | The weekday is in the given set |
| `heure_entre` | The current time is inside a window (night-silence, office hours) |
| `plage_date` | The date is inside a range |
| `apparu` / `supprime` / `modifie` | A file or folder appeared, was deleted, changed |
| `contenu_change` | Watched content differs from the last check |
| `contient` | A file or page contains a pattern (log ERROR lines) |
| `taille_depasse_mo` | A file exceeds N megabytes |
| `est_down` / `down_depuis_min` / `retour_en_ligne` | A service is down, down for N minutes, back up |
| `status_http` | An HTTP check returns a given status |
| `llm_check` | An explicit LLM judgment, described in text |

Evaluation returns one of three verdicts: true, false, or "needs LLM". The `llm_check`
leaf is the only path to a model call, and it only runs after the deterministic prefix
has already passed. A watcher with a weekday guard costs zero tokens six days a week by
construction.

## State-aware leaves

The down/up leaves carry memory between evaluations: `down_depuis_min` tracks how long
a target has been failing, `retour_en_ligne` fires exactly once on recovery. Combined
with a repeat interval, "nag me every 20 minutes while it is down, tell me once when it
is back" is a single watcher.

## The UI: the schema is the form

Each watcher card renders the compiled rule tree as a pipeline diagram: condition
bubbles, and/or branches, the action at the end. Collapsed, a card is a one-line
summary. Expanded, every value in the diagram is editable in place. What you see is
exactly what runs.

## Guardrails

Learned the hard way, enforced server-side:

- **Target validation**: file targets must be absolute existing paths, HTTP targets must
  be well-formed URLs. The agent cannot create a watcher on a relative path that will
  never match.
- **Deduplication**: creating a watcher identical to an existing one is rejected, so a
  confused model cannot pile up six copies.
- **Interval floors**: no accidental every-second hammering of your own services.

## Actions

A triggered watcher can notify any connected channel (Telegram, web feed), or hand the
event to the agent for a full agentic reaction ("when the report lands, summarize it
and file the summary in memory").
