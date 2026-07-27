---
type: skill
name: github-pr-workflow
description: Take a change from branch to merged pull request, CI included.
---

# GitHub pull request workflow

Branch, commit, open the pull request, watch CI, fix what it says, merge, clean up. Every
step is given twice: with `gh`, and with `git` plus `curl` for the machines that do not
have it, which is most servers.

Run the shell blocks through `shell_exec`. Read `github-auth` first if anything returns
401 or 404.

## Two things that need consent

**Pushing and merging are outward actions.** A push puts the user's name on a public
commit; a merge changes the default branch other people build on. Do them because the user
asked for them, not because the workflow reached that step. When in doubt, stop and show
what you are about to push.

**Never force-push a branch that has left the machine**, and never rewrite history on a
shared branch, on your own initiative.

## Setup, once per session

```bash
if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
  AUTH=gh
else
  AUTH=curl   # ${GITHUB_TOKEN} comes from the LaRuche vault
fi

OWNER_REPO=$(git remote get-url origin | sed -E 's|.*github\.com[:/]||; s|\.git$||')
OWNER=${OWNER_REPO%%/*}
REPO=${OWNER_REPO##*/}
echo "$AUTH on $OWNER/$REPO"
```

If `OWNER_REPO` comes out empty or still looks like a URL, the remote is not GitHub, or
there is no `origin`. Stop there: everything below will fail in a way that blames
authentication.

## 1. Branch

```bash
git fetch origin
git switch main && git pull --ff-only origin main
git switch -c feat/jwt-authentication
```

`--ff-only` refuses to create a merge commit behind your back. If it fails, the local
`main` has commits that are not on the remote, and that is worth understanding before
branching off it.

Prefix by intent: `feat/`, `fix/`, `refactor/`, `docs/`, `ci/`, `chore/`.

## 2. Commit

Edit with `file_write` and `file_edit`, then stage deliberately:

```bash
git status
git add src/auth.py src/models/user.py tests/test_auth.py
git commit -m "feat: add JWT-based authentication

Sessions were stored server-side, which blocked horizontal scaling.
Tokens move that state to the client.

Closes #42"
```

Name the files. `git add .` sweeps in build output, editor droppings and, once in a while,
a `.env`. Read `git status` before staging, every time.

The subject line follows Conventional Commits, `type(scope): summary`, under about 72
characters, imperative. The body says WHY: the diff already says what.

## 3. Open the pull request

```bash
git push -u origin HEAD
```

With `gh`:

```bash
gh pr create --title "feat: add JWT-based authentication" --body "$(cat <<'BODY'
## What this changes
Session state moves from the server to a signed token.

## Why
Server-side sessions pinned every user to one process, which blocked scaling out.

## How to verify
- `pytest tests/test_auth.py -q`
- log in, restart the server, confirm the session survives

Closes #42
BODY
)"
```

`--draft` opens it unreviewable, `--reviewer a,b`, `--label enhancement`, `--base develop`
when the target is not the default branch.

With curl:

```bash
BRANCH=$(git branch --show-current)
curl -s -X POST \
  -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  -H "Accept: application/vnd.github+json" \
  "https://api.github.com/repos/$OWNER/$REPO/pulls" \
  -d "$(python -c "
import json, sys
print(json.dumps({
  'title': 'feat: add JWT-based authentication',
  'body': open('/tmp/pr-body.md').read(),
  'head': sys.argv[1],
  'base': 'main',
}))" "$BRANCH")"
```

Build the JSON with a tool, not with string interpolation. A body containing a quote, a
backtick or a newline produces malformed JSON, and the API answers with a validation error
that says nothing about quoting.

Keep the `number` from the response: it is `PR_NUMBER` everywhere below.

## 4. Watch CI

```bash
gh pr checks           # once
gh pr checks --watch   # until everything settles
```

With curl, and this is where the trap is:

```bash
SHA=$(git rev-parse HEAD)
curl -s -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  "https://api.github.com/repos/$OWNER/$REPO/commits/$SHA/check-runs" \
  | python -c "
import sys, json
runs = json.load(sys.stdin).get('check_runs', [])
if not runs:
    print('no check runs')
for run in runs:
    print(f\"{run['name']}: {run['status']} / {run['conclusion'] or 'pending'}\")"
```

**Use `check-runs`, not `status`.** They are two different APIs. The legacy
`/commits/$SHA/status` endpoint only reports commit statuses posted by external CI. A
repository whose CI is GitHub Actions returns `"state": "pending"` there with an empty
`statuses` array, forever. Polling it waits for something that will never arrive, and the
timeout gets read as "CI is slow" rather than "wrong endpoint".

Poll on the conclusion, with a ceiling:

```bash
SHA=$(git rev-parse HEAD)
for i in $(seq 1 20); do
  PENDING=$(curl -s -H "Authorization: Bearer ${GITHUB_TOKEN}" \
    "https://api.github.com/repos/$OWNER/$REPO/commits/$SHA/check-runs" \
    | python -c "
import sys, json
runs = json.load(sys.stdin).get('check_runs', [])
print(sum(1 for r in runs if r['status'] != 'completed'))")
  echo "poll $i: $PENDING still running"
  [ "$PENDING" = "0" ] && break
  sleep 30
done
```

