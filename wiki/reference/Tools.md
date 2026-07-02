# Tools

Tools are called **abeilles** (bees) in the hive. Around 90 are built in, plus whatever
you add through [MCP](MCP). The agent does not see all of them at once: dynamic
selection matches the conversation's intent against the registry and injects only the
relevant ones, keeping the context lean for small models.

Sensitive tools sit behind approval gates ([Security](Security)); every tool's output
passes through the secrets mask ([Secrets](Secrets)).

## Web

- Search across providers (Tavily, Brave, SearXNG, free scrapers as fallback)
- Fetch with pagination for long pages and PDF text extraction
- Parallel deep search: fan out sub-agent scouts over a research question

## Files

- Read, write, edit, search, list, within the workspace
- Sensitive paths and destructive writes go through approval

## Execution

- Shell commands (approval-gated)
- Python execution (approval-gated)

## Memory

- Recall (semantic + full-text), write, update, supersede
- Node and item CRUD, importance management

## Scheduling and automation

- Create, list, delete crons
- Create and manage missions and kanban tasks
- Create watchers through the compiled rules DSL, with server-side target validation

## Communication

- Send messages to channels (Telegram, feed)
- Voice synthesis for spoken replies

## Sub-agents

- Spawn eclaireuses (scout sub-agents) with their own budget slice and mission

## Skills

- Consult and propose skills (proposals can route through the LaReine queue)

## MCP

- Every tool exposed by servers connected in Settings > MCP

The authoritative list is in the app itself: the Capabilities tab shows every
registered abeille with its schema, and the same registry backs the MCP server
endpoint.
