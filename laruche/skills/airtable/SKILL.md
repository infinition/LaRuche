---
type: skill
name: airtable
description: Airtable REST API via curl — CRUD, filters, upserts, pagination.
version: 1.2.1
author: community
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec, execute_code, file_write]
prerequisites:
  env_vars: [AIRTABLE_API_KEY]
  commands: [curl]
metadata:
  laruche:
    tags: [Airtable, Productivity, Database, API]
    homepage: https://airtable.com/developers/web/api/introduction
---

# Airtable — Bases, Tables & Records

Work with Airtable's REST API via `shell_exec` using `curl` and a Personal Access Token (PAT). No MCP server, no SDK, no OAuth flow.

## Prerequisites

1. Create a **PAT** at https://airtable.com/create/tokens (tokens start with `pat...`).
2. Minimum scopes: `data.records:read`, `data.records:write`, `schema.bases:read`.
3. In the token UI, add each base to the token's **Access** list. A valid token on an unlisted base returns `403`.
4. Secret is available as `${AIRTABLE_API_KEY}` — injected at execution.

> Legacy `key...` API keys were deprecated Feb 2024. Only PATs and OAuth tokens work now.

## API Basics

- **Endpoint:** `https://api.airtable.com/v0`
- **Auth header:** `Authorization: Bearer ${AIRTABLE_API_KEY}`
- **Content-Type:** `application/json` for POST/PATCH/PUT bodies
- **Object IDs:** bases `app...`, tables `tbl...`, records `rec...`, fields `fld...`. IDs never change; names can. Prefer IDs in automations.
- **Rate limit:** 5 req/sec/base. `429` → back off; check `Retry-After` header.

Base `shell_exec` pattern:
```bash
curl -s "https://api.airtable.com/v0/$BASE_ID/$TABLE?maxRecords=5" \
  -H "Authorization: Bearer ${AIRTABLE_API_KEY}" | python3 -m json.tool
```

Always use `-s` to suppress curl's progress bar. Pipe through `python3 -m json.tool` for readable JSON; use `jq` only when filtering/projection is needed.

## Field Types

| Field type | Write shape |
|---|---|
| Single line text | `"Name": "hello"` |
| Long text | `"Notes": "multi\nline"` |
| Number | `"Score": 42` |
| Checkbox | `"Done": true` |
| Single select | `"Status": "Todo"` (option must exist unless `typecast: true`) |
| Multi-select | `"Tags": ["urgent", "bug"]` |
| Date | `"Due": "2026-04-01"` |
| DateTime (UTC) | `"At": "2026-04-01T14:30:00.000Z"` |
| URL / Email / Phone | `"Link": "https://…"` |
| Attachment | `"Files": [{"url": "https://…"}]` (Airtable fetches + rehosts) |
| Linked record | `"Owner": ["recXXXXXXXXXXXXXX"]` (array of record IDs) |
| User | `"AssignedTo": {"id": "usrXXXXXXXXXXXXXX"}` |

Pass `"typecast": true` at the top level of a create/update body to let Airtable auto-coerce values (e.g. create a new select option on the fly).

## Queries

### Check auth
```bash
curl -s -o /dev/null -w "%{http_code}\n" https://api.airtable.com/v0/meta/bases \
  -H "Authorization: Bearer ${AIRTABLE_API_KEY}"
# Expect: 200
```

### List bases
```bash
curl -s "https://api.airtable.com/v0/meta/bases" \
  -H "Authorization: Bearer ${AIRTABLE_API_KEY}" | python3 -m json.tool
```

### List tables + schema for a base
```bash
curl -s "https://api.airtable.com/v0/meta/bases/$BASE_ID/tables" \
  -H "Authorization: Bearer ${AIRTABLE_API_KEY}" | python3 -m json.tool
```
Run this BEFORE mutating — confirms exact field names/IDs and `options.choices` for select fields.

