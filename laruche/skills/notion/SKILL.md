---
type: skill
name: notion
description: "Notion API + ntn CLI: pages, databases, markdown, Workers."
version: 2.0.0
author: community
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec]
prerequisites:
  env_vars: [NOTION_API_KEY]
metadata:
  laruche:
    tags: [Notion, Productivity, Notes, Database, API, CLI, Workers]
    homepage: https://developers.notion.com
---

# Notion

Two execution paths. Pick one:

- **`ntn` CLI** — preferred on macOS/Linux (shorter syntax, one-liner file uploads, required for Workers).
- **HTTP + curl** — works everywhere including Windows; default when `ntn` is not installed.

Same integration token (`${NOTION_API_KEY}`) works for both.

## Setup

**1. Get an integration token**

1. Create at https://notion.so/my-integrations
2. Copy the key (starts with `ntn_` or `secret_`)
3. Store as `NOTION_API_KEY` in LaRuche's secrets vault.
4. **Share target pages/databases with the integration**: page `...` menu → `Connect to` → your integration. Without this, the API returns 404 even though the page exists.

**2. Install `ntn` (macOS / Linux only — skip on Windows until native support ships)**

```bash
curl -fsSL https://ntn.dev | bash
# or: npm install --global ntn  (requires Node 22+)
ntn --version
```

Set these instead of running `ntn login` (works headlessly):
```bash
export NOTION_API_TOKEN=${NOTION_API_KEY}   # ntn reads NOTION_API_TOKEN
export NOTION_KEYRING=0                      # skip OS keychain
```

Windows: use curl (Path B) directly, or install `ntn` inside WSL2.

**3. Runtime path selection**

```bash
if command -v ntn >/dev/null 2>&1; then
  USE_NTN=1
fi
```

## API Notes

- `Notion-Version: 2025-09-03` is required on all HTTP requests (`ntn` sets it automatically).
- In API version 2025-09-03, "databases" are **data sources** — use `/data_sources/` endpoints for queries.
- Each database has two IDs: `database_id` (for creating pages) and `data_source_id` (for querying).
- Page/database IDs are UUIDs; dashes are optional.
- Rate limit: ~3 req/s. The CLI does not bypass this.
- Pipe curl output through `jq`: `... | jq '.results[0].properties'`
- Always pass `-s` to curl to suppress progress bars.
- Check for API errors: `jq -e '.object == "error"'` on the response; `.code` and `.message` give details.

## Path A — `ntn` CLI

```bash
# Search
ntn api v1/search query="page title"

# Read page metadata
ntn api v1/pages/{page_id}

# Read page as Markdown (agent-friendly)
ntn api v1/pages/{page_id}/markdown

# Read page blocks
ntn api v1/blocks/{page_id}/children

# Create page from Markdown
ntn api v1/pages \
  parent[page_id]=PARENT_ID \
  properties[title][0][text][content]="My Page" \
  markdown="# Heading\n\n- item"

# Patch page with Markdown
ntn api v1/pages/{page_id}/markdown -X PATCH \
  markdown="## Update\n\nShipped."

# Create page in a database
ntn api v1/pages \
  parent[database_id]=DB_ID \
  properties[Name][title][0][text][content]="New Item" \
  "properties[Status][select][name]=Todo"

# Query a database (simple filter)
ntn api v1/data_sources/{data_source_id}/query -X POST \
  filter[property]=Status filter[select][equals]=Active

# Complex query (multiple filters, sorts) — pipe JSON
echo '{"filter":{"property":"Status","select":{"equals":"Active"}},"sorts":[{"property":"Date","direction":"descending"}]}' | \
  ntn api v1/data_sources/{data_source_id}/query -X POST --json -

# Update page properties
ntn api v1/pages/{page_id} -X PATCH \
  "properties[Status][select][name]=Done"

# Append a block
ntn api v1/blocks/{page_id}/children -X PATCH \
  'children:=[{"object":"block","type":"paragraph","paragraph":{"rich_text":[{"text":{"content":"Hello"}}]}}]'

# File upload (one-liner — biggest ntn advantage)
ntn files create < photo.png
ntn files create --external-url https://example.com/photo.png
ntn files list

# Create a database
ntn api v1/data_sources \
  parent[page_id]=PARENT_ID \
  "title[0][text][content]=My DB" \
  'properties:={"Name":{"title":{}},"Status":{"select":{"options":[{"name":"Todo"},{"name":"Done"}]}}}'
```

