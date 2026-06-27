---
type: skill
name: github-code-review
description: "Review PRs: diffs, inline comments via gh or REST."
version: 1.2.0
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec, file_read, file_list]
metadata:
  laruche:
    tags: [GitHub, Code-Review, Pull-Requests, Git, Quality]
    related_skills: [github-auth, github-pr-workflow]
---

# GitHub Code Review

Review local changes before pushing, or review open PRs on GitHub.

## Prerequisites

- Authenticated with GitHub (see `github-auth` skill)
- Inside a git repository
- `${GITHUB_TOKEN}` available in secrets vault (for curl fallback)

### Auth Detection

```bash
if command -v gh &>/dev/null && gh auth status &>/dev/null; then
  AUTH="gh"
else
  AUTH="curl"
fi

REMOTE_URL=$(git remote get-url origin)
OWNER_REPO=$(echo "$REMOTE_URL" | sed -E 's|.*github\.com[:/]||; s|\.git$||')
OWNER=$(echo "$OWNER_REPO" | cut -d/ -f1)
REPO=$(echo "$OWNER_REPO" | cut -d/ -f2)
```

---

## 1. Review Local Changes (Pre-Push)

Pure `git` — no API needed.

```bash
git diff main...HEAD --stat          # scope overview
git log main..HEAD --oneline         # commit list
git diff main...HEAD                 # full diff
git diff main...HEAD -- src/file.py  # single file
```

**Quick issue scan:**

```bash
# Debug statements, TODOs
git diff main...HEAD | grep -n "print(\|console\.log\|TODO\|FIXME\|HACK\|debugger"

# Credential patterns
git diff main...HEAD | grep -in "password\|secret\|api_key\|token.*=\|private_key"

# Merge conflict markers
git diff main...HEAD | grep -n "<<<<<<\|>>>>>>\|======="
```

Use `file_read` on affected files for full context around flagged lines.

**Review output format:**

```
## Code Review Summary

### Critical
- **src/auth.py:45** — SQL injection: raw user input in query. Use parameterized queries.

### Warnings
- **src/models/user.py:23** — Password stored plaintext. Use bcrypt/argon2.

### Suggestions
- **src/utils/helpers.py:8** — Duplicates logic in src/core/utils.py:34.

### Looks Good
- Clean middleware separation; good happy-path test coverage.
```

---

## 2. Review a Pull Request on GitHub

### Gather PR context

**With gh (preferred):**
```bash
gh pr view $PR_NUMBER
gh pr diff $PR_NUMBER --name-only
gh pr checks $PR_NUMBER
```

**With curl (fallback — uses `${GITHUB_TOKEN}` from secrets vault):**
```bash
curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/$OWNER/$REPO/pulls/$PR_NUMBER \
  | python3 -c "import sys,json; pr=json.load(sys.stdin); print(pr['title'], pr['user']['login'], pr['head']['ref'])"

curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/$OWNER/$REPO/pulls/$PR_NUMBER/files \
  | python3 -c "import sys,json; [print(f['status'],f['filename']) for f in json.load(sys.stdin)]"
```

### Check out PR locally

```bash
gh pr checkout $PR_NUMBER
# or manually:
git fetch origin pull/$PR_NUMBER/head:pr-$PR_NUMBER && git checkout pr-$PR_NUMBER
git diff main...HEAD   # diff against base
# Use file_read, file_list, shell_exec for tests
```

### Leave comments

**General comment:**
```bash
gh pr comment $PR_NUMBER --body "Overall LGTM, a few inline notes below."
# curl fallback:
curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/$OWNER/$REPO/issues/$PR_NUMBER/comments \
  -d '{"body": "Overall LGTM, a few inline notes below."}'
```

**Single inline comment:**
```bash
HEAD_SHA=$(gh pr view $PR_NUMBER --json headRefOid --jq '.headRefOid')
gh api repos/$OWNER/$REPO/pulls/$PR_NUMBER/comments \
  --method POST \
  -f body="Use a list comprehension here." \
  -f path="src/auth/login.py" \
  -f commit_id="$HEAD_SHA" \
  -f line=45 \
  -f side="RIGHT"
```

`line` = line number in the **new** file. For deleted lines use `"side": "LEFT"`.

### Submit a formal review

```bash
gh pr review $PR_NUMBER --approve --body "LGTM!"
gh pr review $PR_NUMBER --request-changes --body "See inline comments."
gh pr review $PR_NUMBER --comment --body "A few suggestions."
```

**Atomic review with multiple inline comments (curl):**
```bash
HEAD_SHA=$(curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/$OWNER/$REPO/pulls/$PR_NUMBER \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['head']['sha'])")

curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/$OWNER/$REPO/pulls/$PR_NUMBER/reviews \
  -d "{
    \"commit_id\": \"$HEAD_SHA\",
    \"event\": \"REQUEST_CHANGES\",
    \"body\": \"## Review\n2 issues, 1 suggestion.\",
    \"comments\": [
      {\"path\": \"src/auth.py\",   \"line\": 45, \"body\": \"SQL injection — use parameterized queries.\"},
      {\"path\": \"src/models.py\", \"line\": 23, \"body\": \"Hash passwords before storing.\"},
      {\"path\": \"src/utils.py\",  \"line\": 8,  \"body\": \"Duplicates logic in core/utils.py:34.\"}
    ]
  }"
```

Event values: `"APPROVE"` | `"REQUEST_CHANGES"` | `"COMMENT"`

---

## 3. Review Checklist

| Category | Key checks |
|---|---|
| **Correctness** | Does it do what it claims? Edge cases (null, empty, large data, concurrency)? Errors handled? |
| **Security** | No hardcoded secrets/keys; input validation; no SQLi/XSS/path traversal; auth checks in place |
| **Code Quality** | Clear naming; no premature abstraction; DRY; single-responsibility functions |
| **Testing** | New paths tested? Happy + error paths? Tests readable? |
| **Performance** | No N+1 queries; no blocking ops in async; caching where appropriate |
| **Docs** | Public APIs documented; non-obvious logic explained; README updated if behavior changed |

**Decision rule:**
- **Approve** — no critical/warning issues (minor suggestions OK)
- **Request Changes** — any critical or warning-level issue
- **Comment** — observations only, nothing blocking (good for drafts)

---

## 4. End-to-End PR Review Workflow

```
1. Auth detection (top of skill) — prefer gh, fall back to ${GITHUB_TOKEN}
2. gh pr view $N  →  scope, description, CI status
3. gh pr checkout $N  (or git fetch origin pull/$N/head:pr-$N && git checkout pr-$N)
4. git diff main...HEAD --name-only  →  file list
5. Per file: git diff main...HEAD -- <file>  +  file_read for context
6. Run tests/linter if applicable:
      shell_exec("python -m pytest 2>&1 | tail -20")
      shell_exec("ruff check . 2>&1 | head -30")
7. Apply checklist (Section 3), collect findings
8. Post atomic review (gh pr review or curl /reviews with inline comments)
9. Post summary top-level comment (gh pr comment)
10. Cleanup: git checkout main && git branch -D pr-$N
```