Ten minutes maximum. If it is still running after that, say so rather than looping: some
suites take an hour, and a silent agent looks identical to a stuck one.

## 5. Fix what CI says

```bash
gh run list --branch "$(git branch --show-current)" --limit 5
gh run view <RUN_ID> --log-failed
```

With curl, the logs come back as a redirect to a zip, so `-L` is not optional:

```bash
curl -sL -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  "https://api.github.com/repos/$OWNER/$REPO/actions/runs/$RUN_ID/logs" \
  -o /tmp/ci-logs.zip
unzip -o /tmp/ci-logs.zip -d /tmp/ci-logs && grep -ri "error\|failed" /tmp/ci-logs | head -40
```

Then: read the real error, reproduce it LOCALLY, fix it, run the local check, push once.

**Reproduce locally before pushing a fix.** Pushing a guess and waiting for CI is a
fifteen-minute compile of a hypothesis you could have tested in ten seconds. If the failure
genuinely only happens in CI, that difference is the bug: environment, version, ordering.

Stop after three attempts. Three failed fixes means the diagnosis is wrong, not that the
fourth will land. Say what you tried, paste the failing output, and hand it back.

## 6. Merge

```bash
gh pr merge --squash --delete-branch          # now
gh pr merge --auto --squash --delete-branch   # when checks pass
```

With curl:

```bash
curl -s -X PUT -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  "https://api.github.com/repos/$OWNER/$REPO/pulls/$PR_NUMBER/merge" \
  -d '{"merge_method": "squash"}'
```

`merge_method` is `merge`, `squash` or `rebase`. Match the repository's habit: read
`git log --oneline -20` on the default branch. A merge commit in a history of clean squashes
is noise somebody has to explain.

Auto-merge has no REST endpoint; it is GraphQL only:

```bash
NODE_ID=$(curl -s -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  "https://api.github.com/repos/$OWNER/$REPO/pulls/$PR_NUMBER" \
  | python -c "import sys, json; print(json.load(sys.stdin)['node_id'])")

python - "$NODE_ID" <<'PY' > /tmp/automerge.json
import json, sys
print(json.dumps({
    "query": "mutation($id: ID!) {"
             " enablePullRequestAutoMerge(input: {pullRequestId: $id, mergeMethod: SQUASH})"
             " { clientMutationId } }",
    "variables": {"id": sys.argv[1]},
}))
PY

curl -s -X POST -H "Authorization: Bearer ${GITHUB_TOKEN}" \n  https://api.github.com/graphql --data @/tmp/automerge.json
```

Then clean up:

```bash
git switch main && git pull --ff-only origin main
git branch -d "$BRANCH"
```

`-d` refuses to delete a branch that is not merged. Never reach for `-D` to make that
message go away: it is telling you the work is not where you think it is.

## Quick reference

| Action | gh | REST |
|---|---|---|
| My open PRs | `gh pr list --author @me` | `GET /repos/O/R/pulls?state=open` |
| Diff | `gh pr diff` | `git diff main...HEAD` |
| Comment | `gh pr comment N --body "..."` | `POST /repos/O/R/issues/N/comments` |
| Request review | `gh pr edit N --add-reviewer u` | `POST /repos/O/R/pulls/N/requested_reviewers` |
| Close | `gh pr close N` | `PATCH /repos/O/R/pulls/N {"state":"closed"}` |
| Check out a PR | `gh pr checkout N` | `git fetch origin pull/N/head:pr-N` |

Note `main...HEAD`, three dots: that is the diff against the merge base, which is what the
pull request shows. Two dots compares the tips and includes everything that landed on
`main` since you branched.

## Traps

- **`gh pr create` before pushing** fails with a message about the branch not existing on
  the remote. Push first.
- **A PR body built by string interpolation** breaks on the first quote or backtick.
  Serialise it with a tool.
- **Branch protection rejects the merge**, not the token. Read the message: required
  reviews and required checks are the usual causes, and neither is fixed by a new token.
- **Squashing loses the co-authors** in a multi-author branch unless the trailers are
  carried into the squash message.
- **A push to a protected default branch** is rejected outright. That is the protection
  working. Open a PR.
- **`--delete-branch` deletes it locally too**, so run it from a different branch or expect
  to be moved.

## Failure modes

**`gh` works and `git push` asks for a password.** `gh auth setup-git` was never run. See
`github-auth`.

**422 Validation Failed on PR creation.** Almost always one of: a PR already exists for
this head branch, `head` and `base` are identical, or the body was malformed JSON. Read the
`errors` array in the response; it names the field.

**CI never finishes.** You are polling `/status` on an Actions repository. Use
`/check-runs`. See section 4.

**A check is red but the logs look clean.** The failing step is a different job. List the
runs and open the one whose `conclusion` is `failure`, not the most recent one.

**Merge returns 405 Method Not Allowed.** The PR is not mergeable: conflicts, a required
check pending, or a required review missing. `gh pr view --json mergeable,mergeStateStatus`
says which.

**The branch will not delete after merging.** It was merged by squash, so git cannot see
its commits in `main`. Confirm the PR is merged on GitHub, then delete it with `-D`, which
is the one case where that is correct.
