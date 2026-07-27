---
type: skill
name: spike
description: Throwaway experiments to validate feasibility before building.
---

# Spike

A spike answers one question: **can this actually be done here, with these tools, on this
machine?** It answers it by building the smallest thing that would fail if the answer were
no. Then it gets thrown away.

The output of a spike is not code. It is a verdict, with evidence. Code that survives a
spike is a bonus, and usually a mistake.

Reach for this when the user is circling an idea rather than asking for it: "would this
even work", "I want to see if", "before I commit to", "is it possible to", "A or B?".

## When not to spike

- **The answer is already written down.** Documentation, a changelog, an issue thread. A
  spike that reproduces a documented limitation cost an hour to learn a sentence. Use
  `web-research` first, always.
- **The decision is already made.** If the user asked for the feature, build the feature.
  A spike here is procrastination with a directory structure.
- **It is going into production.** The moment code has to survive, this is not a spike.
  Write a plan, with the `plan` skill.
- **There is nothing to observe.** If you cannot state what result would make the answer
  "no", you do not have a question yet. Sharpen it before writing anything.

## The shape of a good question

One spike, one falsifiable question, with the failing condition stated up front.

Weak: "try websockets".
Strong: "with 200 concurrent connections, does the server deliver a token within 100ms?
If the median exceeds 250ms, the approach is dead."

Write the threshold BEFORE you build. Deciding what counts as success after seeing the
number is how every approach passes.

## Procedure

1. **Split the idea into two to five independent questions**, ordered by what would kill
   the idea fastest. The riskiest one runs first: if it fails, the rest never need to be
   built. Present them as a table and let the user reorder or drop.

   | # | Question | Fails if | Risk |
   |---|---|---|---|
   | 001 | Can we stream tokens over a websocket? | median chunk latency over 250ms | high |
   | 002a | Can pdfjs pull tables out of this PDF? | tables arrive as loose text | medium |
   | 002b | Can camelot do it? | same | medium |

   Two spikes answering the SAME question with different tools share a number and take a
   letter. That is what makes them comparable later.

2. **Look before you build.** Two or three searches, ten minutes: which libraries exist,
   which are maintained, which have already hit this wall. Skip this only for pure logic
   with no dependency. State which approach you picked, and why, in one line.

   ```
   web_search(query="python websocket streaming backpressure 2026")
   web_fetch(url="https://websockets.readthedocs.io/en/stable/faq/index.html")
   shell_exec(command="pip show websockets")
   ```

   `web_fetch` takes ONE `url`, as a string.

3. **Build the smallest observable thing.** One directory per spike, standalone, under
   `spikes/NNN-short-name/` at the repository root.

   ```
   shell_exec(command="mkdir -p spikes/001-websocket-streaming")
   file_write(path="C:/dev/project/spikes/001-websocket-streaming/main.py", content="...")
   shell_exec(command="cd spikes/001-websocket-streaming && python main.py")
   ```

   Pass ABSOLUTE paths to `file_write`: a relative one resolves against the server's
   working directory, not the project.

   Prefer, in order: a CLI that prints an observable result, a single HTML page, a
   one-endpoint server, a test with a recognisable assertion. The user should be able to
   run it and see the answer themselves.

   Hardcode everything. No config system, no container, no environment files, no build
   pipeline. Every minute spent on scaffolding is a minute not spent on the question.

4. **Push past the happy path.** One successful run is not evidence. Feed it the empty
   input, the huge input, the malformed one, the concurrent one. A spike that only proves
   the demo works has proved nothing about the idea.

5. **Write the verdict** into the spike's `README.md`, and stop.

## The verdict

```markdown
## Verdict: VALIDATED | PARTIAL | INVALIDATED

**Question:** <the one question, as asked>
**Threshold:** <what would have made this a no>
**Measured:** <the number or behaviour actually observed>

### What worked
### What did not
### What surprised us
### What this means for the real build
```

- **VALIDATED**, the threshold was met, with a number to point at.
- **PARTIAL**, it works inside constraints. Name them: this version, this size, this
  platform. A PARTIAL with unnamed constraints is a VALIDATED that will hurt later.
- **INVALIDATED**, it does not work, and here is why. **This is a successful spike.** It
  cost an afternoon instead of a sprint. Report it as a win, not an apology.

## Comparing two approaches

When 002a and 002b answer the same question, build both, then set them side by side on
the dimensions that will actually drive the decision. Not features: consequences.

```markdown
| | pdfjs (002a) | camelot (002b) |
|---|---|---|
| Tables extracted correctly | 9 of 10 | 6 of 10 |
| Setup | one npm install | pip plus ghostscript |
| 100-page document | 3s | 18s |
| Rotated text | no | yes |

**Pick pdfjs**, unless rotated text turns out to be common, which we have not measured.
```

Name the winner, and name what would change the answer. A comparison with no
recommendation hands the decision back to the user with more information and no help.

## What to spike next

When spikes already exist and the user asks what is missing, walk `spikes/` and look for
what nobody has tested:

- **Seams.** Two validated spikes that touch the same file, port or database, but were
  never run together.
- **Handoffs.** Spike A's output was assumed to fit spike B's input. Assumed, not shown.
- **The unexamined assumption.** Something the whole idea rests on that no spike names.
- **The second angle** on anything PARTIAL or INVALIDATED.

Propose two to four, as falsifiable questions with thresholds, and let the user pick.

## Traps

- **The spike that becomes the product.** Someone will want to keep it. A spike that takes
  two days to clean up was not a spike, it was a rushed first draft. Say so plainly.
- **Declaring victory on one run.** See step 4.
- **Moving the threshold.** If the number came in at 400ms against a 250ms bar, the answer
  is INVALIDATED, not "close enough with tuning". Tuning is another spike.
- **Spiking the easy question.** The comfortable spike is rarely the risky one. Order by
  what kills the idea, not by what you already know how to build.
- **Leaving `spikes/` in the commit.** Ask before committing throwaway code. Most
  repositories want it ignored.

## Failure modes

**The spike will not run at all, for reasons unrelated to the question.** That is
environment, not evidence. Fix it, or record it as blocked, but do not write INVALIDATED:
nothing was tested.

**The result is ambiguous.** The question was compound. Split it and spike the halves.

**Every approach fails the same way.** That is a finding about the problem, not about the
tools. Report it: the constraint is probably somewhere you have not looked yet.

**The user keeps asking for one more spike.** Three spikes on one subject with no decision
means the decision is not technical. Name the choice that is actually open, and hand it
back.
