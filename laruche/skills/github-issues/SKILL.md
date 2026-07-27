---
type: skill
name: github-issues
description: Create, search, triage and label GitHub issues.
---

# GitHub issues

Open an issue somebody can act on, find the ones that already exist, and keep the backlog
from turning into a landfill. `gh` where it exists, `curl` where it does not.

Read `github-auth` first if anything comes back 401 or 404. Run the shell blocks through
`shell_exec`.

## Setup for the curl path

```bash
OWNER_REPO=$(git remote get-url origin | sed -E 's|.*github\.com[:/]||; s|\.git$||')
OWNER=${OWNER_REPO%%/*}
REPO=${OWNER_REPO##*/}
AUTH="Authorization: Bearer ${GITHUB_TOKEN}"
```

`${GITHUB_TOKEN}` comes from the LaRuche vault. Never echo it. With `gh`, none of this is
needed, and `--repo OWNER/REPO` works from any directory.

## The one that catches everyone

**`GET /issues` returns pull requests too.** In GitHub's data model a pull request IS an
issue with extra fields, so the issues endpoint hands back both. Every count, every
listing and every bulk operation must filter them out:

```python
if "pull_request" not in item:   # a real issue
```

Skip that check and "close all stale issues" closes people's open pull requests. `gh issue
list` filters them for you; the REST API does not.

## Search before creating

The most useful thing you can do with an issue tracker is not open a duplicate.

```bash
gh issue list --search "login redirect next parameter" --state all
gh issue list --state open --label bug
gh issue list --assignee @me
gh issue view 42
```

```bash
curl -s -H "$AUTH" \
  "https://api.github.com/search/issues?q=$(python -c "
import urllib.parse, sys; print(urllib.parse.quote_plus(sys.argv[1]))" \
  "login redirect repo:$OWNER/$REPO state:all")" \
  | python -c "
