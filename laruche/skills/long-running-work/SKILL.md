---
type: skill
name: long-running-work
description: Track multi-step work with todo, plan mode, kanban and missions.
---

# Long-running work

Four mechanisms, four different lifespans. Using the wrong one is how work gets lost
between turns or how a five minute task gets wrapped in ceremony.

| Mechanism | Lives for | Use when |
|---|---|---|
| `todo` | the current mission | a task has several steps you must not drop |
| `plan_mode` | until approval | the change is large enough that the user must agree first |
| kanban | across sessions | the user tracks work items over days |
| mission | across sessions, repeatedly | an objective needs many iterations, possibly on a schedule |

None of these is the right answer when a CONDITION decides rather than you. "Warn me
when X happens", "if Y is true, do Z": that is a watcher, see `watcher-design`, and its
rules cost nothing while nothing happens. A mission on a schedule that keeps checking
whether something occurred is the expensive way to write a watcher.

## Todo

`todo` takes `action`, plus `text` to add and `id` to target an existing entry.

Use it as soon as a request has more than about three steps. Its job is not decoration:
it is what stops step four being forgotten after a long tool result pushes it out of
view.

- Add each step as its own entry, phrased as an action.
- Mark one in progress, do it, mark it done, then move on. One in progress at a time.
- If you discover a new step mid-task, add it immediately rather than holding it.

A todo list that is written once and never updated is worse than none: it reports
progress that did not happen.

## Plan mode

`plan_mode` with `titre` creates `plan.md` in the session's working directory, holding a
skeleton with three headings: Context, Proposed Steps, Approval Required. You then fill
it in BEFORE changing anything.

Use it when the change is structural: many files, a migration, a deletion, anything the
user would want to veto. Do not use it for a fix you were explicitly asked to make.

1. `plan_mode` with a short descriptive `titre`. The answer names the absolute path it
   wrote; read that path rather than assuming where it landed.
2. Fill in `plan.md` with `file_write` or `file_edit`: what you will change, in what
   order, what could break, what you will verify. Concrete file paths, not intentions.
   For what a good plan document contains, open the `plan` skill.
3. Present it and wait for approval. Do not start while asking.

**An existing `plan.md` is moved aside, not overwritten.** If one is already there it is
renamed `plan-<date>_<time>.md` and the answer names the new path. Nothing is lost, but the
old plan is no longer where you left it: if you meant to continue it rather than start a
fresh one, open it at the path the answer gives instead of calling `plan_mode` again.

## Kanban

The board is global and persists across sessions. It belongs to the user, not to a
mission.

- `kanban_list` takes no arguments and shows every task with its status and its UUID. Read
  it before adding, to avoid twins, and to get the id you will need next.
- `kanban_create` with `title` and optional `description` adds one. It answers with the
  new task's UUID.
- `kanban_next` takes NO arguments. It does not advance anything: it ANSWERS the question
  "what should I do next", returning the first Ready task, or a task whose dependencies
  are all completed. When nothing is actionable it says so.
- `kanban_complete` with `id` and `result` closes a task. Completing a task automatically
  unblocks the tasks that depended on it, which is what makes `kanban_next` useful on the
  following call.

`id` is the UUID printed by `kanban_list` or `kanban_create`. It is validated: a title, a
position or a truncated id is rejected outright.

`result` is required by `kanban_complete` and is not a formality: it is what the user
reads later to know what actually happened. "Done" is useless. Say what changed and how
it was verified.

The normal loop is therefore `kanban_next`, do the work, `kanban_complete` with that
task's id, `kanban_next` again.

## Missions

A mission is an objective pursued across many iterations, optionally on a schedule.

- `mission_list` shows objective, cadence, status, iteration count and last run.
- `mission_create` with `objective`, plus optional `cadence` as a cron expression
  (`0 9 * * *` for every day at 09:00) and optional `channel` for delivery. Omit
  `cadence` for a mission advanced manually.
- `mission_delete` removes one.

Write `objective` so it still makes sense with no conversation around it, and so that
progress is recognisable. "Monitor the Rust ecosystem" never ends and never succeeds.
"Every morning, report new releases of the crates listed in Cargo.toml, with breaking
changes flagged" can be executed and judged.

Check `mission_list` before creating: a duplicate mission means the work runs twice and
the user is notified twice.

## Finishing

`task_complete` with `summary` and `confidence` ends the mission. Call it ONLY when the
work is finished and verified.

- `summary` states what was accomplished, concretely.
- `confidence` must be honest. Lowering it when something is unverified is the correct
  behaviour, not a weakness.
- `artifacts` lists what was produced, when there is anything.

Do not call it for an intermediate step. Do not call it while something is still failing:
report the failure instead. A completion claimed on broken work is the worst possible
outcome, because it stops anyone from looking.

## Traps

- **Kanban is not todo.** Kanban outlives the session and is the user's board. Todo is
  scratch space for the current mission. Filling the user's board with your internal
  steps is noise they have to clean up.
- **`kanban_next` does not advance a task.** It reads. Calling it in a loop without
  calling `kanban_complete` returns the same task forever.
- **`kanban_complete` requires `result`, and `id` must be the UUID.** A call with the
  title instead of the id is rejected.
- **A mission with a cadence runs on its own.** Creating one is a standing commitment,
  not a one-off request. Confirm the cadence with the user before creating it.
- **Plan mode is not a way to avoid deciding.** If the user asked for a specific change,
  make it.

## Failure modes

**Steps get forgotten in long tasks.** No todo list, or one never updated. Add the
remaining steps now and mark honestly what is actually done.

**The same mission fires twice a day.** Two missions with near-identical objectives.
`mission_list`, then `mission_delete` the duplicate.

**A kanban task sits in progress forever.** It was advanced but never completed, or the
work stopped. Close it with a `result` that says what happened, including "abandoned
because X". An honest closure is information; an open card is not.
