---
type: skill
name: cron_manager
description: >-
  Create, update, delete, list, and run cron tasks.
---

# cron_manager

Manage LaRuche scheduled tasks via `cron_create`, `cron_delete`, and `cron_list`.

## Procedures

### add_cron(name, cron_expr, prompt)
Create a new scheduled task.
```
cron_create(name=<name>, cron=<cron_expr>, prompt=<prompt>)
```
- `cron_expr` uses standard 5-field cron syntax: `MIN HOUR DOM MON DOW` (e.g. `0 8 * * 1-5` = weekdays at 08:00).
- Fail fast: if a cron with the same name already exists (check via `cron_list`), abort and inform the user - do not create a duplicate.

### update_cron(name, cron_expr, prompt)
There is no native update - replace the task:
1. `cron_list()` → find the entry matching `name`, extract its `id`.
2. If not found, abort with "cron '<name>' not found".
3. `cron_delete(id=<id>)`
4. `cron_create(name=<name>, cron=<cron_expr>, prompt=<prompt>)`

### delete_cron(name)
1. `cron_list()` → find entry matching `name`, extract `id`.
2. If not found, abort with "cron '<name>' not found".
3. `cron_delete(id=<id>)`

### list_summary()
```
cron_list()
```
Format the result as a table: `NAME | SCHEDULE | NEXT RUN | PROMPT (truncated to 60 chars)`.

### run_now(name)
Execute a scheduled task immediately without waiting for its next trigger:
1. `cron_list()` → find entry matching `name`, extract its `prompt`.
2. If not found, abort with "cron '<name>' not found".
3. Execute the `prompt` directly as a new agent instruction (treat it as an inline task).

## Pitfalls
- `cron_list()` may return entries with different id/name field names depending on version - always inspect the raw response before extracting fields.
- Never call `cron_delete` without confirming the `id` - deleting by position is unsafe if the list order changes.
- Cron expressions are UTC unless the LaRuche instance is configured otherwise; warn the user when scheduling time-sensitive tasks.
