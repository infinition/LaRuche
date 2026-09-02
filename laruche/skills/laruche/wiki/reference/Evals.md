# Knowing whether an engine change helped

Prompts and loops are the easiest part of an agent to change and the hardest part to
judge. A new system prompt always looks better on the three examples you happen to try,
because those are the examples you had in mind while writing it. Two weeks later nobody
can say whether the engine got better or whether the person judging it got used to it.

`laruche-evals` exists so that this question has an answer. It replays a fixed set of
missions against the real butinage engine, with the real provider and the real tools,
and judges each run against hard checks. No model rates the output prose. The verdict
is mechanical, and therefore comparable across weeks.

## What it runs

The mission set lives in `laruche/evals/missions.json`. Each entry is a mission
statement plus the expectations that a correct run must satisfy. Missions are
deliberately ordinary: they exercise the loop, not the corner cases. The point is
regression, not benchmarking.

```bash
cargo run -p laruche-evals -- --missions evals/missions.json
```

Useful flags:

| Flag | Effect |
|---|---|
| `--only <substring>` | Run the missions whose id contains this |
| `--repeat N` | Run each mission N times, to see variance rather than one draw |
| `--judge` | Add a model-based judgement on top of the hard checks |
| `--save-baseline` | Record this run as the reference for later comparisons |

The provider comes from the environment: `RUCHE_PROVIDER`, `RUCHE_MODEL`,
`RUCHE_API_KEY`, `RUCHE_API_BASE`, `OLLAMA_URL`, `RUCHE_CONTEXT_MAX`, `RUCHE_AUX_MODEL`.
Point it at whatever you actually ship on.

## What it checks

These are the checks that catch real regressions, in the order they usually fire:

- **Terminal reason.** How the run ended: an answer, a clarification, a reported
  blocker, a sterile loop, an exhausted budget. A change that turns answers into
  budget exhaustion is a regression even if every answer it still produces is good.
- **Mission mode.** The run must stay in the mode it was given. An engine that quietly
  promotes a question into a full mission burns money nobody approved.
- **Web effort.** How much of the run went to the outside world. Both directions matter:
  a research mission that never searched, and a trivial question that searched nine
  times.
- **Scout fan-out.** How many parallel explorations were opened.
- **Pass count.** The number of loop iterations. This is the first number that moves
  when a prompt change makes the model less decisive.
- **Files produced.** For missions whose deliverable is a file, the file must exist and
  be non-empty.
- **Forbidden resignation phrasing.** A run that ends with "I cannot help with that" or
  its many variants fails, even when it terminated cleanly. An agent that gives up
  politely is not an agent that succeeded, and this is exactly the failure that reads
  as success in a transcript.

## Reading the result

The harness prints a markdown table, writes the runs as JSONL, and diffs against the
saved baseline. The diff is the part that matters: a table on its own tells you how
today went, the diff tells you what your change did.

Three readings, and only three:

- **No movement.** Your change is neutral on this mission set. That is a real result,
  and usually the honest one for a prompt tweak.
- **A check flips.** Something got better or worse in a way you can name. Follow the
  check, not the totals.
- **Pass count or web effort drifts without any check flipping.** The cheapest kind of
  regression, and the one that never gets noticed without measurement: same answers,
  more money.

Variance is real. A single run of a single mission proves nothing on a temperature above
zero. Use `--repeat` before concluding anything from a small difference, and re-baseline
deliberately rather than after every green run.

## When to reach for it

Any change to the loop, the system prompts, the tool descriptions, the compaction, the
budgets or the supervision. Those are the places where an improvement and a regression
look identical from the outside, which is the whole reason this harness exists.

Not needed for a UI change, a new tool, or a bug fix with a unit test. Those have
cheaper ways to be verified, and the eval run costs real provider tokens.

The harness is excluded from the default build: `cargo build` at the workspace root does
not compile it. It runs when you ask for it.
