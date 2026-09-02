# LaReine

LaReine is LaRuche's supervision layer. It can review a finished answer, send the
worker through a fresh agentic run, gate selected self-modifications and watch a
planned task while it is running.

It sits outside the worker loop. The worker does the task and uses tools. LaReine
evaluates the result and decides whether it can ship, needs another pass or should be
flagged for a person.

LaReine is off by default. Its settings are in **Settings > LaReine**.

## Four operating modes

The mode controls who makes the final supervision decision.

| Mode | Finished answers | Gated proposals |
|---|---|---|
| Off | No automatic review. The crown action can still request a one-off review. | If the queue gate is off, writes apply directly. If it is on, every covered write waits for a person. |
| Autonomous | LaReine approves, requests a rework or reports an explicit escalation on its own. | Safe changes can apply directly. Sensitive changes require enough confidence; uncertain ones remain queued. Critical changes always wait for a person. |
| Hybrid | Same autonomous loop, but any judge confidence strictly below the configured threshold escalates, even when the verdict says approve. | Same risk policy, with low-confidence sensitive changes explicitly escalated to a person. |
| Human in the loop | Every reviewed answer is escalated instead of being rewritten automatically. | Every covered proposal waits for a person. |

For response review, Human in the loop is advisory rather than a blocking publication
gate. The draft is shown with LaReine's scorecard and warning. It does not wait behind
an approve button. The proposal queue is the part that provides a durable approve or
reject gate before a change is applied.

Setting **Max reworks** to `0` disables automatic response review even if the selected
mode is Autonomous, Hybrid or Human in the loop. Finite values are limited to 10.
Unlimited is stored as `255`, but the time and stagnation safeguards still apply.

## Three supervision levels

| Level | Scope | Current behavior |
|---|---|---|
| Tier 1, responses | Chat answers and background mission results | Runs the judge and, when allowed by the mode, the rework loop. |
| Tier 2, artifacts | Memory and selected self-created artifacts | Uses a durable proposal queue with risk-based approval. |
| Tier 3, live supervision | A planned butinage run | Detects a stalled plan, injects corrective nudges and eventually escalates. |

The controls are independent. Tier 1 also requires an active mode and at least one
allowed rework. Tier 3 requires an active mode. Tier 2 has a separate **Queue memory
changes for review** switch, called `queue_gate` in the settings file.

The current Tier 2 switch records the intended artifact level, but the live routing is
driven by `queue_gate`. Enabling Tier 2 without the queue gate does not hold writes.

## What the judge evaluates

Every completed review produces a scorecard with five values from 0 to 100:

| Field | Meaning |
|---|---|
| Relevance | Does the answer address the actual request? |
| Methodology | Was the work performed and verified with appropriate tools? |
| Objective | Is the requested deliverable complete and usable? |
| Brand conformance | Does the answer follow the configured writing and behavior rules? |
| Confidence | How certain is the judge about its own assessment? |

The global quality score is the average of the first four values. Confidence is kept
separate because it controls Hybrid escalation rather than answer quality.

The scorecard also contains a verdict, a reason, an actionable correction instruction
and the judge's analysis. The verdict is one of `approve`, `revise` or `escalate`.

### The charter

The rubric comes from the cognitive-memory document `system.prompt_reine`. If that
document is empty or absent, LaRuche uses the charter compiled from
`skills/lareine-charter/SKILL.md`.

This makes the reviewer policy editable without rebuilding the application. A team can
require sources, tests, a specific answer structure or its own writing rules. The
charter should describe observable criteria. Vague instructions make the scorecard
less stable.

### Evidence available to LaReine

LaReine does not grade only the final prose. For each draft it receives a bounded view
of the work that produced it:

- the current request and draft;
- up to 20 earlier conversation messages, with 4 as the default;
- the opening request when the recent window no longer reaches it;
- the tools that were available to the worker;
- the tool calls made since the last user message;
- useful call arguments such as a query, URL, path, command or task;
- short tool-result extracts, source URLs and failed-call counts;
- the top-level cognitive-memory domains;
- correction instructions already issued during earlier review rounds.

