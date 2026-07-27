---
type: skill
name: cron_manager
description: Schedule a prompt to run later, once or on a repeating cron expression.
---

# Cron manager

A scheduled task is a PROMPT the agent will run on its own, later, with nobody watching.
Use this when the user says "every morning", "each Monday", "in two hours", "remind me on
the 3rd". Three tools do everything: `cron_create`, `cron_list`, `cron_delete`.

There is no update tool and no run-now tool. Both are compositions, written out below.
The headings that describe them are procedure names, not callable tools: the only names
you may call are the three above.

A cron task fires a prompt on a clock. A watcher fires on a CONDITION becoming true, and
costs nothing while the condition is false. "Every hour, check whether the site is down"
is a watcher, not a cron: see `watcher-design`. A mission is an objective pursued across
iterations, sometimes on a cadence: see `long-running-work`.

## Creating one

`cron_create` requires `name` and `prompt`, plus one way of saying when:

| Argument | Form | Meaning |
|---|---|---|
| `cron_expr` | five fields, `MIN HOUR DOM MON DOW` | repeating |
| `fire_at` | RFC3339, for example `2026-12-31T23:59:00Z` | one shot |

Passing neither returns `Specify either 'cron_expr' or 'fire_at'.` Passing both is not an
error and not a merge: **`fire_at` wins and the cron expression is never evaluated.**

Two further arguments are read but absent from the tool's published schema, so you have
to know they exist:

- `skills`, an array of skill names loaded into the scheduled run's prompt. A task whose
  prompt says "check the feeds" will not find `blogwatcher` on its own; name it here.
- `channel`, where the answer is delivered. Left out, the task inherits the channel it was
  created from, which is usually what the user wants.

Procedure:

1. `cron_list` first. A task with the same intent already existing means the user gets
   told twice, on a schedule, forever. There is no duplicate protection in the tool.
2. `cron_create` with `name`, `prompt`, and one of `cron_expr` or `fire_at`.
3. Read the answer. It ends with `Cron task created with ID <uuid>`. Keep that id: it is
   the only handle for deleting the task.
4. `cron_list` again and confirm the entry is there with the schedule you intended.
5. Tell the user, in local time, when it will next run.

Write `prompt` as a complete instruction for an agent with no conversation behind it. It
runs cold, weeks later. "Do the thing we discussed" schedules a failure.

## The cron expression, exactly as this scheduler reads it

Five fields, separated by spaces: `MIN HOUR DOM MON DOW`. **Any other count never fires,
and nothing reports it.** Six-field expressions with seconds, `@daily`, `@hourly`, and
`MON` or `SUN` day names are all rejected this way, silently.

Each field accepts `*`, an exact number, a comma list (`1,3,5`), a range (`1-5`), or a
step (`*/15`). Names, a range with a step (`1-5/2`), `L` and `#` are not implemented.

Two things differ from the cron you know, and both produce a task that looks correct and
never fires:

- **Day of week is 1 to 7, Monday to Sunday.** `0` is not Sunday here, it matches
  nothing. `0 9 * * 0` never runs. Sunday is `7`.
- **Times are LOCAL to the machine running the node**, not UTC. Do not convert.
  `0 8 * * 1-5` is 08:00 local, Monday to Friday.

`fire_at` is the opposite: it is parsed as RFC3339 and is therefore UTC when it ends in
`Z`. A `fire_at` string RFC3339 cannot parse is silently dropped, so a malformed one with
no `cron_expr` surfaces as `Specify either 'cron_expr' or 'fire_at'.`

```
0 8 * * 1-5        08:00 local, Monday to Friday
*/15 * * * *       every quarter hour
0 9 1 * *          09:00 local on the 1st of every month
30 20 * * 7        20:30 local on Sunday
```

## Listing

`cron_list` takes no arguments and returns a JSON array. Each entry carries `id`, `name`,
`cron_expr`, `fire_at` and `enabled`. **It does not return the prompt.**

It also lists mission cadences, which are not cron tasks. Those entries carry
`"kind": "mission"` and an id shaped `mission:<slug>`. This is deliberate, so that "what
is scheduled?" has one truthful answer. But `cron_delete` cannot remove them: a mission
goes through `mission_delete` with its slug.

Present the result as a table: name, schedule, enabled. Read the raw JSON before
extracting anything; do not assume field names from this document alone.

## Deleting

`cron_delete` with `id`, the UUID from `cron_list`. There is no deletion by name and none
by position: list order is not stable, so deleting "the second one" eventually deletes
something else.

1. `cron_list`, find the entry, read its `id`.
2. Not found: stop and say so. Do not delete the closest match.
3. The id starts with `mission:`: this is a mission. Use `mission_delete` with the part
   after the colon.
4. `cron_delete` with the `id`.
5. `cron_list` again. A schedule that survives a deletion you reported keeps firing long
   after everyone has forgotten it exists.

## Updating one, without an update tool

Replace it, in this order, so a failure cannot leave the user with nothing scheduled:

1. `cron_list`, find the entry, read its `id`.
2. Recover the prompt. `cron_list` does not return it, so it comes from this conversation
   or from the user. Do not guess it.
3. `cron_create` with the new schedule and that prompt.
4. Confirm the new one exists with `cron_list`.
5. `cron_delete` the OLD `id`.

Creating before deleting means a crash between the two leaves a duplicate, which the user
can see and remove. Deleting first means a crash leaves silence, which nobody notices.

## Running one now, without a run-now tool

`cron_list` returns the schedule but NOT the prompt, so there is nothing to replay unless
you already have it.

1. If the task was created in this session, you have the prompt. Execute it directly as
   an instruction.
2. Otherwise ask the user what it was meant to do, or read it from the Cron page of the
   dashboard. Do not invent a plausible prompt and run it: a scheduled task can send
   messages and write files.

## Traps

- **A one-shot task disables itself after firing and stays in the list.** `enabled: false`
  next to a `fire_at` in the past is a task that already ran, not a broken one. Delete it
  if the user wants the list clean.
- **A `fire_at` in the past fires on the next tick**, within a minute, not never.
- **The same task never fires twice inside one minute**, so `* * * * *` gives one run per
  minute whatever the polling rate.
- **The prompt runs with no conversation around it.** Absolute paths, full context, an
  explicit success criterion.
- **A schedule is a standing commitment.** Confirm the cadence with the user before
  creating anything recurring, and say what it will do each time it fires.
- **Never schedule anything that spends money, writes to a third party, or deletes data**
  unless the user asked for exactly that.

## Failure modes

**Created, listed, enabled, never fires.** In order of likelihood: the expression does not
have exactly five fields; day of week was written `0` for Sunday; the time was converted
to UTC when the scheduler reads local; a day name or `@daily` was used. Re-read the
expression against the rules above, then `cron_list` to see what was actually stored.

**`Specify either 'cron_expr' or 'fire_at'.`** Neither was given, or `fire_at` was not
valid RFC3339 and was dropped. RFC3339 wants the `T` and an offset:
`2026-12-31T23:59:00Z`, not `2026-12-31 23:59`.

**`cron_delete` reports the id was not found.** The id came from a mission entry, or from
an earlier listing that has since changed. `cron_list` again and take the current id.

**The user gets the same notification twice.** Two tasks with the same intent, or a cron
task duplicating a mission cadence. `cron_list` shows both; delete each with the tool that
matches its kind.

**It fires but the answer goes nowhere.** The origin channel no longer exists. Recreate
the task with an explicit `channel`.
