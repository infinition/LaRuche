# LaRuche v2 🐝

Edge AI agent in Rust. Multi-channel (Telegram / Discord / Slack), persistent
cognitive memory, self-improvement, and swarm federation. One binary, low
footprint, runs locally.

What none of the references (third-party, third-party, Claude Code) combine in a single
runtime: native multi-channel, per-channel memory, a Rust edge engine, a curator
that creates verified skills and tools, mesh federation, and an MCP client and
server.

## Run

```
lancer_butinage.bat
```

This starts the node (`laruche-node`) with the **butinage** engine and the
**sqlite** memory backend, then opens the web dashboard. The launcher exposes a
few settings: engine (`RUCHE_MOTEUR`), memory backend
(`LARUCHE_MEMOIRE_BACKEND` = `native|sqlite|sidecar`), and optional web-search
keys (`LARUCHE_TAVILY_KEY`, `LARUCHE_BRAVE_KEY`, `LARUCHE_SEARXNG_URL`).

Everything else is configured live in **Settings** (model, providers, context,
curator, secrets, MCP, channels) with no restart.

## Architecture (`laruche/` workspace)

| Crate | Role |
|---|---|
| `laruche-node` | HTTP/axum server: API routes, dashboard, channel bots, MCP server, state |
| `laruche-essaim` | Agent engine: ReAct loop, tools, prompt, providers, curator |
| `laruche-butinage` | Testable ReAct core: `butiner()`, cap (boussole/jauge/vigie), escale, eclaireuses |
| `laruche-memoire` | Cognitive map (nodes + items), sqlite / native backends |
| `laruche-dashboard` | Web UI (SPA) served by the node |
| `miel-protocol` | Mesh: mDNS discovery, manifest, node federation |
| `laruche-kanban` · `laruche-watchers` · `laruche-channels` · `laruche-compaction` · `laruche-permissions` · `laruche-skills` · `laruche-voix` · `laruche-cli` | Dedicated modules |

**Brand vocabulary** (kept in French, that is the identity): *butinage* = the
agentic loop, *ruche* / *essaim* = the swarm, *nectar* / *Source* = memory,
*escale* = compaction, *éclaireuse* = sub-agent, *curateur* = the component that
auto-creates verified skills and tools, *Miel* = the mesh protocol.

## What works

- **Multi-channel** Telegram (Discord and Slack wired) with **per-channel
  persistent memory** (UUIDv5).
- **Butinage engine**: ReAct, anti-loop, parallel tools, compaction, budgeted
  sub-agents, live steering.
- **Semantic cognitive memory**: the DB surfaces relevant facts and skills
  (dynamic selection of both tools and the skill catalog based on intent).
- **Self-improvement**: the curator creates and patches **verified** skills and
  plugins in the background.
- **Hot-editable system prompts** (memory `system.*`: identity, behavior,
  planning, curator, consolidation) with restore-to-default.
- **Encrypted secrets** (`${NAME}` / `@@NAME`, the LLM only sees names) with
  `@@` autocomplete everywhere.
- **MCP client and server** (`POST /mcp`).
- **Mesh**: node discovery and skill federation (`/api/mesh/sync`).
- **User hooks** (pre/post-tool, `hooks.json`).
- **Automation hub**: crons, watchers, kanban, missions (with a per-task channel).
- **Full dashboard**: chat, memory (editable), capabilities, missions, settings,
  plugin file browser.
- **Bilingual UI** (FR/EN) with a language toggle.

## Configuration

- **Settings > General**: generation (max passes, temperature, max tokens,
  dynamic tool limit, narrow-context threshold), context/compaction, curator.
- **Settings > Providers / Secrets / MCP / Channels**: configured live.
- **Memory > `system.*`**: edit the system prompts.
- **Telegram commands**: `/help` `/status` `/crons` `/delcron <name|all>`
  `/sethome` `/clear`.

## Docker

```
docker compose up
```

## History and dev docs

Older design documents (vision, audit, roadmaps, handoffs, briefings) are
archived in [`docs/_archive/`](docs/_archive/). Obsolete launchers and dev
scripts are in [`_archive/`](_archive/) and [`laruche/_archive/`](laruche/_archive/).
Current work is tracked in [`ROADMAP.md`](ROADMAP.md).
