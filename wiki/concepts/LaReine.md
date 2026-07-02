# LaReine

LaReine ("the queen") is LaRuche's built-in supervisor: a judge that reviews the agent's
answers before you see them, with the authority to send the agent back to work.

## Why a supervisor

Agents fail in a specific way: they produce confident answers built on skipped work.
The tool call that was never made, the page that was never fetched, the "I checked and
everything is fine" that checked nothing. A judge that only reads the final answer
cannot catch this. LaReine can, because it sees the run, not just the prose.

## How judging works

1. The agent produces a draft answer.
2. LaReine scores it against **the charter**, an editable document describing what a
   good answer looks like (grounding, methodology, completeness, tone).
3. The **methodology score is grounded in facts**: LaReine sees which tools were
   actually called during the run. An answer claiming research that never happened
   scores accordingly.
4. Based on the verdict, LaReine either releases the answer, escalates to you with its
   concerns, or sends the agent back.

## The redo loop

A redo is a fresh agentic run with LaReine's critique injected, not a rephrase of the
same text. Two guards keep this honest:

- **Best-draft anti-regression**: LaRuche keeps the best-scoring draft across retries.
  If redo number 2 is worse than redo number 1, you get redo number 1. Quality never
  goes backward because the judge asked for more.
- **Budget**: `LARUCHE_REINE_BUDGET_SECS` caps the total time spent on judging and
  redoing, so supervision never turns a quick question into a ten-minute wait.

## The proposal queue

Self-modification goes through review. When the agent (or the curator, or the dream
pass) wants to change its own equipment: a new skill, a memory edit, a deletion, the
change can be routed into a durable proposal queue instead of applying silently.

It works like a pull request: the proposal shows the full content, you approve or
reject, and the queue survives restarts. The agent proposes, you dispose.

## Scorecards

Every verdict is appended to a JSONL scorecard journal and surfaced in the dashboard
(`/api/reine/scorecards`): scores over time, redo rates, escalations. You can literally
watch answer quality as a time series, and see whether a model change or a charter edit
made things better.

## Configuration

Everything is in **Settings > LaReine**: on/off, the charter text, thresholds, the
budget. The charter is yours to write; the defaults are a starting point, not a rule.