**ntn syntax:** `key=value` (string), `key[nested]=value` (nested object), `key:=value` (boolean/number/null/array).

## Path B — HTTP + curl (cross-platform)

Base headers for every request (set as shell vars to reduce repetition):
```bash
H_AUTH="Authorization: Bearer ${NOTION_API_KEY}"
H_VER="Notion-Version: 2025-09-03"
H_JSON="Content-Type: application/json"
BASE="https://api.notion.com/v1"
```

```bash
# Search
curl -s -X POST "$BASE/search" -H "$H_AUTH" -H "$H_VER" -H "$H_JSON" \
  -d '{"query":"page title"}'

# Read page metadata
curl -s "$BASE/pages/{page_id}" -H "$H_AUTH" -H "$H_VER"

# Read page as Markdown
curl -s "$BASE/pages/{page_id}/markdown" -H "$H_AUTH" -H "$H_VER"

# Read page blocks
curl -s "$BASE/blocks/{page_id}/children" -H "$H_AUTH" -H "$H_VER"

# Create page from Markdown
curl -s -X POST "$BASE/pages" -H "$H_AUTH" -H "$H_VER" -H "$H_JSON" -d '{
  "parent": {"page_id": "PARENT_ID"},
  "properties": {"title": [{"text": {"content": "My Page"}}]},
  "markdown": "# Heading\n\n- item"
}'

# Patch page with Markdown
curl -s -X PATCH "$BASE/pages/{page_id}/markdown" -H "$H_AUTH" -H "$H_VER" -H "$H_JSON" \
  -d '{"markdown":"## Update\n\nShipped."}'

# Create page in a database
curl -s -X POST "$BASE/pages" -H "$H_AUTH" -H "$H_VER" -H "$H_JSON" -d '{
  "parent": {"database_id": "DB_ID"},
  "properties": {
    "Name": {"title": [{"text": {"content": "New Item"}}]},
    "Status": {"select": {"name": "Todo"}}
  }
}'

# Query a database
curl -s -X POST "$BASE/data_sources/{data_source_id}/query" -H "$H_AUTH" -H "$H_VER" -H "$H_JSON" -d '{
  "filter": {"property": "Status", "select": {"equals": "Active"}},
  "sorts": [{"property": "Date", "direction": "descending"}]
}'

# Update page properties
curl -s -X PATCH "$BASE/pages/{page_id}" -H "$H_AUTH" -H "$H_VER" -H "$H_JSON" \
  -d '{"properties": {"Status": {"select": {"name": "Done"}}}}'

# Append blocks
curl -s -X PATCH "$BASE/blocks/{page_id}/children" -H "$H_AUTH" -H "$H_VER" -H "$H_JSON" -d '{
  "children": [{"object":"block","type":"paragraph","paragraph":{"rich_text":[{"text":{"content":"Hello"}}]}}]
}'

# Create a database
curl -s -X POST "$BASE/data_sources" -H "$H_AUTH" -H "$H_VER" -H "$H_JSON" -d '{
  "parent": {"page_id": "PARENT_ID"},
  "title": [{"text": {"content": "My DB"}}],
  "properties": {
    "Name": {"title": {}},
    "Status": {"select": {"options": [{"name":"Todo"},{"name":"Done"}]}},
    "Date": {"date": {}}
  }
}'

# File upload (3-step)
# 1. Create upload
UPLOAD=$(curl -s -X POST "$BASE/file_uploads" -H "$H_AUTH" -H "$H_VER" -H "$H_JSON" \
  -d '{"filename":"photo.png","content_type":"image/png"}')
UPLOAD_URL=$(echo $UPLOAD | jq -r '.upload_url')
FILE_ID=$(echo $UPLOAD | jq -r '.id')
# 2. PUT bytes
curl -s -X PUT "$UPLOAD_URL" --data-binary @photo.png
# 3. Reference $FILE_ID in a page/block payload
```

