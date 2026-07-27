---
type: skill
name: systematic-debugging
description: Find the root cause of a bug before writing any fix.
---

# Systematic Debugging

**Core principle:** Find root cause before attempting any fix. Symptom patches are failure.

```
NO FIXES WITHOUT ROOT CAUSE INVESTIGATION FIRST
```

If you haven't completed Phase 1, you cannot propose fixes. Apply this skill to ALL technical issues - especially "quick fixes" and emergencies.

---

## The Feedback Loop Rule

Before reading code to build a theory, establish a **tight** command that goes red on the exact symptom and green when the bug is fixed. Tight = fast, deterministic, agent-runnable, specific.

When a clean repro is hard, spend disproportionate effort building the loop. Guessing without a red-capable loop is the failure mode this skill exists to prevent.

---

## Phase 1: Root Cause Investigation

### 1. Read Error Messages Carefully

Read stack traces completely - line numbers, file paths, error codes. Don't skip warnings.

```bash
# View recent logs
shell_exec("tail -100 logs/app.log")

# Search error string in codebase
shell_exec("grep -rn 'ErrorString' src/")
```

Use `file_read` on relevant source files. To trace the error across the tree, use
`file_search` (`path` and `pattern`, plus `content` to match inside files) or run grep
through `shell_exec`.

### 2. Build a Tight Feedback Loop

Can you trigger the exact symptom with one command? Pick the lowest-cost loop type:

1. **Failing test** at the seam that reaches the bug (unit, integration, e2e)
2. **HTTP/curl** against a running dev server
3. **CLI invocation** with fixture input, diff stdout against expected
4. **Headless browser** asserting on DOM, console, or network
5. **Replay captured trace** - HAR, request payload, event log, webhook body
6. **Throwaway harness** booting the smallest useful slice of the system
7. **Property/fuzz loop** for intermittent wrong output over broad input space
8. **`git bisect run` harness** when the bug appeared between two known states
9. **Differential loop** - old vs new version, two configs, two providers, two datasets
10. **Human-in-the-loop script** (last resort) - structure the human steps, capture the result

**Tighten the loop:**
- Faster: cache setup, narrow scope, skip unrelated initialization
- Sharper: assert the exact symptom, not generic success
- More deterministic: pin time, seed randomness, isolate filesystem, freeze network

For non-deterministic bugs, raise reproduction rate before analyzing. Run 100×, parallelize, add stress, narrow timing windows. A 50% flake is debuggable; 1% usually is not.

```bash
shell_exec("pytest tests/test_module.py::test_name -v")
shell_exec("for i in $(seq 1 100); do pytest tests/test_flake.py::test_name -q || break; done")
```

### 3. Check Recent Changes

```bash
shell_exec("git log --oneline -10")
shell_exec("git diff")
shell_exec("git log -p --follow src/problematic_file.py | head -100")
```

### 4. Gather Evidence in Multi-Component Systems

For each component boundary (API → service → database, CI → build → deploy):
- Log what data enters and exits
- Verify environment/config propagation
- Check state at each layer

Run once to gather evidence → identify WHERE it breaks → investigate that component.

### 5. Trace Data Flow

Trace bad values upstream to their origin. Fix at the source, not at the symptom.

```bash
shell_exec("grep -rn 'function_name(' src/")
shell_exec("grep -rn 'variable_name\s*=' src/")
```

### Phase 1 Checklist

- [ ] Error messages fully read and understood
- [ ] Tight loop command exists and is red on the exact symptom
- [ ] Loop is deterministic, or flaky bug has high enough reproduction rate
- [ ] Recent changes identified and reviewed
- [ ] Evidence gathered (logs, state, data flow)
- [ ] Root cause hypotheses can be stated and tested

**STOP:** Do not proceed to Phase 2 until you understand WHY it's happening.

---

## Phase 2: Pattern Analysis

### 1. Minimize the Reproduction

Shrink the repro to the smallest scenario still going red. Remove inputs, callers, config, steps one at a time - re-running the loop after each cut. Done when removing anything more makes the loop go green.

### 2. Find Working Examples

```bash
shell_exec("grep -rn 'similar_pattern' src/")
```

### 3. Compare Against References

Read the reference implementation completely - every line. Understand the pattern fully before applying.

### 4. Identify Differences

List every difference between working and broken code, however small. Don't assume "that can't matter."

### 5. Understand Dependencies

What config, environment, or assumptions does this component require?

---

## Phase 3: Hypothesis and Testing

### 1. Form Ranked Falsifiable Hypotheses

Generate 3–5 plausible hypotheses before testing any. Rank by likelihood and cheapness to falsify. State the prediction each makes: "If X is the cause, then observing/changing Y should produce Z." Discard hypotheses that don't make testable predictions.

Show the ranked list to the user if present - they may have domain knowledge that re-ranks it instantly.

### 2. Test Minimally

Test the top hypothesis with the smallest possible probe. Change one variable at a time. Never fix multiple things at once. Prefer debugger/REPL inspection - one breakpoint beats ten logs.

If adding temporary logs, tag every line with a unique prefix (e.g., `[DEBUG-a4f2]`) so cleanup is a single search.

### 3. Verify Before Continuing

- Loop goes green → Phase 4
- Loop still red → form a NEW hypothesis; do NOT layer more fixes on top
- Don't know → say so explicitly; use `web_search`/`read_extract` to research

---

## Phase 4: Implementation

### 1. Write a Regression Test First

Write the simplest automated test that reproduces the bug and is currently red.

### 2. Implement a Single Fix

Address the root cause only. One change at a time. No "while I'm here" improvements. No bundled refactoring.

### 3. Verify the Fix

```bash
shell_exec("pytest tests/test_module.py::test_regression -v")
shell_exec("pytest tests/ -q")
```

### 4. Rule of Three

- Fix #1 failed → return to Phase 1 with new information
- Fix #2 failed → return to Phase 1, look harder
- **Fix #3 failed → STOP. Question the architecture.**

3+ failed fixes = architectural problem, not a bug:
- Each fix reveals new shared state/coupling elsewhere
- Fixes require massive refactoring
- Each fix creates symptoms in a different place

Discuss with the user before attempting another fix. The pattern itself may be wrong.

---

## Red Flags - Stop and Return to Phase 1

- "Quick fix for now, investigate later"
- "Just try changing X and see if it works"
- "It's probably X, let me fix that" (without tracing data flow)
- Making multiple changes at once to narrow it down
- "I don't fully understand but this might work"
- Proposing solutions before reproducing the bug
- "One more fix attempt" when 2+ have already failed

---

## Quick Reference

| Phase | Key Activity | Done When |
|-------|-------------|-----------|
| 1. Root Cause | Read errors, build loop, check changes, gather evidence | Know WHY |
| 2. Pattern | Minimize repro, find working examples, diff | Know WHAT differs |
| 3. Hypothesis | Rank hypotheses, test one at a time | Confirmed cause |
| 4. Implementation | Write regression test, fix root cause, verify | All tests pass |
