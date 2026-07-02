# Configuration

Two layers: environment variables set at launch (the launcher-level knobs), and the
Settings UI for everything else, editable live with no restart.

## Environment variables

### Core

| Variable | Default | Purpose |
|---|---|---|
| `LARUCHE_PORT` | `8419` | Web UI and API port |
| `LARUCHE_MEMOIRE_BACKEND` | `sqlite` | Memory backend: `sqlite`, `native`, `sidecar` |
| `LARUCHE_BIND_LAN` | off | Bind beyond loopback. Off means the node is unreachable from other machines. |

### TLS

| Variable | Default | Purpose |
|---|---|---|
| `LARUCHE_HTTPS` | off | Serve HTTPS with a self-signed certificate (needed for the microphone from other devices) |
| `LARUCHE_TLS_CERT` / `LARUCHE_TLS_KEY` | none | Use your own certificate and key |

### Memory

| Variable | Default | Purpose |
|---|---|---|
| `LARUCHE_EMBED_URL` | local Ollama | Embedding endpoint for semantic recall |
| `LARUCHE_EMBED_MODEL` | `nomic-embed-text` | Embedding model |
| `LARUCHE_OKF_GIT_SECS` | `1800` | Interval for git time-travel snapshots of the memory, `0` disables |

### Web search

| Variable | Default | Purpose |
|---|---|---|
| `LARUCHE_TAVILY_KEY` | none | Tavily API key |
| `LARUCHE_BRAVE_KEY` | none | Brave Search API key |
| `LARUCHE_SEARXNG_URL` | none | A SearXNG instance |

With none set, the search tools fall back to free scrapers. A key improves reliability.

### Background components

| Variable | Default | Purpose |
|---|---|---|
| `LARUCHE_CURATEUR_COOLDOWN_SECS` | `600` | Minimum delay between curator passes |
| `LARUCHE_REINE_BUDGET_SECS` | set in Settings | Time cap for LaReine judging and redo loops |

## Settings UI

All live, no restart:

- **General**: generation parameters (max passes, temperature, max tokens, dynamic tool
  limit), context and compaction thresholds, curator toggle.
- **Providers**: model endpoints, context sizes, per-channel model assignment.
- **LaReine**: on/off, the charter, thresholds, budget, the proposal queue.
- **Secrets**: the vault ([Secrets](Secrets)).
- **MCP**: external servers ([MCP](MCP)).
- **Channels**: Telegram, Discord, Slack tokens and options.
- **Voice**: backends, voice, wake word ([Voice](Voice)).

## System prompts

The agent's system prompts are memory entries (`system.*`): identity, behavior,
planning, curator, consolidation. Edit them in the Memory tab; restore-to-default is
one click. See [Cognitive Memory](Cognitive-Memory).
