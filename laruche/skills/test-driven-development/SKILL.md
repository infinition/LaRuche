---
type: skill
name: test-driven-development
description: "TDD: RED-GREEN-REFACTOR cycle, test-first enforcement."
version: 1.2.0
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec]
metadata:
  laruche:
    tags: [testing, tdd, development, quality, red-green-refactor]
    related_skills: [systematic-debugging, plan]
---

# Test-Driven Development (TDD)

## Core Principle

Write the test first. Watch it fail. Write minimal code to pass.

**If you didn't watch the test fail, you don't know if it tests the right thing.**

**Violating the letter of the rules is violating the spirit of the rules.**

## When to Use

**Always:** new features, bug fixes, refactoring, behavior changes.

**Exceptions (ask the user first):** throwaway prototypes, generated code, configuration files.

Thinking "skip TDD just this once"? Stop. That's rationalization.

## The Iron Law

```
NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST
```

Wrote code before the test? Delete it. Start over. No exceptions — don't keep it as "reference," don't "adapt" it while writing tests. Delete means delete.

## Red-Green-Refactor Cycle

### RED — Write Failing Test

Write one minimal test showing what should happen.

**Good:**
```python
def test_retries_failed_operations_3_times():
    attempts = 0
    def operation():
        nonlocal attempts
        attempts += 1
        if attempts < 3:
            raise Exception('fail')
        return 'success'

    result = retry_operation(operation)

    assert result == 'success'
    assert attempts == 3
```
Clear name, tests real behavior, one thing.

**Bad:**
```python
def test_retry_works():
    mock = MagicMock()
    mock.side_effect = [Exception(), Exception(), 'success']
    result = retry_operation(mock)
    assert result == 'success'  # Vague name, tests mock not real code
```

Requirements:
- One behavior per test
- Descriptive name ("and" in name? Split it)
- Real code, not mocks (unless truly unavoidable)
- Name describes behavior, not implementation

### Verify RED — Watch It Fail (MANDATORY)

```bash
pytest tests/test_feature.py::test_specific_behavior -v
```

Confirm:
- Test fails (not errors from typos)
- Failure message is expected
- Fails because the feature is missing

**Test passes immediately?** You're testing existing behavior. Fix the test.
**Test errors?** Fix the error, re-run until it fails correctly.

### GREEN — Minimal Code

Write the simplest code to pass the test. Nothing more.

Cheating is OK in GREEN: hardcode return values, copy-paste, duplicate code, skip edge cases. Fix it in REFACTOR. Don't add features or refactor other code.

### Verify GREEN — Watch It Pass (MANDATORY)

```bash
# Run the specific test
pytest tests/test_feature.py::test_specific_behavior -v

# Run ALL tests — check for regressions
pytest tests/ -q
```

**Test fails?** Fix the code, not the test.
**Other tests fail?** Fix regressions now.

### REFACTOR — Clean Up

After green only: remove duplication, improve names, extract helpers, simplify expressions. Keep tests green throughout. Don't add behavior.

**If tests fail during refactor:** undo immediately. Take smaller steps.

### Repeat

Next failing test for next behavior. One cycle at a time.

## Avoid Horizontal Slices

Do **not** write all tests first then all implementation. That produces brittle tests designed before the implementation taught you what behavior and interface actually matter.

```text
WRONG:  RED: test1, test2, test3 → GREEN: impl1, impl2, impl3
RIGHT:  RED→GREEN: test1→impl1 / test2→impl2 / test3→impl3
```

Use vertical tracer bullets: one end-to-end behavior slice per cycle. Proves the path works, teaches you the interface, keeps each next test grounded.

## LaRuche Integration

Run tests at each step via `shell_exec`:

```python
# RED — verify failure
shell_exec("pytest tests/test_feature.py::test_name -v")

# GREEN — verify pass
shell_exec("pytest tests/test_feature.py::test_name -v")

# Full suite — verify no regressions
shell_exec("pytest tests/ -q")
```

When dispatching subagents for implementation, include in their goal:

```
Follow TDD strictly:
1. Write failing test FIRST
2. Run test — confirm it fails for the right reason
3. Write minimal code to pass
4. Run test — confirm it passes
5. Run full suite — fix any regressions
6. Refactor if needed, keep green

Test command: pytest tests/ -q
```

**Bug found?** Write a failing test reproducing it first. Follow the TDD cycle. The test proves the fix and prevents regression. Never fix bugs without a test.

## Common Rationalizations

| Excuse | Reality |
|--------|---------|
| "Too simple to test" | Simple code breaks. Test takes 30 seconds. |
| "I'll test after" | Tests-after pass immediately — prove nothing. |
| "Tests after same goals" | Tests-after = "what does this do?" Tests-first = "what should this do?" |
| "Already manually tested" | Ad-hoc ≠ systematic. No record, can't re-run. |
| "Deleting X hours is wasteful" | Sunk cost. Unverified code is technical debt. |
| "Keep as reference" | You'll adapt it. That's testing after. Delete means delete. |
| "Need to explore first" | Fine. Throw away exploration, start with TDD. |
| "Hard to test" | Listen: hard to test = hard to use. Simplify the design. |
| "TDD will slow me down" | TDD faster than debugging in production. |
| "Existing code has no tests" | Add tests for every line you touch. |

## Red Flags — STOP and Start Over

- Code written before test
- Test passes immediately on first run
- Can't explain why the test failed
- Tests added "later"
- Rationalizing "just this once"
- "Keep as reference" or "adapt existing code"

**Any of these → delete code, restart with TDD.**

## Verification Checklist

Before marking work complete:

- [ ] Every new function/method has a test
- [ ] Watched each test fail before implementing
- [ ] Each test failed for expected reason (feature missing, not typo)
- [ ] Wrote minimal code to pass each test
- [ ] All tests pass, output pristine
- [ ] Tests use real code (mocks only if unavoidable)
- [ ] Edge cases and errors covered

Can't check all boxes? You skipped TDD. Start over.

## When Stuck

| Problem | Solution |
|---------|----------|
| Don't know how to test | Write the wished-for API. Write the assertion first. Ask the user. |
| Test too complicated | Design too complicated. Simplify the interface. |
| Must mock everything | Code too coupled. Use dependency injection. |
| Test setup huge | Extract helpers. Still complex? Simplify the design. |

## Final Rule

```
Production code → test exists and failed first
Otherwise → not TDD
```

No exceptions without the user's explicit permission.
