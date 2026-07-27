---
type: skill
name: github-pr-workflow
description: >-
  GitHub PR lifecycle: branch, commit, open, CI, merge.
---

# GitHub Pull Request Workflow

Manages the full PR lifecycle. Each section shows `gh` first, then `git` + `curl` fallback for machines without it. Run all bash blocks via `shell_exec`.

## Prerequisites

- Authenticated with GitHub (see `github-auth` skill); `GITHUB_TOKEN` available as `${GITHUB_TOKEN}`
- Inside a git repository with a GitHub remote

### Auth Detection (run once, reuse `$AUTH` throughout)

```bash
if command -v gh &>/dev/null && gh auth status &>/dev/null; then
  AUTH="gh"
else
  AUTH="git"
  # ${GITHUB_TOKEN} is injected by LaRuche secrets vault at execution time
fi
echo "Using: $AUTH"
```

### Extract Owner/Repo (required for curl commands)

```bash
REMOTE_URL=$(git remote get-url origin)
OWNER_REPO=$(echo "$REMOTE_URL" | sed -E 's|.*github\.com[:/]||; s|\.git$||')
OWNER=$(echo "$OWNER_REPO" | cut -d/ -f1)
REPO=$(echo "$OWNER_REPO" | cut -d/ -f2)
```

---

## 1. Branch Creation

```bash
git fetch origin
git checkout main && git pull origin main
git checkout -b feat/add-user-authentication
```

Branch naming: `feat/`, `fix/`, `refactor/`, `docs/`, `ci/` + description.

## 2. Making Commits

Use `file_write` / `file_read` to edit files, then commit:

```bash
git add src/auth.py src/models/user.py tests/test_auth.py
git commit -m "feat: add JWT-based user authentication

- Add login/register endpoints
- Add User model with password hashing
- Add auth middleware for protected routes
- Add unit tests for auth flow"
```

Conventional Commits format: `type(scope): short description` - types: `feat`, `fix`, `refactor`, `docs`, `test`, `ci`, `chore`, `perf`.

## 3. Push and Create PR

```bash
git push -u origin HEAD
```

**With gh:**

```bash
gh pr create \
  --title "feat: add JWT-based user authentication" \
  --body "## Summary
- Adds login and register API endpoints
- JWT token generation and validation

## Test Plan
- [ ] Unit tests pass

Closes #42"
```

Options: `--draft`, `--reviewer user1,user2`, `--label "enhancement"`, `--base develop`

**With curl:**

```bash
BRANCH=$(git branch --show-current)
curl -s -X POST \
  -H "Authorization: token ${GITHUB_TOKEN}" \
  -H "Accept: application/vnd.github.v3+json" \
  https://api.github.com/repos/$OWNER/$REPO/pulls \
  -d "{\"title\": \"feat: add JWT-based user authentication\",
       \"body\": \"## Summary\nAdds login and register API endpoints.\n\nCloses #42\",
       \"head\": \"$BRANCH\",
       \"base\": \"main\"}"
# Save the returned "number" field as PR_NUMBER for later steps.
# Add "draft": true to the JSON body to create as draft.
```

## 4. Monitor CI Status

**With gh:**

```bash
gh pr checks          # one-shot
gh pr checks --watch  # polls every 10s until all checks finish
```

**With curl:**

```bash
SHA=$(git rev-parse HEAD)

# Combined commit status
curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/$OWNER/$REPO/commits/$SHA/status \
  | python3 -c "
import sys, json; data = json.load(sys.stdin)
print(f\"Overall: {data['state']}\")
for s in data.get('statuses', []):
    print(f\"  {s['context']}: {s['state']} - {s.get('description','')}\")"

# GitHub Actions check runs
curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/$OWNER/$REPO/commits/$SHA/check-runs \
  | python3 -c "
import sys, json
for cr in json.load(sys.stdin).get('check_runs', []):
    print(f\"  {cr['name']}: {cr['status']} / {cr['conclusion'] or 'pending'}\")"
```

