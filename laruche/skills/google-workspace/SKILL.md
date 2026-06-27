---
type: skill
name: google-workspace
description: "Gmail, Calendar, Drive, Sheets, Docs, Contacts via OAuth2."
version: 1.1.0
author: third-party
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec, file_write, skill_view]
scripts:
  - scripts/setup.py
  - scripts/google_api.py
  - scripts/gws_bridge.py
required_credential_files:
  - path: google_token.json
    description: Google OAuth2 token (created by setup script)
  - path: google_client_secret.json
    description: OAuth2 client credentials (from Google Cloud Console)
metadata:
  laruche:
    tags: [Google, Gmail, Calendar, Drive, Sheets, Docs, Contacts, Email, OAuth]
    homepage: https://developers.google.com/workspace
    related_skills: [himalaya]
---

# Google Workspace

Gmail, Calendar, Drive, Contacts, Sheets, and Docs via OAuth2. When `gws` is installed it is used as the execution backend; otherwise the bundled Python scripts handle everything.

**Script shorthands** (set once per session):

```bash
GSETUP="python SKILL_DIR/scripts/setup.py"
GAPI="python SKILL_DIR/scripts/google_api.py"
```

`SKILL_DIR` = absolute path to the directory containing this SKILL.md.
Tokens are stored in the gws CLI config directory; pass any required API keys/secrets as LaRuche secrets (`${NAME}`).

## References

- `references/gmail-search-syntax.md` — Gmail search operators (`is:unread`, `from:`, `newer_than:`, etc.)

## First-Time Setup

### Step 0 — Check existing auth

```bash
$GSETUP --check
```

Prints `AUTHENTICATED` → skip to Usage. Otherwise continue.

### Step 1 — Triage

- **Email only?** Use the `himalaya` skill instead (Gmail App Password, no Cloud project needed).
- **Email + Calendar / full Workspace?** Continue below. Pass `--services email,calendar` or `--services all` to limit OAuth scopes.
- **Advanced Protection account?** Workspace admin must allowlist the OAuth client ID before Step 4 works.

### Step 2 — Create OAuth credentials (one-time, ~5 min)

1. Create/select a project: <https://console.cloud.google.com/projectselector2/home/dashboard>
2. Enable APIs at <https://console.cloud.google.com/apis/library>: Gmail, Calendar, Drive, Sheets, Docs, People
3. Create credentials: <https://console.cloud.google.com/apis/credentials> → Create Credentials → OAuth 2.0 Client ID → Desktop app
4. If app is in Testing, add the user as a test user: <https://console.cloud.google.com/auth/audience>
5. Download the JSON → ask user for the path

```bash
$GSETUP --client-secret /path/to/client_secret.json
```

If the user provides raw client ID / secret values instead of a file, construct a valid Desktop OAuth JSON, save it locally (e.g. `~/Downloads/client_secret.json`), then run `--client-secret` against that file.

### Step 3 — Get authorization URL

```bash
$GSETUP --auth-url --services email,calendar --format json
# or: --services calendar,drive,sheets,docs
# or: --services all
```

- Extract the `auth_url` field and send it to the user.
- Tell the user: the browser will fail on `http://localhost:1` after approval — that is expected. Copy the entire redirected URL from the address bar.
- If the user gets `Error 403: access_denied` → send them to <https://console.cloud.google.com/auth/audience> to add themselves as a test user.

### Step 4 — Exchange the code

```bash
$GSETUP --auth-code "THE_URL_OR_CODE_THE_USER_PASTED" --format json
```

Accepts the full redirect URL or just the code string. If `--auth-code` fails (expired/already used), the response includes a fresh `fresh_auth_url` — send it to the user and retry.

### Step 5 — Verify

```bash
$GSETUP --check
```

Should print `AUTHENTICATED`. Token auto-refreshes from this point on.

**Notes:**
- Pending OAuth state is stored in the token directory until exchange completes.
- If `gws` is installed, `google_api.py` points it at the resolved token path — no separate `gws auth login` needed.
- Revoke: `$GSETUP --revoke`
- Missing deps: `$GSETUP --install-deps`

## Usage

### Gmail

