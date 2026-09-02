# Architecture

LaRuche separates the desktop shell, the server node and the terminal client. The node
is the center of the system: its API, embedded SPA, channel bots, MCP server and
background jobs run in one Rust process. Optional voice services and browser extensions
connect to it over local interfaces.

## Rust workspace

The Cargo workspace contains 17 packages:

| Package | Role |
|---|---|
| `laruche-bureau` | Tauri desktop application, bundled-node startup and LAN discovery |
| `laruche-node` | axum server, API routes, UI, channel bots, MCP and background jobs |
| `laruche-client` | Shared node client library |
| `laruche-cli` | Terminal TUI |
| `laruche-dashboard` | Vanilla JavaScript SPA embedded in the node |
| `laruche-essaim` | Providers, tools, prompts, sessions, curator, deliberation and LaReine |
| `laruche-butinage` | ReAct loop, compass, gauge, sentinel, compaction handoff and scouts |
| `laruche-memoire` | SQLite, FTS5, embeddings, ranking and OKF import/export |
| `laruche-watchers` | Watcher store and compiled predicate DSL |
| `miel-protocol` | mDNS discovery, manifests and mesh messages |
| `laruche-skills` | Skill parsing, validation and storage |
| `laruche-kanban` | Task board used by missions and automation |
| `laruche-events` | Shared event types |
| `laruche-permissions` | Tool approval policy |
| `laruche-compaction` | Context compaction primitives |
| `laruche-evals` | End-to-end evaluation runner |
| `laruche-icones` | Release icon generator, outside default builds |

The optional `laruche-voix` Python package provides STT and TTS services. The Chrome
extension lives in `extension-chrome/`; the VS Code extension lives in
`laruche/laruche-vscode/`.

## Design rules

- **One authoritative interface.** The desktop window and browser both display the SPA
  served by the node. There is no second frontend to keep synchronized.
- **A testable engine core.** `laruche-butinage` keeps model-independent loop behavior
  separate from providers and IO. The eval runner then tests the assembled system.
- **One hive home.** Memory, sessions, skills, plugins, secrets, settings and journals
  resolve to one data directory regardless of the launcher.
- **Live configuration.** Providers, models, context limits, tools, channels, LaReine,
  MCP and voice can be changed from Settings without restarting the node.
- **Embedded assets.** The SPA has no CDN dependency and remains available offline.
- **Optional edges.** Browser control, voice services and remote hive clients attach to
  the node without duplicating its state.

## How a message flows

1. A message arrives from the SPA, desktop app, TUI, Telegram, Discord, Slack or an API.
2. `laruche-essaim` resolves the session, provider profile, working directory and
   channel-specific model.
3. Context assembly recalls the relevant memory, skills and tool schemas for the intent.
4. `laruche-butinage` runs the model and tool loop while budgets, anti-loop logic,
   cancellation and compaction remain active.
5. Tools execute through one registry. Approval policy, disabled tools and secret
   masking are enforced at dispatch time.
6. Screenshots or camera frames returned by tools join the same multimodal transcript
   as images attached in chat.
7. If enabled, LaReine judges the draft and can accept it, request a fresh run or ask
   the user to decide.
8. The session, activity feed and tool statistics are persisted. Memories used in the
   answer receive reinforcement, and the curator may later propose a reusable skill.

## Background jobs

The node owns:

- watcher evaluation and correlated event rules;
- crons, missions and kanban progression;
- curator passes and LaReine proposal handling;
- memory dream proposals and embedding backfill;
- OKF markdown snapshots committed to `memoire-okf/`;
- channel delivery and the durable outbox;
- mDNS announcements and optional mesh synchronization;
- MCP server and external MCP client connections.

The desktop application starts or joins a node. It does not duplicate these jobs.