**Poll until complete (curl):**

```bash
SHA=$(git rev-parse HEAD)
for i in $(seq 1 20); do
  STATUS=$(curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
    https://api.github.com/repos/$OWNER/$REPO/commits/$SHA/status \
    | python3 -c "import sys,json; print(json.load(sys.stdin)['state'])")
  echo "Check $i: $STATUS"
  [ "$STATUS" = "success" ] || [ "$STATUS" = "failure" ] || [ "$STATUS" = "error" ] && break
  sleep 30
done
```

## 5. Auto-Fix CI Failures

### Get Failure Logs

**With gh:**

```bash
gh run list --branch $(git branch --show-current) --limit 5
gh run view <RUN_ID> --log-failed
```

**With curl:**

```bash
BRANCH=$(git branch --show-current)
curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
  "https://api.github.com/repos/$OWNER/$REPO/actions/runs?branch=$BRANCH&per_page=5" \
  | python3 -c "
import sys, json
for r in json.load(sys.stdin)['workflow_runs']:
    print(f\"Run {r['id']}: {r['name']} - {r['conclusion'] or r['status']}\")"

# Download failed run logs
RUN_ID=<run_id>
curl -s -L -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/$OWNER/$REPO/actions/runs/$RUN_ID/logs \
  -o /tmp/ci-logs.zip
unzip -o /tmp/ci-logs.zip -d /tmp/ci-logs && cat /tmp/ci-logs/*.txt
```

### Fix and Push

Use `file_read` to inspect files, `file_write` to apply fixes, then:

```bash
git add <fixed_files>
git commit -m "fix: resolve CI failure in <check_name>"
git push
```

### Auto-Fix Loop

1. Check CI status → identify failures
2. Read logs → understand the error
3. `file_read` + `file_write` → fix the code
4. `git add . && git commit -m "fix: ..." && git push`
5. Wait for CI → re-check (Section 4)
6. Repeat up to 3 times, then surface to user if still failing

## 6. Merge

**With gh:**

```bash
gh pr merge --squash --delete-branch         # immediate merge
gh pr merge --auto --squash --delete-branch  # merge when all checks pass
```

**With curl:**

```bash
PR_NUMBER=<number>
curl -s -X PUT \
  -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/$OWNER/$REPO/pulls/$PR_NUMBER/merge \
  -d "{\"merge_method\": \"squash\",
       \"commit_title\": \"feat: add user authentication (#$PR_NUMBER)\"}"

# Delete remote branch and clean up locally
BRANCH=$(git branch --show-current)
git push origin --delete $BRANCH
git checkout main && git pull origin main
git branch -d $BRANCH
```

Merge methods: `"merge"`, `"squash"`, `"rebase"`.

**Enable auto-merge via GraphQL (curl only - REST doesn't support it):**

```bash
PR_NODE_ID=$(curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/$OWNER/$REPO/pulls/$PR_NUMBER \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['node_id'])")

curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/graphql \
  -d "{\"query\": \"mutation { enablePullRequestAutoMerge(input: {pullRequestId: \\\"$PR_NODE_ID\\\", mergeMethod: SQUASH}) { clientMutationId } }\"}"
```

## Quick Reference

| Action | gh | curl |
|--------|-----|------|
| List my PRs | `gh pr list --author @me` | `GET /repos/$OWNER/$REPO/pulls?state=open` |
| View diff | `gh pr diff` | `git diff main...HEAD` |
| Add comment | `gh pr comment N --body "..."` | `POST /repos/$OWNER/$REPO/issues/N/comments` |
| Request review | `gh pr edit N --add-reviewer user` | `POST /repos/$OWNER/$REPO/pulls/N/requested_reviewers` |
| Close PR | `gh pr close N` | `PATCH /repos/$OWNER/$REPO/pulls/N {"state":"closed"}` |
| Check out PR | `gh pr checkout N` | `git fetch origin pull/N/head:pr-N && git checkout pr-N` |
