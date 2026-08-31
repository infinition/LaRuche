# Table Ronde

The table ronde is LaRuche's structured multi-agent deliberation mode. It is available
next to normal chat and is meant for questions where competing analyses are more useful
than one fast answer.

Its main output is not a consensus score. It is a final answer together with the
positions that still disagree, who holds them and why.

## Four mission types

The mission determines the deliverable and the exact tool whitelist available to every
specialist.

| Mission | Deliverable | Access |
|---|---|---|
| Answer | An answer and its disagreements | Web and cognitive-memory reading |
| Code | A plan, code and written files | Research, local file reading and writing, git status and diff |
| Research | A report and its sources | Deep web research, images and local file reading |
| Experiment | A reproducible result from executed code | Research, file writing, Python, scripts and shell commands |

The whitelist grows by level:

- Every mission: `web_search`, `web_fetch`, `read_extract`, `memory_search`,
  `memory_read_node`, `memory_tree`, `math_eval`.
- Research, Code and Experiment: `web_deep_search`, `image_search`, `file_read`,
  `file_list`, `file_search`.
- Code and Experiment: `file_write`, `file_edit`, `git_status`, `git_diff`.
- Experiment only: `execute_code`, `run_script`, `shell_exec`.

No mission opens destructive memory, plugin, skill, watcher or scheduler tools. It also
cannot message another channel, spawn more agents or send mesh messages. A specialist
can make at most three tool calls per intervention. Each tool result is limited to
4,000 characters before it returns to the specialist so one large file cannot consume
the context of every later round.

Code and Experiment can change the machine. In the current interface, selecting one of
those missions and sending the request starts the deliberation without a second
mission-level confirmation dialog. Check the selected mission before submitting.

## The specialist pool

LaRuche ships these reasoning strategies:

| Specialist | Role |
|---|---|
| Orchestrator | Represents team selection and cost control. It does not debate. |
| Scientist | Checks claims, evidence, uncertainty and testability. |
| Systems engineer | Tests scale, latency, cost, failure modes and shared state. |
| Attacker | Looks for concrete abuse paths and invalid trust assumptions. |
| Contrarian | Attacks positions and states what evidence would change its mind. |
| Visionary | Explores alternatives beyond current constraints and marks feasibility. |
| Optimizer | Removes cost, dependencies and complexity while naming what is lost. |
| Arbiter | Compares, decides and writes the final answer without adding new ideas. |

**Manage the team** can hire or reserve specialists, change their name, avatar, role,
reasoning strategy and provider profile, and add custom specialists. Changes are stored
in `specialistes.json` inside the hive data directory. A custom entry with the same id
replaces the shipped definition and can later be removed to return to the built-in
version.

The provider profile is important. Several strategies running on the same base model
still share that model's blind spots. Assigning different provider profiles is what
creates real model diversity. If a selected profile no longer exists, the specialist
falls back to the active profile so the debate can continue.

### Current team selection behavior

The current web flow sends every hired contributor and contrarian to the table. The
orchestrator is displayed as a structural role but is not called to choose a smaller
team dynamically. If no eligible specialist is hired, the mission's default team is
used as a fallback. The arbiter is always outside the participant count and speaks only
for the final synthesis.

## The constitution

Every specialist receives the same constitution before its own strategy. It requires:

1. no invented facts;
2. explicit assumptions;
3. a distinction between fact, hypothesis and opinion;
4. a declared confidence level;
5. named unknowns;
6. an explanation whenever a position changes;
7. criticism of arguments, never participants;
8. truth before consensus.

The effective constitution can be replaced through the cognitive-memory document
`system.constitution`. An empty replacement falls back to the built-in text.

Each intervention uses a structured header followed by free text:

- agreement: approve, reserve or oppose;
- confidence from 0 to 100;
- what changed the specialist's position;
- what evidence would refute it;
- assumptions and unknowns;
- the actual position and reasoning.

Agreement and confidence are self-reported by the specialist. LaRuche displays them; it
does not infer them from the prose.

## How a deliberation runs

1. Every participant answers independently. They cannot see the other first-round
   answers.
2. Participants read the latest position from each other specialist and revise their
   own position in parallel.
3. Once the debate is established, the contrarian attacks the current positions.
4. Contributors answer the objections directed at them.
5. The arbiter lists remaining disagreements first, then agreements, then writes the
   final synthesis using only material already present in the debate.

Participants in the same stage run in parallel. Later stages see only the latest
position from each specialist, not every earlier draft, which keeps the context bounded.

The normal web request asks for at most three debate rounds. The engine has a hard cap
of four rounds, eight participants and 30,000 accounted tokens for the whole deliberation.
It can stop earlier when positions stop moving. If the token budget is reached, it
still reserves a final call for the arbiter so the work remains usable.

A specialist that fails to answer does not discard the other work. If no one answers a
stage, the run stops instead of retrying indefinitely. A separate 32-stage guard
protects against control-flow regressions.

## What the interface shows

The table receives an NDJSON stream while the run is active. It shows:

- the current stage and the specialists working now;
- interventions as they arrive;
- each specialist's latest agreement, confidence and change-of-position marker;
- the arbiter's final answer;
- remaining dissidents before the complete transcript;
- token use, round count and the stop reason;
- a transcript grouped by round, including refutation conditions.

There is deliberately no consensus percentage. Agreement between models measures
conformity, not correctness, especially when they share the same base model.

## History and storage

Completed deliberations are stored as individual JSON files in `deliberations/` inside
the hive data directory. Each record contains the question, mission, participants,
rounds, token count, interventions, final agreement distribution and dissidents.

The Table ronde sidebar lists previous runs and reopens their full transcript. The API
also supports deleting a saved run. Deliberations are kept out of cognitive memory on
purpose: a debate is an audit trace containing several competing claims, not a fact the
agent has learned.

## When to use it

Use the table ronde for architecture choices, security reviews, research with disputed
evidence, implementation plans and experiments where several failure modes matter.

Normal chat is usually better for a direct factual answer, a small edit or work where
one agent can verify the result cheaply. Every additional specialist adds one model
call per active stage, and later stages carry the other positions in their context.