import sys, json
for item in json.load(sys.stdin)['items']:
    print(f\"#{item['number']}  {item['state']:6}  {item['title']}\")"
```

Search `--state all`, not just open. A closed issue explaining why something was rejected
is the most valuable result you can get, and it is invisible in the default view.

URL-encode the query. A raw `#` or `:` in a query string silently truncates it, and the
search then returns confident nonsense.

## Writing one worth acting on

An issue is read by someone with no context, possibly months later, possibly you. It needs
to be reproducible from its own text.

```bash
gh issue create --title "Login redirect drops the ?next= parameter" --body "$(cat <<'BODY'
## What happens
Signing in from `/settings?next=/billing` lands on `/dashboard`, not `/billing`.

## Expected
Redirect to the path in `next`, when it is a same-origin relative path.

## Steps
1. sign out
2. open `/settings?next=/billing`
3. sign in with any account

## Environment
v2.4.1, Firefox 128, Windows 11. Also reproduced on Chrome 129.

## Notes
`auth/middleware.py:88` reads `next` before the session is rebuilt, so it reads an
empty session and falls back to the default.
BODY
)" --label bug
```

The title is the symptom in one line, not "login broken". The steps must start from a
state anyone can reach. Version and platform belong in the issue, not in a follow-up
question three days later.

Bundled starting points: `templates/bug-report.md` and `templates/feature-request.md`.

## Labels, assignment, comments

```bash
gh issue edit 42 --add-label "priority:high,bug" --remove-label needs-triage
gh issue edit 42 --add-assignee @me
gh issue comment 42 --body "Root cause is the middleware ordering, fix in progress."
```

```bash
curl -s -X POST -H "$AUTH" \
  "https://api.github.com/repos/$OWNER/$REPO/issues/42/labels" \
  -d '{"labels": ["priority:high", "bug"]}'

curl -s -X DELETE -H "$AUTH" \
  "https://api.github.com/repos/$OWNER/$REPO/issues/42/labels/needs-triage"

curl -s -X POST -H "$AUTH" \
  "https://api.github.com/repos/$OWNER/$REPO/issues/42/assignees" \
  -d '{"assignees": ["someone"]}'
```

Use labels the repository already has. Invented ones fragment the taxonomy and nobody
filters on them:

```bash
gh label list
```

An assignee must have write access. Assigning someone who does not is silently ignored:
the API returns 201 and the issue stays unassigned.

## Closing

```bash
gh issue close 42 --reason completed     # or: not planned
gh issue reopen 42
```

```bash
curl -s -X PATCH -H "$AUTH" \
  "https://api.github.com/repos/$OWNER/$REPO/issues/42" \
  -d '{"state": "closed", "state_reason": "completed"}'
```

`state_reason` is `completed` or `not_planned`, with an underscore in the API and a space
in the `gh` flag. Setting `state` without a reason leaves the issue closed with no
explanation, which reads as abandoned rather than decided.

Say why in a comment before closing. A closed issue with no final comment is the single
most common source of the same bug being reported again.

## Linking an issue to the work

In the pull request body, these close the issue on merge:

```
Closes #42     Fixes #42     Resolves #42
```

They only work when the PR targets the DEFAULT branch. Merging into `develop` with
`Closes #42` closes nothing, and everyone assumes it did.

Start a branch from an issue, which names it and links it in one step:

```bash
gh issue develop 42 --checkout
```

## Triage

1. `gh issue list --label needs-triage --state open`
2. Read the issue in full, including comments. The last comment often contains the actual
   reproduction.
3. Is it reproducible from the text alone? If not, ask for exactly what is missing, one
   specific question, and leave `needs-triage` on.
4. Label: kind, priority, area. Only labels that already exist.
5. Assign only when the owner is genuinely known. An unowned issue is honest; a
   misassigned one is invisible.
6. Remove `needs-triage`.

## Bulk operations

Bulk edits are irreversible in practice and hit real people's notifications. **List what
would be touched first, show it to the user, and only then act.**

```bash
# 1. see what matches, and confirm it is only issues
gh issue list --label wontfix --state open --json number,title

# 2. only after the user agrees
gh issue list --label wontfix --state open --json number --jq '.[].number' \
  | xargs -I{} gh issue close {} --reason "not planned"
```

With curl, filter the pull requests out explicitly:

```bash
curl -s -H "$AUTH" \
  "https://api.github.com/repos/$OWNER/$REPO/issues?labels=wontfix&state=open&per_page=100" \
  | python -c "
import sys, json
for item in json.load(sys.stdin):
    if 'pull_request' not in item:
        print(item['number'])"
```

## Quick reference

| Action | gh | REST |
|---|---|---|
| List | `gh issue list` | `GET /repos/O/R/issues` |
| View | `gh issue view N` | `GET /repos/O/R/issues/N` |
| Create | `gh issue create` | `POST /repos/O/R/issues` |
| Label | `gh issue edit N --add-label x` | `POST /repos/O/R/issues/N/labels` |
| Assign | `gh issue edit N --add-assignee u` | `POST /repos/O/R/issues/N/assignees` |
| Comment | `gh issue comment N --body "..."` | `POST /repos/O/R/issues/N/comments` |
| Close | `gh issue close N` | `PATCH /repos/O/R/issues/N` |
| Search | `gh issue list --search "..."` | `GET /search/issues?q=...` |

## Traps

- **Pagination stops at 30.** The default `per_page` is 30, the maximum is 100, and
  beyond that you must follow the `Link` header. "There are 30 open issues" is usually
  the page size, not the count.
- **Search is rate limited separately** and more tightly than the rest of the API, and it
  is eventually consistent: an issue created seconds ago may not be findable yet.
- **`--label "a,b"` on `gh` means two labels**, but a label containing a comma cannot be
  expressed that way at all. Use repeated `--add-label` flags.
- **Issue numbers and pull request numbers share one sequence.** `#42` is either. Do not
  assume from the number alone.
- **Closing is not deleting**, and nothing here deletes. Deleting an issue requires admin
  rights and the web interface, deliberately.

## Failure modes

**404 on an issue that exists in the browser.** Either the token lacks `repo` for a
private repository, or `$OWNER/$REPO` was parsed from a remote that is not GitHub. Echo
`$OWNER_REPO` and check it.

**422 Validation Failed on create.** A label or an assignee that does not exist. The
`errors` array names the field. Create the label first, or drop it.

**The listing is full of pull requests.** The `pull_request` filter is missing. See the top
of this file.

**A comment posts with literal `\n` in it.** The body went through shell interpolation.
Use a here-document with `gh`, or build the JSON with a tool for curl.

**`Closes #42` did not close the issue.** The pull request targeted a non-default branch,
or the keyword was in a comment rather than the PR description. Only the description
counts.
