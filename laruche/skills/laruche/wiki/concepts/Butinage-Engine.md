# The Butinage Engine

Butinage is LaRuche's agent loop. It turns a request into a sequence of model calls,
tool calls, observations and decisions, then stops with an answer, a clarification or a
reported blocker. The implementation is built to run with cloud models and local models
that may return malformed calls, repeat a failed action or lose track of a long task.

## The ReAct loop

One run follows this cycle:

1. **Assemble context.** LaRuche combines the stable system prompt, the conversation,
   the current plan, relevant memory, the available tool schemas and volatile data such
   as the current time.
2. **Call the model.** The provider receives the messages and only the tools selected for
   this request. A model call has its own timeout and retry policy.
3. **Interpret the turn.** The engine reads native tool calls when the provider supports
   them. A tolerant parser can recover calls emitted as text by weaker models.
4. **Validate every call.** Arguments are checked against the tool's JSON Schema before
   execution. Invalid arguments become an observation the model can correct.
5. **Run the tools.** Robust profiles can execute independent calls from one turn in
   parallel. The fragile profile limits the model to one call at a time.
6. **Observe.** Results, errors and safety decisions are returned to the next model pass.
   Large observations keep their beginning and end while the middle is elided.
7. **Choose a bearing.** The Boussole decides whether to continue, land, ask for missing
   information or accept the collected result.
8. **Checkpoint or finish.** Continuing runs persist their notebook. Successful runs
   remove it, reinforce memory that was actually used and store a concise episode.

This is ReAct in practical form: reason from the current state, act through a tool,
observe the result, then reassess. A text answer does not automatically mean the loop is
complete. Completion depends on structured state such as tool calls, stop reason, plan
progress, exploration requirements and remaining retries.

## What is sent on each pass

The system prompt is kept byte-stable so providers can reuse their prefix cache. Data
that changes during a run stays in a volatile tail. Providers with strict local chat
templates cannot accept a trailing system message, so LaRuche merges that tail into the
last user turn instead.

The active tool list is also bounded. Dynamic selection matches the request to the tool
registry and injects only relevant schemas. On model contexts of 40,000 tokens or less,
dynamic selection is forced to avoid spending most of the window on tool definitions.
This changes what the model can see, not what tools are installed.

The main chat receives a budgeted working set from cognitive memory before the run.
Scouts and resumed runs can perform their own initial recall so they do not start blind.

## Calls, validation and observations

Native provider calls are preferred. Text fallback exists for models that print a tool
name and JSON instead of using the provider protocol. Recovery is intentionally bounded:
malformed output can trigger a corrective pass, but it cannot cause unlimited retries.

Before a tool runs, the engine checks:

- that the tool exists in the active registry;
- that its arguments match the declared schema;
- that permission and approval rules allow the call;
- that the Vigie has not blocked an exact repeated dead end;
- that the run has not been cancelled.

The default tool timeout is 300 seconds and the default single model-call timeout is 600
seconds. Individual tools may override the tool limit. Observation size follows the
model context: roughly one quarter of the declared window, clamped between 24,000 and
400,000 characters. Truncation preserves the head and tail, which usually contain the
request context and the final error or conclusion.

## The Boussole

The Boussole is the single decision point after a pass. It returns one of four outcomes:

| Decision | Effect |
|---|---|
| `Poser` | Land and return the answer or a clean stop reason |
| `Relancer` | Add one internal corrective instruction and run another pass |
| `Recolter` | Accept a structured mission result |
| `Clarifier` | Ask the user for information that is required to continue |

It operates on facts recorded by the engine, not on keyword guesses in the answer. It
knows whether tools were called, whether a result was truncated, whether exploration is
complete, whether delegation is available and how many sterile relaunches remain.

The default sterile relaunch limit is three. Normal assistant text is allowed to finish
a standard turn. The engine does not force every response into another pass just because
the global pass ceiling has not been reached.

## The Vigie

The Vigie watches tool activity for three concrete loop patterns:

- the same tool with the same arguments fails repeatedly;
- one tool keeps failing even when its arguments change;
- an idempotent read repeats with the same result and makes no progress.

It first warns the model. Under strict and robust profiles it can then block the exact
call or stop a run in which the same tool keeps failing. A successful call clears the
relevant failure counters. Non-idempotent writes are not classified as stagnant merely
because their output text looks alike.

The counters are stored in the run notebook. Restarting a crashed run does not reset its
loop history. The Native Tools profile uses warnings without hard Vigie stops because it
trusts strong provider-native tool handling.