### List records
```bash
curl -s "https://api.airtable.com/v0/$BASE_ID/$TABLE?maxRecords=100" \
  -H "Authorization: Bearer ${AIRTABLE_API_KEY}" | python3 -m json.tool
```

### Get a single record
```bash
curl -s "https://api.airtable.com/v0/$BASE_ID/$TABLE/$RECORD_ID" \
  -H "Authorization: Bearer ${AIRTABLE_API_KEY}" | python3 -m json.tool
```

### Filter records (filterByFormula)
Formulas must be URL-encoded. Use Python stdlib — never hand-encode:
```bash
FORMULA="{Status}='Todo'"
ENC=$(python3 -c 'import sys, urllib.parse; print(urllib.parse.quote(sys.argv[1], safe=""))' "$FORMULA")
curl -s "https://api.airtable.com/v0/$BASE_ID/$TABLE?filterByFormula=$ENC&maxRecords=20" \
  -H "Authorization: Bearer ${AIRTABLE_API_KEY}" | python3 -m json.tool
```

Useful formula patterns:
- Exact match: `{Email}='user@example.com'`
- Contains: `FIND('bug', LOWER({Title}))`
- Multiple: `AND({Status}='Todo', {Priority}='High')`
- Not empty: `NOT({Assignee}='')`
- Date: `IS_AFTER({Due}, TODAY())`

### Sort + select specific fields
```bash
curl -s "https://api.airtable.com/v0/$BASE_ID/$TABLE?sort%5B0%5D%5Bfield%5D=Priority&sort%5B0%5D%5Bdirection%5D=asc&fields%5B%5D=Name&fields%5B%5D=Status" \
  -H "Authorization: Bearer ${AIRTABLE_API_KEY}" | python3 -m json.tool
```
Square brackets in query params must be URL-encoded (`%5B` / `%5D`).

### Use a named view
```bash
curl -s "https://api.airtable.com/v0/$BASE_ID/$TABLE?view=Grid%20view&maxRecords=50" \
  -H "Authorization: Bearer ${AIRTABLE_API_KEY}" | python3 -m json.tool
```

## Mutations

### Create a record
```bash
curl -s -X POST "https://api.airtable.com/v0/$BASE_ID/$TABLE" \
  -H "Authorization: Bearer ${AIRTABLE_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"fields":{"Name":"New task","Status":"Todo","Priority":"High"}}' | python3 -m json.tool
```

### Batch create (up to 10 records per call)
```bash
curl -s -X POST "https://api.airtable.com/v0/$BASE_ID/$TABLE" \
  -H "Authorization: Bearer ${AIRTABLE_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "typecast": true,
    "records": [
      {"fields": {"Name": "Task A", "Status": "Todo"}},
      {"fields": {"Name": "Task B", "Status": "In progress"}}
    ]
  }' | python3 -m json.tool
```
For >10 records, loop in batches of 10. Use `execute_code` (Python) for large datasets to avoid shell quoting issues with dynamic JSON bodies.

### Update a record (PATCH — merges, preserves unspecified fields)
```bash
curl -s -X PATCH "https://api.airtable.com/v0/$BASE_ID/$TABLE/$RECORD_ID" \
  -H "Authorization: Bearer ${AIRTABLE_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"fields":{"Status":"Done"}}' | python3 -m json.tool
```
Use `PUT` only to replace a record entirely (clears all fields you omit).

### Upsert by a merge field (no record ID needed)
```bash
curl -s -X PATCH "https://api.airtable.com/v0/$BASE_ID/$TABLE" \
  -H "Authorization: Bearer ${AIRTABLE_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "performUpsert": {"fieldsToMergeOn": ["Email"]},
    "records": [
      {"fields": {"Email": "user@example.com", "Status": "Active"}}
    ]
  }' | python3 -m json.tool
```
Creates records whose merge-field value is new; patches records where it already exists. Ideal for idempotent syncs.

