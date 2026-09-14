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

### Mission budget and fallback routes

Both live in Settings under General, in the Advanced block, and both go through
`POST /api/config/provider`.

`mission_budget_tokens` is the total input plus output envelope for one mission. It is
shared: retries, compaction, validation calls and sub-agents all draw from it, so a
mission budget bounds the mission and not one call. `0` keeps the engine running with no
token ceiling; the pass and timeout limits still apply either way.

`fallback_profiles` is a list of at most eight complete routes, tried in order when the
main one fails. Each route declares:

| Field | Meaning |
|---|---|
| `provider` | The dialect to speak on this route |
| `model` | The model to ask for |
| `context_max_tokens` | This route's real window, at least 256 |
| `api_key` | Optional. A vault reference such as `@@NAME` is resolved at call time |
| `api_base` | Optional. The endpoint for this route |
| `ollama_url` | Optional. The Ollama endpoint for this route |
| `text_tools` | Optional. `true` when this route needs the text tool protocol |

A route is a full destination rather than a model name, because changing provider changes
the endpoint, the authentication and the shape of the request. A cloud route has to be
written out; nothing is inferred from the main provider. The older `fallback_models`
list, a comma-separated set of model names on the current provider, still works.

Write a credential as a vault reference rather than a literal. A literal key is stored,
but it is never sent back to the browser: the Settings form shows an empty field for it
and keeps what is stored when you save. To remove such a key, remove the route.

## Settings UI

All live, no restart:

- **General**: generation parameters (max passes, temperature, max tokens, dynamic tool
  limit), context and compaction thresholds, curator and agent reactions. Its Advanced
  block also holds the mission token budget and the fallback routes described below.
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
