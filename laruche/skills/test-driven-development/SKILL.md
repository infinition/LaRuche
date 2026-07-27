---
type: skill
name: test-driven-development
description: Write the failing test first, then the code that makes it pass.
---

# Test-driven development

One rule, and everything else follows from it: **you do not write production code until a
test is failing for the right reason.**

Not "you write tests". Everyone writes tests. The order is the whole technique, because a
test written after the code can only confirm what the code already does. It never asks
whether that was the right thing to build.

## Why the order is not a formality

A test you never saw fail is not a test, it is an assertion of faith. It might be passing
because the behaviour works. It might be passing because it asserts nothing, because the
fixture is wrong, because it exercises a mock instead of the code, or because you compared
the return of a function that yields `None` against `None`.

You cannot tell those apart by reading it. You can only tell by watching it fail, then
watching that specific failure disappear.

That is the entire argument. Everything below is bookkeeping around it.

## When this applies

Every behaviour change: a feature, a bug fix, a refactor that alters semantics.

It does not apply to a throwaway experiment, to generated code, or to configuration. For
an experiment use the `spike` skill, and throw the result away rather than retrofitting
tests onto it.

If you catch yourself reasoning "this one is too simple to need a test", notice what that
sentence actually is. It is never a conclusion about the code, it is a preference about
the next ten minutes.

## The cycle

### Red: write one failing test

One behaviour. Name it after the behaviour, not the function:
`retries_three_times_before_giving_up`, not `test_retry`. If the name needs an "and", you
are testing two things; split it.

Test the real code. A test that exercises a mock verifies that the mocking library works,
which was not in doubt.

```python
def test_retries_three_times_before_giving_up():
    attempts = []

    def flaky():
        attempts.append(1)
        if len(attempts) < 3:
            raise ConnectionError("boom")
        return "ok"

    assert retry(flaky) == "ok"
    assert len(attempts) == 3
```

### Watch it fail, and read the failure

```
shell_exec(command="pytest tests/test_retry.py::test_retries_three_times_before_giving_up -v")
```

This step is not ceremony. You are checking three things:

- it FAILS, rather than erroring on an import or a typo;
- it fails for the RIGHT reason, naming the behaviour that is missing;
- the message would tell a stranger what is wrong.

**If it passes immediately, something is wrong.** Either the behaviour already exists, in
which case there is nothing to build, or the test asserts nothing. Read it again before
writing a line of production code.

### Green: the smallest thing that passes

Cheating is allowed here, and is often correct. Return the constant. Duplicate the block.
Ignore the edge case. The goal of this step is a passing test, not good code.

Resist writing the general solution now. The NEXT failing test is what tells you what the
general solution actually needs to be, and it is rarely what you would have guessed.

### Watch it pass, and watch nothing else break

```
shell_exec(command="pytest tests/test_retry.py::test_retries_three_times_before_giving_up -v")
shell_exec(command="pytest -q")
```

The second command is the one people skip. A green test beside three newly red ones is not
progress. Fix the regressions now, while you still know exactly which change caused them.

If the new test still fails, fix the CODE. Editing the test until it passes converts a real
failure into a permanent lie.

### Refactor, only on green

Now remove the duplication, fix the names, extract the helper. Run the suite after each
step. If a refactor turns anything red, undo it immediately and take a smaller step: you
have just learned the change was not behaviour-preserving.

Do not add behaviour here. New behaviour needs a new failing test.

### Then do it again

One behaviour per cycle, end to end. Not all the tests, then all the code: that produces
tests designed against an interface nobody has used yet, brittle in the specific way that
makes people stop trusting the suite.

```
wrong:  test1 test2 test3, then code1 code2 code3
right:  test1 -> code1, test2 -> code2, test3 -> code3
```

## Fixing a bug

The bug IS the missing test. Before touching the fix:

1. Write a test that reproduces the bug, and that fails.
2. Watch it fail with exactly the symptom the user reported. If it fails differently, you
   have not reproduced their bug, and the fix will be for a different one.
3. Fix it. Watch the test go green.

The test now proves two things at once: that the bug was real, and that it will be caught
if it returns. Both are lost if you fix first and test afterwards.

For a bug you cannot yet reproduce, use `systematic-debugging` to find it, then come back
here to fix it.

## Delegating implementation

When you hand work to another agent with `delegate` or `spawn_specialist`, the discipline
does not travel by itself. Put it in the brief, with the exact command:

```
Follow TDD strictly. For each behaviour:
1. write the failing test first
2. run it, confirm it fails for the right reason
3. write the minimum code to pass
4. run it again, then run the whole suite
5. refactor only while green
Test command: pytest -q
Do not report success while any test is red.
```

Then verify what comes back. A delegate reporting "all tests pass" is a claim, not a fact.
Run the suite yourself.

## Traps

- **Writing the test after the code, then reordering the commits.** The test still only
  describes what was built. The order counts when it is lived, not when it is staged.
- **Keeping the code you wrote first "as a reference".** You will adapt it, which is
  testing afterwards with extra steps. Delete it.
- **Mocking the thing under test.** If the only way to test it is to replace most of it,
  the design is telling you something. Hard to test means hard to use.
- **A test that can never fail.** Break the production code on purpose once in a while. A
  test that stays green through that is decoration.
- **Enormous setup.** Twenty lines of fixture for a three-line assertion is a design
  problem surfacing as a testing problem.
- **Running only the new test.** The suite is the regression net. Run it.

## Failure modes

**The test passes on its very first run.** Either the behaviour exists already, or the
test asserts nothing meaningful. Break the production code deliberately; if the test stays
green, rewrite it.

**The test errors instead of failing.** An import, a typo, a missing fixture. That is not
Red yet. Fix the error and re-run until you get a real assertion failure.

**One green test, three regressions.** The implementation changed shared behaviour. Fix
them before continuing: the cause is exactly one change old.

**You cannot see how to test it.** Write the call you WISH existed, and the assertion you
want from it, then make that run. The interface you invent while writing the test is
usually better than the one you would have derived from the implementation. If it is still
impossible, the coupling is the real problem.

**The existing code has no tests at all.** Do not stop to retrofit a suite. Add tests for
what you touch, as you touch it. Coverage then grows along the paths people actually
change, which is where it is worth having.

**The user explicitly asks you to skip it.** Then skip it. Say plainly what is not
covered, and move on. This skill is a default, not a veto over the person whose repository
it is.
