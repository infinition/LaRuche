---
type: skill
name: plan
description: "Write an actionable markdown plan (no execution): bite-sized tasks, exact paths, full code."
version: 2.0.0
license: MIT
platforms: [linux, macos, windows]
tools: [file_read, file_list, file_write, shell_exec]
metadata:
  laruche:
    tags: [planning, plan-mode, implementation, workflow, design]
    related_skills: [subagent-driven-development, test-driven-development, requesting-code-review]
---

# Plan Mode

Use this skill when the user wants a plan, not execution.

## Core behavior

**Planning only — no implementation this turn.**

- Do not edit project files except the plan file itself.
- Do not run mutating commands, commit, push, or perform external actions.
- Inspect the repo freely with read-only tools (`file_read`, `file_list`, `shell_exec` read-only).
- Deliverable: a markdown plan saved under `.laruche/plans/`.

## Save location

```
.laruche/plans/YYYY-MM-DD_HHMMSS-<slug>.md
```

Use a path relative to the active workspace root. If the runtime specifies a target path, use that instead.

## Interaction style

- Clear request → write the plan directly.
- Invoked without context → infer from conversation history.
- Genuinely underspecified → ask one brief clarifying question.
- After saving → reply with a one-line summary and the saved path.

---

## Writing the Plan

**Core principle:** A good plan makes implementation obvious. If someone has to guess, the plan is incomplete.

Assume the implementer is skilled but has zero codebase context and weak test design instincts. Give them bite-sized tasks, exact paths, complete code, and exact commands.

### Plan document structure

```markdown
# [Feature Name] Implementation Plan

**Goal:** [One sentence]
**Architecture:** [2–3 sentences on approach]
**Tech Stack:** [Key technologies/libraries]

---
```

### Task structure

````markdown
### Task N: [Descriptive Name]

**Objective:** [One sentence]

**Files:**
- Create: `exact/path/to/new_file.py`
- Modify: `exact/path/to/existing.py:45-67`
- Test: `tests/path/to/test_file.py`

**Step 1: Write failing test**
```python
def test_specific_behavior():
    assert function(input) == expected
```

**Step 2: Verify failure** — `pytest tests/path/test.py::test_specific_behavior -v`
Expected: FAIL — "function not defined"

**Step 3: Write minimal implementation**
```python
def function(input):
    return expected
```

**Step 4: Verify pass** — `pytest tests/path/test.py::test_specific_behavior -v`
Expected: PASS

**Step 5: Commit**
```bash
git add tests/path/test.py src/path/file.py
git commit -m "feat: add specific feature"
```
````

### Task granularity

Each task = 2–5 minutes of focused work. One action per step. If a task feels large, split it.

### Task ordering

1. Setup / infrastructure
2. Core functionality (TDD per unit)
3. Edge cases
4. Integration
5. Cleanup / documentation

### Planning process

1. **Understand** — read requirements, acceptance criteria, constraints.
2. **Explore** — use `file_list`, `file_read`, `shell_exec` (read-only) to understand structure and find similar patterns.
3. **Design** — choose architecture, file organization, dependencies, testing strategy.
4. **Write tasks** — exact paths, complete copy-pasteable code, exact commands with expected output, verification steps.
5. **Review** — tasks are sequential, bite-sized, DRY, YAGNI, TDD-compliant, leave nothing to guesswork.

### Principles

- **DRY** — extract repeated logic into shared functions.
- **YAGNI** — implement only what is needed now.
- **TDD** — every code-producing task includes the full red→green cycle.
- **Frequent commits** — one commit per completed task.

### Common mistakes

| Bad | Good |
|-----|------|
| "Add authentication" | "Create User model with email and password_hash fields" |
| "Add validation function" | Include the complete function code |
| "Test it works" | "`pytest tests/test_auth.py -v` — expected: 3 passed" |
| "Create the model file" | "Create: `src/models/user.py`" |

## Execution handoff

After saving the plan, offer:

> "Plan saved at `<path>`. Ready to execute task-by-task — shall I start?"
