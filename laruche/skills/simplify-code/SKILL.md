---
type: skill
name: simplify-code
description: >-
  3-agent parallel cleanup: reuse, quality, efficiency.
---

# Simplify Code - Parallel Review & Cleanup

Three focused reviewers run in parallel - each owns one category (reuse,
quality, efficiency), searches the full codebase, then returns structured
findings. You pay one round-trip of latency, not three.

**Invoke only when the user explicitly asks.** Three subagents cost real
tokens - do not auto-run after every edit.

## Trigger phrases

- "simplify" / "simplify my changes" / "clean up my changes"
- "/simplify"

Optional modifiers:

| Modifier | Behavior |
|---|---|
| `focus on efficiency` | Run only the efficiency reviewer (or weight that tier first) |
| `just report` / `dry run` | Present all findings, apply nothing |
| `the last commit` / `staged` / `src/foo.py` | Scope the diff (see Phase 1) |

## Phase 1 - Capture the diff

Pick source in this order:

```bash
git diff                  # default: uncommitted working-tree changes
git diff HEAD             # if above is empty
git diff --staged         # user asks for "staged"
git diff HEAD~1           # user asks for "last commit"
git diff main...HEAD      # user asks for "this branch" / "my PR"
git diff -- src/foo.py    # specific file
```

Use `shell_exec` to run these. If all are empty and there's no git repo, fall
back to files the user named or recently edited. If there's nothing to review,
say so and stop.

**Size warning:** if the diff exceeds ~2000 changed lines, warn the user -
three subagents each carrying that much context is expensive. Offer to scope
down before proceeding.

## Phase 2 - Three reviewers in parallel

Spawn three subagents concurrently via LaRuche's parallel task mechanism (e.g.
`tool_call` with batch/concurrent dispatch, or three simultaneous `shell_exec`
script invocations). Pass each the **full diff text** and the repo's absolute
path.

Give each reviewer these instructions:

- Search the codebase with `file_search` / `shell_exec grep -r` - never reason
  from the diff alone.
- Apply **Chesterton's Fence**: run `git blame` on suspicious lines before
  flagging them for removal. Unknown purpose → `confidence: low`.
- Report findings in this format:
  ```
  file:line → problem → fix | confidence: high/medium/low | risk: SAFE/CAREFUL/RISKY
  ```
  - **SAFE** - proven not to change behavior (unused import, dead comment,
    pass-through wrapper). Auto-apply.
  - **CAREFUL** - improves without changing semantics (rename local, flatten
    ternary, extract helper). Apply with test verification.
  - **RISKY** - may change behavior or breaks public contracts (N+1
    restructuring, public API rename, concurrency fix). Flag only - do NOT
    auto-apply.
- Skip nits and pure style churn. Only flag material improvements.

### Reviewer 1 - Code Reuse

> Scan for code that duplicates functionality already in the codebase. Use
> `grep -r` (via `shell_exec`) on utility modules, shared helpers, and adjacent
> files. Flag: new functions that duplicate existing ones; hand-rolled logic an
> existing utility already covers (path ops, env checks, type guards, parsing).
> For each finding, name the existing thing and its file:line.

### Reviewer 2 - Code Quality

> Scan for: redundant state (values derivable from existing state); parameter
> sprawl (bolted-on params instead of restructuring); copy-paste blocks that
> should share an abstraction; leaky abstractions; stringly-typed code (raw
> strings where a constant/enum/registry exists - search the codebase first);
> AI slop patterns (`// increment counter` above `count++`; defensive null-checks
> on already-validated inputs; `as any` casts; style inconsistencies with the
> rest of the file). For each, give the concrete refactor.

### Reviewer 3 - Efficiency

> Scan for: redundant computation, repeated reads, duplicate API calls, N+1
> patterns; missed concurrency (sequential independent ops); hot-path bloat
> (heavy work on startup or per-request); TOCTOU pre-checks (existence check
> before op → do the op and handle error instead); memory leaks (unbounded
> growth, missing cleanup, listener leaks); overly broad reads (loading whole
> file when a slice suffices); silent failures (empty catch, `except: pass`,
> `.catch(() => {})` - these hide bugs, at minimum log before swallowing).
> For each, give the concrete fix and why it's faster or safer.

## Phase 3 - Aggregate and apply

1. **Merge** all findings; dedupe overlaps.
2. **Discard false positives** - drop weak or wrong suggestions (no need to
   argue; just omit them).
3. **Resolve conflicts** (e.g. "reuse util X" vs "X is slow, inline it"):
   `correctness > user's stated focus > readability/reuse > micro-perf`.
   When two defensible fixes are mutually exclusive, pick the one touching less
   code and note the alternative.
4. **Apply in risk order** using `file_edit`:
   - **SAFE first** - auto-apply, then run tests.
   - **CAREFUL next** - one file at a time, run tests after each. Revert on
     failure.
   - **RISKY - never auto-apply.** Present with risk description and test
     coverage status.
   - Dry-run mode: present all three tiers, apply nothing.
5. **Verify** with `shell_exec`: run the project's targeted tests for touched
   files plus any linter/type-check the repo uses. Revert fixes that break tests
   and report them.
6. **Summarize**: short list of applied fixes by category + risk tier, plus
   deliberately skipped findings and why.

## Pitfalls

- **Give the WHOLE diff to each reviewer.** Cross-file duplication and N+1s
  only appear with the full picture - do not split the diff.
- **Reviewers must cite evidence.** A reuse finding with no `file:line` pointer
  is noise. Drop findings that lack it.
- **Apply ≠ rewrite.** Scope edits to what the diff touched plus the minimal
  surrounding change a fix requires. Do not refactor the whole module.
- **Respect project conventions.** If the repo has CLAUDE.md, a linter config,
  or AGENTS.md, inject those rules into reviewer prompts.
- **Dead-code tools lie.** `knip`, `ts-prune`, `depcheck` flag exports used
  dynamically. Always `grep -r` the symbol name before removing.
- **Public contracts are RISKY.** Export names, API routes, DB columns, config
  keys - even a bad name is a contract. Never auto-rename them.
- **Don't widen beyond 3 reviewers.** More reviewers = more conflicts to
  reconcile, not better coverage.

## Related

- `requesting-code-review` - pre-commit security/quality gate.
- `test-driven-development` - test coverage for changes being cleaned up.
