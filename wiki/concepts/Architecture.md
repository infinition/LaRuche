# Architecture

LaRuche is a Rust workspace compiled into a single process. The web dashboard, the API,
the channel bots, the MCP server, and every background job run inside one binary.

## Crates

| Crate | Role |
|---|---|
| `laruche-node` | axum HTTP server: API routes, web UI, channel bots, MCP server, background jobs (curator, dream, OKF git snapshots, watchers scheduler) |
| `laruche-essaim` | Agent layer: the tool registry (abeilles), providers, prompt assembly, working-set recall, curator, LaReine judge and live loop |
| `laruche-butinage` | The pure ReAct core: engine loop, compass (planning), gauge (token budget), sentinel (anti-loop), escale (compaction), eclaireuses (sub-agents). No IO, heavily tested. |
| `laruche-memoire` | The cognitive map: SQLite + FTS5 + embeddings, decay, supersede, hebbian ranking, dream pass, OKF markdown export/import |
| `laruche-watchers` | Event watchers and the compiled rules DSL |
| `miel-protocol` | Mesh: mDNS discovery, node manifests, federation |
| `laruche-dashboard` | Web SPA in vanilla JS, compiled into the binary, works offline |
| `laruche-evals` | Evals harness: fixed missions replayed against the real engine, scored baselines |
| `laruche-cli` | Terminal TUI with five views and a slash palette |
| `laruche-skills` | Skill store: markdown skills, dynamic selection by intent |
| `laruche-kanban` | Kanban board backing the missions |
| `laruche-permissions` | The permission engine behind the approval gates |
| `laruche-voix` | Voice: TTS/STT orchestration, streaming, wake word |

## Design rules

- **The core is pure.** `laruche-butinage` has no IO. Every engine behavior (anti-loop,
  budget, compaction triggering, sub-agent fan-out) is testable without a model or a
  network. The evals harness then validates the assembled system against real models.
- **One process, plain storage.** SQLite for memory, markdown for skills and exports,
  JSONL for journals. Everything on disk is human-readable.
- **Live configuration.** Providers, models, context sizes, the curator, LaReine,
  channels: all editable at runtime through Settings, no restart.
- **The UI is embedded.** No CDN, no external assets, the dashboard works with the
  network cable unplugged.

## How a chat message flows

1. The message arrives through a channel (web UI, Telegram, TUI, voice, MCP).
2. `laruche-essaim` assembles the context: system prompts from memory (`system.*`
   entries), the working set recalled from the cognitive map, the dynamically selected
   tools and skills for this intent.
3. `laruche-butinage` runs the ReAct loop: the model reasons, calls tools, the sentinel
   watches for loops, the gauge watches the budget, the escale compacts if needed.
4. Tool calls execute through the abeille registry, behind approval gates for the
   sensitive ones, with secrets substituted in and masked out.
5. If LaReine is on, the draft answer is judged against the charter; it may be sent
   back for a redo before you ever see it.
6. Memories actually used in the answer get hebbian reinforcement; the session is
   persisted; the curator may later mine the finished conversation for a new skill.

## Background jobs

All spawned by `laruche-node` at startup, all polite about resources:

- **Watchers scheduler**: evaluates compiled rules on their intervals.
- **Curator**: reviews finished conversations, proposes skills. Single-flight, cooldown,
  yields whenever a chat or agent run is active.
- **Dream pass**: memory consolidation proposals.
- **OKF git snapshots**: periodic memory export committed to `memoire-okf/`.
- **Mesh announcer**: mDNS presence, when enabled.
