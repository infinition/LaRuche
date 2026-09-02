# Configuration

Two layers: supported environment variables set at launch, and the Settings UI for
everything that can change live. Internal diagnostics and test-only variables are not
part of this user-facing reference.

## Environment variables

### Core

| Variable | Default | Purpose |
|---|---|---|
| `LARUCHE_PORT` | `8419` | Web UI and API port |
| `LARUCHE_DATA_DIR` | OS user data directory | Hive home for memory, sessions, skills and configuration |
| `LARUCHE_MEMOIRE_BACKEND` | `sqlite` | Memory backend: `sqlite`, `native`, `sidecar` |
| `LARUCHE_BIND_LAN` | off | Bind beyond loopback. Off means the node is unreachable from other machines. |
| `LARUCHE_NO_BROWSER` | off | Do not open the system browser when the node starts |

### Desktop application

| Variable | Default | Purpose |
|---|---|---|
| `LARUCHE_URL` | `http://127.0.0.1:8419` | Explicit node for the desktop application |
| `LARUCHE_SANS_NOEUD` | off | Client-only mode. Never start or locate a local node. |
| `LARUCHE_DECOUVRIR` | off | Print hives found over mDNS and exit without opening a window. |

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
| `LARUCHE_MEMOIRE_ARBITRE` | on | Set to `0` to disable the LLM contradiction arbiter |
| `LARUCHE_TRASH_TTL_SECS` | built-in default | Retention for automatically purged memory trash |

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
| `LARUCHE_DREAM_INTERVAL_SECS` | built-in default | Interval between memory dream passes |

### Providers

The preferred path is a profile in Settings. Environment variables remain useful for
headless deployments:

| Variable | Purpose |
|---|---|
| `LARUCHE_PROVIDER` | Provider family |
| `LARUCHE_MODEL` | Default model |
| `LARUCHE_API_KEY` | Provider key |
| `LARUCHE_API_BASE` | Provider endpoint |
| `LARUCHE_OLLAMA_URL` | Ollama endpoint |
| `LARUCHE_OPENAI_ENDPOINTS` | Additional OpenAI-compatible endpoints |
| `LARUCHE_VISION` | `0` never sends images to the model, `1` sends them even after a provider refused |

See [Providers and profiles](Providers-and-Profiles) for what a profile actually is, why
the active model is a pair rather than a name, and how a peer hive becomes a provider.

## Settings UI

All live, no restart:

- **General**: generation parameters (max passes, temperature, max tokens, dynamic tool
  limit), context and compaction thresholds, curator and agent reactions.
- **Providers**: model endpoints, context sizes, per-channel model assignment.
- **LaReine**: Autonomous, Hybrid and Human in the loop modes, response and live-task
  supervision, judge provider, context window, rework limit, confidence threshold,
  proposal queue, scorecards, training capture and SFT, DPO or judge exports. See the
  [LaReine guide](LaReine) for the exact mode and tier behavior.
- **Secrets**: the vault ([Secrets](Secrets)).
- **MCP**: external servers ([MCP](MCP)).
- **Channels**: Telegram, Discord, Slack tokens and options.
- **Voice**: backends, voice, wake word ([Voice](Voice)).
- **Tools and permissions**: approval mode, disabled tools, dynamic selection and the
  visible computer-control halo.
- **Help**: the installed version, the Miel protocol version, an update check against
  the published releases, and links to the wiki and the repository.

## System prompts

The agent's system prompts are memory entries (`system.*`): identity, behavior,
planning, curator, consolidation. Edit them in the Memory tab; restore-to-default is
one click. See [Cognitive Memory](Cognitive-Memory).
