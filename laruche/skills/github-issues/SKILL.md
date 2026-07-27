---
type: skill
name: github-issues
description: Create, search, triage and label GitHub issues.
---

# GitHub Issues Management

Create, search, triage, and manage GitHub issues. Prefer `gh`; fall back to `curl` when unavailable.

## Prerequisites

- Authenticated with GitHub (see `github-auth` skill)
- Inside a git repo with a GitHub remote, or pass `--repo OWNER/REPO` explicitly

### Setup (curl fallback only)

```bash
REMOTE_URL=$(git remote get-url origin)
OWNER_REPO=$(echo "$REMOTE_URL" | sed -E 's|.*github\.com[:/]||; s|\.git$||')
OWNER=$(echo "$OWNER_REPO" | cut -d/ -f1)
REPO=$(echo "$OWNER_REPO" | cut -d/ -f2)
AUTH_HEADER="Authorization: token ${GITHUB_TOKEN}"
```

> `${GITHUB_TOKEN}` is injected from the LaRuche secrets vault at execution time.

---

## 1. Viewing Issues

```bash
# gh
gh issue list
gh issue list --state open --label "bug"
gh issue list --assignee @me
gh issue list --search "authentication error" --state all
gh issue view 42

# curl
curl -s -H "$AUTH_HEADER" \
  "https://api.github.com/repos/$OWNER/$REPO/issues?state=open&per_page=20" \
  | python3 -c "
import sys, json
for i in json.load(sys.stdin):
    if 'pull_request' not in i:
        labels = ', '.join(l['name'] for l in i['labels'])
        print(f\"#{i['number']:5}  {i['state']:6}  {labels:30}  {i['title']}\")"

# curl - search
curl -s -H "$AUTH_HEADER" \
  "https://api.github.com/search/issues?q=authentication+error+repo:$OWNER/$REPO" \
  | python3 -c "
import sys, json
for i in json.load(sys.stdin)['items']:
    print(f\"#{i['number']}  {i['state']:6}  {i['title']}\")"
```

> **Pitfall:** `/issues` returns PRs too - always filter with `'pull_request' not in i`.

---

## 2. Creating Issues

```bash
# gh
gh issue create \
  --title "Login redirect ignores ?next= parameter" \
  --body "$(cat <<'EOF'
## Description
After login, users always land on /dashboard instead of the requested page.

## Steps to Reproduce
1. Navigate to /settings while logged out
2. Get redirected to /login?next=/settings
3. Log in

## Expected Behavior
Respect the ?next= query parameter.
EOF
)" \
  --label "bug,backend" \
  --assignee "username"

# curl
curl -s -X POST -H "$AUTH_HEADER" \
  https://api.github.com/repos/$OWNER/$REPO/issues \
  -d '{
    "title": "Login redirect ignores ?next= parameter",
    "body": "## Description\nAfter login, users land on /dashboard.\n\n## Steps to Reproduce\n1. Go to /settings while logged out\n2. Log in → lands on /dashboard instead of /settings\n\n## Expected Behavior\nRespect the ?next= parameter.",
    "labels": ["bug", "backend"],
    "assignees": ["username"]
  }'
```

### Issue Templates

**Bug report:**
```
## Description
<what's happening>

## Steps to Reproduce
1. <step>

## Expected / Actual Behavior
Expected: <what should happen>
Actual:   <what actually happens>

## Environment
- OS: <os>  Version: <version>
```

**Feature request:**
```
## Feature Description
<what you want>

## Motivation
<why this is useful>

## Proposed Solution / Alternatives
<how it could work; other approaches considered>
```

---

## 3. Managing Issues

### Labels