### Delete a record
```bash
curl -s -X DELETE "https://api.airtable.com/v0/$BASE_ID/$TABLE/$RECORD_ID" \
  -H "Authorization: Bearer ${AIRTABLE_API_KEY}" | python3 -m json.tool
```

### Batch delete (up to 10)
```bash
curl -s -X DELETE "https://api.airtable.com/v0/$BASE_ID/$TABLE?records%5B%5D=rec1&records%5B%5D=rec2" \
  -H "Authorization: Bearer ${AIRTABLE_API_KEY}" | python3 -m json.tool
```

## Pagination

List endpoints return at most **100 records per page**. If the response includes `"offset"`, pass it on the next call. For large dumps, prefer `execute_code` (Python) — cleaner than a shell loop:

```python
import os, urllib.request, urllib.parse, json

TOKEN = os.environ["AIRTABLE_API_KEY"]
BASE_ID = "appXXX"
TABLE = "tblXXX"
records, offset = [], ""

while True:
    params = {"pageSize": "100"}
    if offset:
        params["offset"] = offset
    url = f"https://api.airtable.com/v0/{BASE_ID}/{TABLE}?" + urllib.parse.urlencode(params)
    req = urllib.request.Request(url, headers={"Authorization": f"Bearer {TOKEN}"})
    data = json.loads(urllib.request.urlopen(req).read())
    records.extend(data["records"])
    offset = data.get("offset", "")
    if not offset:
        break

print(f"Total records: {len(records)}")
```

Shell loop alternative (simpler datasets):
```bash
OFFSET=""
while :; do
  URL="https://api.airtable.com/v0/$BASE_ID/$TABLE?pageSize=100"
  [ -n "$OFFSET" ] && URL="$URL&offset=$OFFSET"
  RESP=$(curl -s "$URL" -H "Authorization: Bearer ${AIRTABLE_API_KEY}")
  echo "$RESP" | python3 -c 'import json,sys; d=json.load(sys.stdin); [print(r["id"], r["fields"].get("Name","")) for r in d["records"]]'
  OFFSET=$(echo "$RESP" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("offset",""))')
  [ -z "$OFFSET" ] && break
done
```

## Workflow

1. **Check auth** — verify `200` (see Queries above).
2. **Find the base** — list bases or ask the user for the `app...` ID.
3. **Inspect schema** — `GET /v0/meta/bases/$BASE_ID/tables` before any mutation; cache exact field names.
4. **Read before write** — for "update X where Y", `filterByFormula` to resolve `rec...` ID, then `PATCH`.
5. **Batch writes** — combine creates into 10-record POSTs to stay under the rate limit.
6. **Destructive ops** — deletions are permanent. For "delete all Xs", show the filter + record count and confirm before executing.
7. **Large result sets** — pipe output through `file_write` if the data will be processed later.

## Pitfalls

- **`filterByFormula` must be URL-encoded.** Field names with spaces also need encoding. Always use `python3 urllib.parse.quote` — never hand-escape.
- **Empty fields are omitted from responses.** A missing key means the value is empty, not that the field doesn't exist. Verify via schema.
- **Single-select options must exist.** Writing an unknown option errors with `INVALID_MULTIPLE_CHOICE_OPTIONS` unless you pass `"typecast": true`.
- **`403` on one base but not another** means the token's Access list doesn't include that base — not a scope issue. Fix at https://airtable.com/create/tokens.
- **Rate limits are per base.** 5 req/sec on `baseA` is independent of `baseB`. On `429`, check `Retry-After` and sleep accordingly.
- **Read the `errors` array** on non-2xx: `AUTHENTICATION_REQUIRED`, `INVALID_PERMISSIONS`, `MODEL_ID_NOT_FOUND`, `INVALID_MULTIPLE_CHOICE_OPTIONS` identify the exact problem.
- **Dynamic JSON bodies with shell variables** are error-prone. Prefer `execute_code` (Python `requests`-free: use `urllib.request`) when field values contain quotes, newlines, or non-ASCII.