## Property Types (database items)

| Type | Format |
|---|---|
| Title | `{"title":[{"text":{"content":"..."}}]}` |
| Rich text | `{"rich_text":[{"text":{"content":"..."}}]}` |
| Select | `{"select":{"name":"Option"}}` |
| Multi-select | `{"multi_select":[{"name":"A"},{"name":"B"}]}` |
| Date | `{"date":{"start":"2026-01-15","end":"2026-01-16"}}` |
| Checkbox | `{"checkbox":true}` |
| Number | `{"number":42}` |
| URL | `{"url":"https://..."}` |
| Email | `{"email":"user@example.com"}` |
| Relation | `{"relation":[{"id":"page_id"}]}` |

## Notion-Flavored Markdown

Standard CommonMark + XML tags for Notion blocks. Use **tabs** for indentation.

```
<callout icon="🎯" color="blue_bg">Content</callout>

<details color="gray">
<summary>Toggle title</summary>
	Children indented one tab
</details>

<columns>
	<column>Left</column>
	<column>Right</column>
</columns>

<table_of_contents color="gray"/>
```

Inline: `<mention-user url="..."/>`, `<mention-page url="...">Title</mention-page>`, `<mention-date start="2026-05-15"/>`, `<span underline="true">text</span>`, `<span color="blue">text</span>`, inline math `$x^2$`, block math `$$ ... $$`, citations `[^https://example.com]`.

Colors: `gray brown orange yellow green blue purple pink red` + `*_bg` variants for backgrounds.

Headings 5/6 collapse to H4. Multiple `>` lines = separate quote blocks; use `<br>` inside a single `>` for multi-line quotes.

## Notion Workers (advanced — requires `ntn`, Business/Enterprise plan)

Workers = TypeScript programs Notion hosts. Capabilities: **Syncs** (pull external data on schedule), **Tools** (callable from Notion Custom Agents), **Webhooks** (receive HTTP events).

Free through August 11, 2026; metered on Notion credits after. Windows requires WSL2.

```bash
ntn workers new my-worker   # scaffold
cd my-worker
# Edit src/index.ts
ntn workers deploy --name my-worker
```

`src/index.ts` — minimal tool:
```typescript
import { Worker } from "@notionhq/workers";
const worker = new Worker();
export default worker;

worker.tool("greet", {
  title: "Greet a User",
  description: "Returns a friendly greeting",
  inputSchema: { type: "object", properties: { name: { type: "string" } }, required: ["name"] },
  execute: async ({ name }) => `Hello, ${name}!`,
});
```

Webhook capability:
```typescript
worker.webhook("onGithubPush", {
  title: "GitHub Push Handler",
  execute: async (events, { notion }) => {
    for (const event of events) {
      console.log("got delivery", event.deliveryId);
      // event.body, event.rawBody (for signature verification), event.headers
    }
  },
});
```

Worker lifecycle:
```bash
ntn workers list
ntn workers exec <capability-key> -d '{"name":"world"}'
ntn workers sync trigger <key>     # run a sync now
ntn workers sync pause <key>
ntn workers env set GITHUB_WEBHOOK_SECRET=...
ntn workers runs list
ntn workers runs logs <run-id>
ntn workers webhooks list          # shows the generated webhook URL (treat as secret)
```

Docs: https://developers.notion.com/workers

## Pitfalls

- **404 on valid page**: integration not connected to that page — go to `...` → `Connect to`.
- **`ntn` on Windows**: not yet supported natively; use curl or WSL2.
- **`data_source_id` vs `database_id`**: use `database_id` when creating pages, `data_source_id` when querying.
- **`is_inline: true`**: required when creating databases embedded inside a page.
- **View filters**: cannot be set via API — UI-only.
- **Workers plan gate**: deploy requires Business or Enterprise; scaffolding and local testing work on any plan.
- **API errors**: response body `{"object":"error","code":"...","message":"..."}` — always check `jq '.object'` before processing results.
- **Notion MCP server**: LaRuche can wire it via `tool_call` / MCP tools for streaming Notion access; the curl/ntn paths above are sufficient for most one-shot tasks.