This is what makes the methodology score useful. A draft that says tests passed can be
compared with the actual trace. A research answer can be checked against the searches,
sources and failures recorded during the run.

The evidence view is deliberately capped. Tool evidence gets a shared character budget
and each result is shortened before it enters the judge prompt. This prevents one large
page or command output from consuming every later review round.

## Tier 1: review and rework

The automatic response path runs after the worker has produced an answer:

1. LaReine builds the conversation context and the evidence view.
2. The judge returns a structured scorecard.
3. The mode and confidence threshold select approve, revise or escalate.
4. A revision starts a new worker run with the original request and LaReine's concrete
   instruction.
5. The new answer is judged again until it passes or a safeguard stops the loop.
6. The final answer, verdict and analysis are written back to the session.

A rework is not a paraphrase call. It is a complete agent run with access to the normal
tools. Read-only tools are encouraged when the problem is missing evidence. The rework
brief is scoped to the answer text and explicitly forbids file mutations caused only by
a writing correction.

The same Tier 1 path can review the result of a background mission. It returns the
revised result to the mission instead of treating mission output as a separate,
unreviewed channel.

### Rework safeguards

Several independent limits prevent supervision from becoming an endless loop:

- **Round limit**: 0 to 10 finite reworks, or Unlimited.
- **Wall-clock budget**: `LARUCHE_REINE_BUDGET_SECS`, 600 seconds by default, covers
  the complete review and rework sequence.
- **Stagnation stop**: two consecutive rounds without a better score stop further
  rework.
- **Best-draft retention**: if a later draft scores worse, LaRuche can ship the best
  earlier draft instead.
- **Provider-error bypass**: a short response that is clearly a provider or connection
  error is flagged without spending more calls trying to rewrite it.
- **Failed-rework fallback**: an empty result, a system error or a failed worker run
  leaves the previous answer intact.
- **Best effort judge**: provider failures and invalid scorecard output never break the
  original turn.

Unlimited therefore means no round count limit. It does not disable the time budget,
stagnation detection or failure fallbacks.

## One-off review from the conversation

When automatic supervision is inactive, a crown action is available on assistant
messages.

The first action calls `POST /api/reine/appel`. It is judge-only: LaReine returns the
verdict, scores, correction and analysis without replacing the answer. It works while
the global mode is Off and uses a wider context window of 12 messages.

If the verdict asks for work, **Send LaRuche back to work** calls
`POST /api/reine/renvoyer`. That action is explicit because it spends tokens and can
replace the answer. It temporarily ensures Autonomous mode, Tier 1, at least one
rework and at least 12 context messages for that run. The configured judge profile is
still used.

Both endpoints check that the caller may access the session.

## Judge provider

LaReine can use the worker's active model or a separate provider profile. A distinct
judge is usually more useful because a model tends to repeat or accept its own blind
spots. A stronger remote model can supervise a smaller local worker, while a small
local judge can provide cheap routine checks.

Judge calls use a low temperature of `0.2` and allow up to 2,048 output tokens so the
structured scorecard has room to complete. A separate judge still adds one model call
per review round, and every requested rework adds a full worker run.

## Tier 2: proposal queue

The queue is a review boundary for selected changes to LaRuche's own state. Proposals
are stored in `laruche-reine-queue.json` and remain available after a restart.

Current end-to-end coverage includes:

- curator memory additions and updates;
- whole memory-node deletion requested through the tool;
- skills created by the agent;
- duplicate-memory cleanup proposed by the dream pass.

The proposal model also defines new-tool and mission types, but the generic queue
approver does not currently apply those two types. They should not be treated as
complete Tier 2 workflows yet.

### Risk policy

| Risk | Examples | Automatic behavior |
|---|---|---|
| Safe | New memory information that does not collide with existing state | Can apply automatically in Autonomous and Hybrid modes. |
| Sensitive | Updating state or creating a skill | Can apply only when the mode and confidence policy allow it. |
| Critical | Deletion, overwrite or dream deduplication | Never applies automatically. A person must approve it. |

