# Tool coverage

Every registered tool is either documented in a skill, or listed below as deliberately
skill-less. Nothing is left unaccounted for by accident.

A skill is a PROCEDURE: several steps, an order that matters, traps that cost an hour.
A tool that takes one call and whose schema already says everything does not get one.
Each description in this directory is injected into the system prompt on every turn, so
a skill written only to fill a checklist costs tokens forever and teaches nothing.

## Check it yourself

```bash
python scripts/check_tool_coverage.py
```

It reads every `fn nom(&self) -> &str` in `laruche-essaim/src/abeilles/` and
`laruche-node/src/`, cross-references the skill bodies, and exits non-zero on any tool
that is neither covered nor declared below.

## Covered by family

| Family | Skill |
|---|---|
| `memory_*`, `knowledge_*` | `cognitive-memory` |
| `skill_*` | `skill-forge` |
| `plugin_*`, `mcp_*`, `tool_search`, `tool_call`, `run_script`, `reload_plugins` | `extend-toolset` |
| `delegate`, `spawn_specialist`, `mixture_of_agents`, `submit_job`, `check_job_status` | `delegation` |
| `todo`, `plan_mode`, `kanban_*`, `mission_*`, `task_complete` | `long-running-work` |
| `git_*` | `local-git` |
| `watcher_*`, `file_watch` | `watcher-design` |
| `lsp` | `codebase-inspection` |
| `web_*`, `read_extract`, `browser_*`, `image_search` | `web-research` |
| `research_mode`, `session_search` | `deep_research_synthesis` |
| `file_*`, `shell_exec`, `execute_code` | the procedure skills that use them |

## Deliberately without a skill

These are single calls. Their schema description is the documentation, and wrapping them
in a procedure would add a page to read and nothing to know.

| Tool | Why not |
|---|---|
| `system_info` | Returns OS, hostname, working directory and time. One call, no arguments, no failure mode. |
| `math_eval` | Evaluates an expression. Nothing surrounds it. |
| `clarify` | Asks the user a question. The judgement is WHEN to ask, which belongs in the agent's charter, not in a procedure. |
| `calendar_add` | `title` and `date` as `YYYY-MM-DD`, optional `time` as `HH:MM`. The schema carries the formats. |
| `calendar_list` | Optional date filter. Read-only. |
| `media_present` | Displays a file below the response. Accepts http(s) URLs and local paths inside the working directory. |
| `mesh_send` | Sends a message to another LaRuche instance. Outbound, requires approval, so the user is the gate rather than a written procedure. |
| `finding` | Records a finding during a mission. One call, one payload. |

If one of these grows arguments, modes or a real failure mode, it stops belonging here
and gets a skill.

## Adding a tool

A new tool must, in the same change, either be added to a skill or be listed above with
a reason. `check_tool_coverage.py` fails the build otherwise. That is the point: coverage
that is not enforced decays within a month.
