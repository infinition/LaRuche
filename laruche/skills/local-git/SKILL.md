---
type: skill
name: local-git
description: Inspect and commit local git work, or branch into an isolated worktree.
---

# Local git

These tools act on the repository the session is working in. They are for LOCAL history:
seeing what changed and recording it. Anything involving GitHub itself, pull requests,
issues, reviews, authentication, belongs to the github-* skills, which drive the `gh`
CLI instead.

## Seeing the state

- `git_status` with optional `path`. Always the first call. What is modified, what is
  staged, what is untracked.
- `git_diff` with optional `path` and `staged`. Without `staged`, it shows unstaged
  changes; with it, what is actually about to be committed. **Read the staged diff before
  every commit.** It is the only way to know what you are recording.
- `git_log` with optional `count`. Recent history. Read it before writing a message, to
  match the repository's existing style.

## Committing

`git_commit` with `message`, and optional `add_all`.

1. `git_status`, to see the full picture.
2. `git_diff` with `staged: true`, to read exactly what will be recorded.
3. If untracked or unstaged files belong in this commit, `add_all` stages everything.
   Look at the status output first: `add_all` will sweep in build artefacts, secrets and
   scratch files if they are not ignored. When in doubt, do not use it.
4. `git_commit` with a message that says WHY, not what. The diff already says what.
   - First line under about 72 characters, imperative: "Fix watcher never firing on log
     targets".
   - Then a blank line, then the reasoning if it is not obvious.
5. `git_log` to confirm the commit landed with the message you intended.

Commit only when the user asked for it, or when you have been told to work that way.
Recording history is a decision that belongs to them.

## Isolated worktrees

For speculative or risky work, branch out instead of experimenting in place.

- `git_worktree_enter` with `branch_name` creates an isolated worktree and moves the
  session's working directory into it. The main checkout is untouched.
- `git_worktree_exit` returns to the original repository.

Use it when you are about to try something you may want to throw away entirely: a large
refactor, a dependency upgrade, an approach you are unsure of.

While inside, every path-relative tool resolves against the WORKTREE, not the original
repository. If you write to a path that looks familiar, you are writing to the copy.
Commit before exiting; uncommitted work in a worktree is easy to lose track of.

## Traps

- **`add_all` is not "stage my changes".** It stages everything git can see that is not
  ignored. Run `git_status` first and read the untracked list. One careless `add_all` is
  how a token or a 200 MB model file enters history permanently.
- **`git_diff` without `staged` does not show what you are about to commit.** Two
  different questions, two different flags.
- **Committing on the default branch.** If the repository has a `main` or `master` and
  the user has not said to commit there, create a branch first.
- **Never rewrite published history** on your own initiative: no force push, no reset
  hard, no amend on a commit that already left the machine. If it looks necessary, say so
  and let the user decide.
- **Do not skip hooks.** A failing hook is a real signal. Fix the cause.

## Failure modes

**`git_commit` reports nothing to commit.** Nothing is staged. Either the edits went to a
different path than you think, or `add_all` was omitted. Run `git_status` and read where
the changes actually are.

**The commit contains files you did not expect.** `add_all` swept them in. Say so
immediately rather than quietly continuing, and let the user decide whether to amend
before it is pushed.

**Tools report a clean repository but your edits exist.** You are inside a worktree, or
the session's working directory is not the repository you think. `git_status` output
names the path; check it against where you wrote.

**`git_worktree_enter` fails on an existing branch name.** The branch or worktree is
already there. Pick another name, or exit and reuse the existing one.