```bash
# Search (returns JSON: id, from, subject, date, snippet, labels)
$GAPI gmail search "is:unread" --max 10
$GAPI gmail search "from:boss@company.com newer_than:1d"
$GAPI gmail search "has:attachment filename:pdf newer_than:7d"

# Read full message
$GAPI gmail get MESSAGE_ID

# Send
$GAPI gmail send --to user@example.com --subject "Hello" --body "Message text"
$GAPI gmail send --to user@example.com --subject "Report" --body "<h1>Q4</h1>" --html

# Reply (auto-threads, sets In-Reply-To)
$GAPI gmail reply MESSAGE_ID --body "Thanks, that works."

# Labels
$GAPI gmail labels
$GAPI gmail modify MESSAGE_ID --add-labels LABEL_ID
$GAPI gmail modify MESSAGE_ID --remove-labels UNREAD
```

For complex queries: `skill_view("google-workspace", file_path="references/gmail-search-syntax.md")`.

### Calendar

```bash
# List events (default: next 7 days)
$GAPI calendar list
$GAPI calendar list --start 2026-03-01T00:00:00Z --end 2026-03-07T23:59:59Z

# Create (ISO 8601 with timezone required)
$GAPI calendar create --summary "Standup" --start 2026-03-01T10:00:00-06:00 --end 2026-03-01T10:30:00-06:00
$GAPI calendar create --summary "Review" --start 2026-03-01T14:00:00Z --end 2026-03-01T15:00:00Z --attendees "alice@co.com,bob@co.com"

# Delete
$GAPI calendar delete EVENT_ID
```

### Drive

```bash
# Search
$GAPI drive search "quarterly report" --max 10
$GAPI drive search "mimeType='application/pdf'" --raw-query --max 5

# Metadata
$GAPI drive get FILE_ID

# Upload (auto-detects MIME)
$GAPI drive upload /path/to/report.pdf --name "Logo.png" --parent FOLDER_ID

# Download (Google-native export: Docs→pdf, Sheets→csv, Slides→pdf, Drawings→png)
$GAPI drive download FILE_ID --output ~/doc.pdf
$GAPI drive download DOC_ID --export-mime text/plain --output ~/doc.txt

# Folder
$GAPI drive create-folder "Reports" --parent FOLDER_ID

# Share
$GAPI drive share FILE_ID --email alice@example.com --role reader
$GAPI drive share FILE_ID --email alice@example.com --role writer --notify
$GAPI drive share FILE_ID --type anyone --role reader

# Delete (default: trash; --permanent skips trash)
$GAPI drive delete FILE_ID
$GAPI drive delete FILE_ID --permanent
```

### Contacts

```bash
$GAPI contacts list --max 20
```

### Sheets

```bash
$GAPI sheets create --title "Q4 Budget"
$GAPI sheets get SHEET_ID "Sheet1!A1:D10"
$GAPI sheets update SHEET_ID "Sheet1!A1:B2" --values '[["Name","Score"],["Alice","95"]]'
$GAPI sheets append SHEET_ID "Sheet1!A:C" --values '[["new","row","data"]]'
```

### Docs

```bash
$GAPI docs get DOC_ID
$GAPI docs create --title "Meeting Notes" --body "First paragraph..."
$GAPI docs append DOC_ID --text "Additional content to append"
```

All commands return JSON.

## Rules

1. **Confirm before acting** — never send email, create/delete calendar events, delete/share Drive files, or modify Docs/Sheets without showing the user what will be done and getting explicit approval. Prefer `drive delete` (trash) over `--permanent`.
2. **Check auth before first use** — run `$GSETUP --check`. If it fails, guide through setup.
3. **Calendar times must include timezone** — ISO 8601 with offset (`2026-03-01T10:00:00-06:00`) or UTC (`Z`).
4. **Rate limits** — avoid rapid sequential API calls; batch reads when possible.

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `NOT_AUTHENTICATED` | Run setup Steps 2–5 |
| `REFRESH_FAILED` | Token revoked/expired — redo Steps 3–5 |
| `HttpError 403: Insufficient Permission` | Missing scope — `$GSETUP --revoke` then redo Steps 3–5 |
| `AUTHENTICATED (partial)` / "Token missing scopes" | New scopes needed — `$GSETUP --revoke` then redo Steps 3–5 |
| `HttpError 403: Access Not Configured` | API not enabled in Google Cloud Console |
| `ModuleNotFoundError` | Run `$GSETUP --install-deps` |
| Advanced Protection blocks auth | Workspace admin must allowlist the OAuth client ID |
