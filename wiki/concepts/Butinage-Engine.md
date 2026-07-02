# The Butinage Engine

Butinage is French for what bees do: foraging, flower to flower. It is LaRuche's name
for the agentic loop, and the engine is the part of the system that took the most
hardening, because it has to work with local models that misformat, repeat themselves,
and run out of context.

## The loop

A ReAct cycle: the model reasons, decides on tool calls, the engine executes them and
feeds results back, until the model produces a final answer or a limit is hit. Parallel
tool calls in a single pass are supported and encouraged in the prompt.

## The rails

Every rail exists because a real failure demanded it:

- **Compass (boussole)**: lightweight planning state that keeps long missions oriented,
  so the model does not forget the goal at step 14.
- **Gauge (jauge)**: token accounting per run. Budgets for the run, for sub-agents, for
  tool outputs. When the context approaches the provider's limit, the gauge triggers
  the escale.
- **Sentinel (vigie)**: anti-loop detection. A model calling the same tool with the same
  arguments again, or oscillating between two states, gets interrupted with a corrective
  nudge instead of burning the budget.
- **Escale**: LLM-driven compaction. The conversation so far is summarized and the run
  continues on the summary. Long missions survive small contexts.
- **Schema validation**: every tool call is validated against the tool's JSON Schema
  client-side, before execution. Invalid arguments come back to the model as a
  corrective message it can fix, not as a crash.
- **Tolerant parsing**: native tool calling when the provider supports it, text-format
  fallbacks when the model emits tool calls as prose or malformed JSON.
- **Per-tool timeouts** and **cooperative cancellation**: a stuck fetch does not stick
  the run, and the stop button actually stops things.

## Live steering

You can talk to a run in progress. A steering message is injected into the loop at the
next pass boundary: "actually, focus on the second option" redirects the mission without
killing it. Runs survive a page refresh; the web UI reattaches to the live run.

## Eclaireuses (scouts)

For research fan-out, the engine spawns parallel sub-agents, each with its own budget
slice and a narrowed mission, each able to recall from memory. Their reports are merged
back into the parent run. This is how "research everything about X" becomes ten focused
reads instead of one shallow one.

## Evals

`laruche-evals` replays fixed missions against the real assembled engine and scores the
outcomes. It exists so that engine changes are measured, not vibed. Baselines live with
the repo; regressions in tool-call success, loop rate, or budget burn show up as numbers.