The Vigie is separate from LaReine. The Vigie detects mechanical tool loops inside the
engine. LaReine reviews quality, policy and plan progress at the supervision layer.

## Plans and progress

A model can create or update an itinerary. The engine persists its steps, measures
progress between passes and exposes updates to the live activity view. The plan is state,
not decorative prose: it supports resume, Tier 3 LaReine supervision and stagnation
detection.

The model can promote a standard run to exploration mode through `research_mode` or a
deep-research plan. That promotion is one-way for the current run, so the model cannot
drop the exploration requirements to finish early.

## Deep research

Exploration mode adds a stricter protocol:

- split the question into at most three or four independent angles;
- delegate up to four scouts in parallel when delegation is available;
- use a guardian pass to cross-check the collected reports;
- verify unstable claims with current web sources;
- keep a findings ledger and cover a minimum search effort;
- perform one final self-check before landing.

The parent run expects 12 weighted web actions by default. A delegated scout counts as
three because it performs its own focused searches. Individual scouts use a smaller
minimum of three web calls. Sub-agents cannot delegate again, which prevents recursive
fan-out.

The self-check can bounce a proposed completion once. It asks the model to look for a
missing angle, weak evidence or an unsupported conclusion. The next valid completion
lands, so verification cannot become its own loop.

## Context budget and Escale

The Jauge tracks input and output tokens across the whole run. A run can have an explicit
cumulative token budget, or `0` for no additional budget beyond provider and pass limits.
The default model context is 128,000 tokens and compaction starts according to the
configured threshold, normally 75 percent in the assembled engine.

When the conversation approaches the window, Escale compacts older history while
keeping the 12 most recent messages intact by default. The preferred path asks an
auxiliary model for a dense summary that preserves discoveries, decisions, plan state and
failed approaches. If that call fails or returns nothing usable, an extractive fallback
keeps the run moving without discarding the history.

System prompts, the current user anchor and recent messages remain separate from the
summary. Internal nudges used to correct the model are not shown as user messages and are
not persisted as part of the conversation.

## Retry and stop conditions

The defaults are deliberately finite:

| Limit | Default |
|---|---:|
| Pass ceiling | 100 |
| Sterile corrective relaunches | 3 |
| Rate-limit retries | 3 |
| Transient retries | 3 |
| Model call timeout | 600 seconds |
| Tool call timeout | 300 seconds |
| Run token budget | Unlimited unless configured |

Rate limits and transient provider failures use their own counters. Rejected or blocked
tool calls cannot consume the entire pass ceiling forever. Near a pass or token limit,
the model receives one wind-down instruction so it can summarize the useful result or
state the blocker.

A run can finish because the mission completed, the user must clarify something, the
budget or pass ceiling was reached, a sterile loop was confirmed, the user stopped it or
a fatal provider error occurred. If the last pass only contained tool calls, the engine
can still return the last non-empty assistant text instead of producing a blank answer.

## Cancellation, steering and resume

The Stop control is cooperative and checked while waiting for the model, between passes
and around tool execution. A steering message is queued into the next pass boundary, so
the user can redirect a live run without throwing away completed work.

After each continuing pass, the engine writes a notebook containing history, plan,
usage, mission mode and Vigie counters. A page reload can reattach to the live job. After
a process interruption, a compatible notebook can resume the run with memory recall and
the exploration protocol restored when needed. The notebook is deleted after a clean
success and retained after an incomplete stop for inspection or continuation.

## Model profiles

| Profile | Intended target | Behavior |
|---|---|---|
| Fragile | Small local models | One tool per turn, strict Vigie, tolerant text parsing |
| Robust | Capable local or remote models | Parallel tools, standard Vigie thresholds |
| Native Tools | Providers with reliable native calls | Parallel tools, trusted stop reasons, warning-only Vigie |

The engine stays the same across profiles. Only the rails and the amount of trust change.

## After landing

LaRuche performs two memory operations after a successful answer. First, recalled facts
are reinforced only when the final answer actually used them. Second, a concise episode
can be stored so later runs know what was attempted, what worked and which dead ends to
avoid. These steps happen after completion and do not alter the answer already returned.

LaReine may also review the result, request rework or record a scorecard, depending on
its configured mode. See [LaReine](LaReine) for the exact supervision semantics.

## Measuring a change to this engine

Everything on this page is a knob someone will eventually want to turn, and a prompt or
a rail is the kind of change that always looks like an improvement to the person who
made it. `laruche-evals` replays fixed missions against this engine, with the real
provider and the real tools, and judges each run mechanically against a saved baseline.

See [Evals](Evals) for what it checks and how to read a diff.
