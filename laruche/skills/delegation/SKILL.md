---
type: skill
name: delegation
description: >-
  Hand a sub-task to another agent, a specialist model, or a background job, and collect it.
---

# Delegating

Four ways to offload work. They differ in who does it, whether you wait, and what comes
back. Picking wrong wastes a model call or loses the result.

| Tool | Runs | You get back | Use when |
|---|---|---|---|
| `delegate` | now, same swarm | the finished result | a self-contained sub-task |
| `spawn_specialist` | now, chosen model | the finished result | the sub-task needs a different model or role |
| `mixture_of_agents` | now, several in parallel | one merged answer | quality matters more than cost |
| `submit_job` | in the background | a job id, to poll later | it takes minutes and blocks nothing |

Before any of them, ask whether you can just do it. Delegation costs a full context
build. For anything you can finish in two or three tool calls, doing it yourself is
faster and more reliable.

## delegate

`delegate` with `task`, plus optional `context` and `role`.

The delegate starts COLD. It does not see this conversation. Anything it needs must be in
`task` and `context`: absolute paths, the exact goal, what "done" looks like, and what it
must not touch. A vague `task` returns vague work, and you will not be able to tell that
it misunderstood.

Write `task` as a complete brief a stranger could execute. Include the acceptance
criterion explicitly.

## spawn_specialist

`spawn_specialist` with `role` and `task`, plus optional `context`, `provider` and
`model`.

Same cold start, same briefing rules. Use it when the sub-task wants a specific profile:
a stronger model for a hard piece of reasoning, a cheaper one for bulk extraction, a
local one for something that must not leave the machine.

Leave `provider` and `model` unset unless you have a reason. The default is configured
and works; naming a model that is not available fails the whole call.

## mixture_of_agents

`mixture_of_agents` with `prompt` and `candidates`, plus optional `model`, `provider` and
`api_base`.

Several candidates answer the same prompt, and the answers are merged. Use it for
questions where a single pass is unreliable: an open judgement, a design trade-off, a
summary that must not miss anything.

Do not use it for anything deterministic. Running a command, reading a file or computing
a value has one right answer; asking three models to vote on it costs several times more
and can produce a merged answer that none of them actually gave.

## submit_job

`submit_job` with `script`, plus optional `label`.

Runs in the background. Returns a job id immediately.

1. `submit_job` with the script. Give it a `label`, because a bare id is unreadable in a
   later turn.
2. Keep working. That is the point.
3. `check_job_status` with `job_id` when you need the result.

**Never assume a job succeeded.** Poll with `check_job_status` and read the output before
reporting anything about it. Reporting a background job as done without checking is
fabrication.

If it is still running when you are otherwise finished, say so explicitly instead of
claiming completion.

## Traps

- **The delegate cannot see this conversation.** The single most common failure. Every
  path, every constraint, every already-established fact must be restated in `task` or
  `context`.
- **Do not delegate the whole user request.** Then you are a relay, you cannot verify
  anything, and the result arrives with no one having checked it.
- **Delegating something ambiguous multiplies the ambiguity.** Resolve it first, or ask
  the user, then delegate a clear task.
- **Verify what comes back.** A delegate reporting success is a claim, not a fact. If it
  says it wrote a file, read the file.
- **A job id is not a result.** Nothing has happened until `check_job_status` says so.

## Failure modes

**The delegate returns work that misses the point.** The brief was thin. Do not re-run it
identically: rewrite `task` with the missing constraints spelled out, then retry.

**The delegate says it cannot find a file that exists.** A relative path was passed. It
does not share your working directory. Pass absolute paths.

**`check_job_status` reports failure with no detail.** Run the same script directly with
`shell_exec` to see the real error. Background output is often truncated.

**`mixture_of_agents` returns a bland merge.** The prompt asked for something factual. Do
it in one pass instead.

**`spawn_specialist` fails immediately.** The named `provider` or `model` is not
configured. Drop both fields and retry with the default.