When the queue gate is enabled while LaReine is Off, it becomes a pure human gate and
queues every covered write. In Human in the loop mode it also queues everything.

For curator memory writes, the current proposal policy uses an internal confidence
threshold of 60. The configurable Hybrid threshold applies to response review; it is
not yet passed into this queue decision.

Proposals can be approved or rejected individually from the Memory interface. **Apply
all safe** accepts every pending safe addition. Rejected items remain in the journal
for audit. Pending items older than 14 days are marked expired when the queue is listed.
The status model also supports obsolete proposals whose target changed after creation.

Turning LaReine or the queue gate off does not delete or auto-apply the backlog. Existing
proposals remain pending. Only new writes stop being gated when `queue_gate` is off.

## Tier 3: live task supervision

Tier 3 watches plan progress inside the butinage loop. It is different from Tier 1:
it acts while a task is still running rather than judging only the final answer.

The current policy is deterministic:

1. If no plan exists, supervision stays passive.
2. After three consecutive passes without a newly completed plan step, LaReine injects
   an internal corrective message.
3. The message names the next unfinished step and can include relevant cognitive-memory
   recall for that step.
4. Progress resets the stagnation counter.
5. After two corrective interventions without progress, the run ends with an
   escalation instead of consuming more passes.

Tier 3 is available only when its switch is enabled and the LaReine mode is not Off.
The stagnation threshold and intervention count currently use built-in defaults rather
than settings exposed in the dashboard.

LaReine and the Vigie solve related but different problems. The Vigie blocks repeated
tool failures and sterile call patterns. Tier 3 watches whether the declared plan is
actually advancing, even when individual calls are technically valid.

## Scorecards and training data

Every completed review appends a numeric record to
`evals/reine-scorecards.jsonl`. The settings dashboard aggregates:

- review, approve, revise and escalation counts;
- number of answers actually reworked;
- average relevance, methodology, objective, brand and confidence scores;
- average review rounds;
- the most recent scorecards.

Scorecards do not store enough text to train a model. Full-text dataset capture is a
separate opt-in setting. When enabled, it records the request, initial draft, released
answer, corrections, reasoning and scores in `evals/reine-dataset.jsonl`. The dashboard
can export SFT conversations, genuine DPO preference pairs and judge-distillation
examples.

Secret values are masked before full-text capture, but this is not a complete privacy
scrub. See [Training Datasets](Training-Datasets) for the formats, curation workflow and
security limits.

## Configuration and stored state

The main settings are persisted in `laruche-reine.json`:

| Setting | Default | Purpose |
|---|---:|---|
| `mode` | `off` | Off, Autonomous, Hybrid or Human in the loop |
| `max_revues` | `0` | Maximum reworks; `255` means Unlimited |
| `seuil_confiance` | `60` | Hybrid response-escalation threshold |
| `contexte_messages` | `4` | Earlier messages shown to the judge, capped at 20 |
| `provider_profile` | worker profile | Optional separate judge profile |
| `tier_reponse` | `true` | Tier 1 switch |
| `tier_artefacts` | `false` | Stored Tier 2 intent switch |
| `tier_supervision` | `false` | Tier 3 switch |
| `queue_gate` | `false` | Live gate for covered memory and artifact writes |
| `dataset` | `false` | Full-text training capture |

The scorecard journal is always written after completed reviews. The full-text dataset
is not. This separation makes it possible to track quality without retaining entire
conversations.

## Choosing a mode

- Use **Off** when latency matters and call the crown only on answers worth auditing.
- Use **Autonomous** when the worker can safely redo routine work without waiting for a
  person.
- Use **Hybrid** when autonomous rework is useful but uncertain judgments should be
  visible to a person.
- Use **Human in the loop** when every review must be flagged and no response should be
  rewritten automatically. It is not a strict response-approval gate.
- Enable the **proposal queue** when memory deletion, generated skills or curator writes
  need an approve or reject trail.
- Enable **Tier 3** for long planned tasks where lack of progress matters more than one
  failed tool call.

For serious evaluation or dataset collection, use a judge different from the worker,
keep the charter concrete and inspect score trends before changing models or thresholds.