```bash
# gh
gh issue edit 42 --add-label "priority:high,bug"
gh issue edit 42 --remove-label "needs-triage"

# curl - add
curl -s -X POST -H "$AUTH_HEADER" \
  https://api.github.com/repos/$OWNER/$REPO/issues/42/labels \
  -d '{"labels": ["priority:high", "bug"]}'

# curl - remove
curl -s -X DELETE -H "$AUTH_HEADER" \
  https://api.github.com/repos/$OWNER/$REPO/issues/42/labels/needs-triage

# List repo labels
curl -s -H "$AUTH_HEADER" \
  https://api.github.com/repos/$OWNER/$REPO/labels \
  | python3 -c "
import sys, json
for l in json.load(sys.stdin): print(f\"  {l['name']:30}  {l.get('description','')}\")"
```

### Assignment & Comments

```bash
# gh
gh issue edit 42 --add-assignee @me
gh issue comment 42 --body "Root cause: auth middleware. Fix in progress."

# curl - assign
curl -s -X POST -H "$AUTH_HEADER" \
  https://api.github.com/repos/$OWNER/$REPO/issues/42/assignees \
  -d '{"assignees": ["username"]}'

# curl - comment
curl -s -X POST -H "$AUTH_HEADER" \
  https://api.github.com/repos/$OWNER/$REPO/issues/42/comments \
  -d '{"body": "Root cause: auth middleware. Fix in progress."}'
```

### Close / Reopen

```bash
# gh
gh issue close 42 --reason "completed"   # or "not planned"
gh issue reopen 42

# curl
curl -s -X PATCH -H "$AUTH_HEADER" \
  https://api.github.com/repos/$OWNER/$REPO/issues/42 \
  -d '{"state": "closed", "state_reason": "completed"}'   # or "not_planned"
```

### Link Issues to PRs

Include in the PR body to auto-close on merge:
```
Closes #42    Fixes #42    Resolves #42
```

Create a branch directly from an issue:
```bash
gh issue develop 42 --checkout          # gh (preferred)
git checkout -b fix/issue-42-login-redirect   # manual fallback
```

---

## 4. Triage Workflow

1. **List untriaged:**
   ```bash
   gh issue list --label "needs-triage" --state open
   ```
2. **Read each issue** - view details, understand scope.
3. **Apply labels and priority** (see §3).
4. **Assign** if owner is clear.
5. **Comment** with triage notes or requests for more info.
6. **Remove** `needs-triage` once processed.

---

## 5. Bulk Operations

```bash
# Close all "wontfix" issues - gh
gh issue list --label "wontfix" --json number --jq '.[].number' | \
  xargs -I {} gh issue close {} --reason "not planned"

# Close all "wontfix" issues - curl
curl -s -H "$AUTH_HEADER" \
  "https://api.github.com/repos/$OWNER/$REPO/issues?labels=wontfix&state=open" \
  | python3 -c "import sys,json; [print(i['number']) for i in json.load(sys.stdin)]" \
  | while read num; do
      curl -s -X PATCH -H "$AUTH_HEADER" \
        https://api.github.com/repos/$OWNER/$REPO/issues/$num \
        -d '{"state": "closed", "state_reason": "not_planned"}'
      echo "Closed #$num"
    done
```

---

## Quick Reference

| Action | gh | REST endpoint |
|--------|-----|--------------|
| List issues | `gh issue list` | `GET /repos/{o}/{r}/issues` |
| View issue | `gh issue view N` | `GET /repos/{o}/{r}/issues/N` |
| Create issue | `gh issue create ...` | `POST /repos/{o}/{r}/issues` |
| Add labels | `gh issue edit N --add-label ...` | `POST /repos/{o}/{r}/issues/N/labels` |
| Assign | `gh issue edit N --add-assignee ...` | `POST /repos/{o}/{r}/issues/N/assignees` |
| Comment | `gh issue comment N --body ...` | `POST /repos/{o}/{r}/issues/N/comments` |
| Close | `gh issue close N` | `PATCH /repos/{o}/{r}/issues/N` |
| Search | `gh issue list --search "..."` | `GET /search/issues?q=...` |
