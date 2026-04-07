<div align="center">
<img width="3533" height="997" alt="image" src="https://github.com/user-attachments/assets/458f9822-8c9e-44da-896c-20ba238925d3" />
</div>

# LaRuche + L'Essaim

<div align="center">
  <img width="287" height="317" alt="icon-removebg-preview" src="https://github.com/user-attachments/assets/a6a37836-7e97-4203-b041-1edb7ec36263" />
</div>

**Open-source AI agent platform. Local-first, privacy-focused.**

Plug a LaRuche node into your local network and AI becomes available to every connected device.
Zero configuration, zero cloud dependency, privacy-first by design.

---

## Features

### Agent Engine (Essaim)

- **ReAct reasoning loop** -- Multi-step tool calling with planning, reflection, and sub-agent delegation
- **23+ built-in tools (Abeilles)** -- File I/O, shell exec, web search, web fetch, deep search, math, git, calendar, browser automation, RAG knowledge base, file watch, system info, delegation, and more
- **Context compaction** -- Auto-summarization when context grows too large for small models
- **Model failover** -- Automatic fallback to alternative models on errors
- **Approval gating** -- Dangerous tools require user confirmation before execution
- **Parallel tool execution** -- Multiple tool calls executed concurrently

### Multi-Provider LLM

- **Simultaneous providers** -- Ollama (local), OpenAI, Anthropic, Groq, OpenRouter, or any OpenAI-compatible API, all active at once
- **Provider profiles** -- Manage multiple API keys and endpoints via `provider-profiles.json` or the Settings UI
- **Per-model selection** -- Switch between local and cloud models from the UI or CLI, per conversation
- **Unified model list** -- All models from all providers appear in a single dropdown, grouped by provider

### Networking (Miel Protocol)

- **Zero-config discovery** -- mDNS broadcast (DNS-SD) for automatic LAN node detection
- **Swarm intelligence** -- Collective model listing, tensor-parallel sharding across nodes
- **Cross-node session sync** -- Sessions and users replicate transparently between nodes
- **Proximity auth** -- Device tokens via physical proximity (button press / NFC)

### Multi-User Authentication

- **QR code login** -- Enrollment via display name, permanent auth QR saved on smartphone
- **Challenge-based login** -- Ephemeral QR (60s) scanned from phone to authenticate browser
- **BLAKE3 cookie auth** -- Signed cookies with 30-day expiry, shared across cluster
- **Per-user sessions** -- Each user sees only their own conversations (legacy sessions visible to all)
- **Auto-enrollment** -- Open registration, no admin approval needed

### Interfaces

- **SPA Web Dashboard** -- Hash-routed single page app with chat, dashboard, sessions, settings, console
- **Interactive CLI TUI** -- Full terminal UI with Ratatui (sidebar panels, markdown rendering, model picker, autocompletion)
- **Server TUI** -- Fixed-layout terminal UI for `laruche-node` with scrolling logs and live system gauges
- **Voice pipeline** -- STT (Whisper) + TTS (edge-tts / Kokoro / pyttsx3) via WebSocket
- **Telegram bot** -- Native Rust integration, auto-start from config
- **Discord / Slack** -- Webhook-based integrations
- **MCP server** -- JSON-RPC over stdio for Claude Desktop, Cursor, etc.
- **VS Code extension** -- Inline AI assistance, chat sidebar, swarm node selection

### Data & Persistence

- **Session management** -- Persistent conversations with search, export (Markdown), fork, delete
- **RAG knowledge base** -- Add documents, search with embeddings (nomic-embed-text)
- **Cron scheduler** -- Schedule recurring agent tasks with cron expressions
- **Plugin system** -- Drop JSON tool definitions into `plugins/` for instant custom tools

## Architecture

```text
+-------------------------------------------------------------------+
|                        LaRuche Platform                           |
+-------------------------------------------------------------------+
|                                                                   |
|  +------------------+    Miel (mDNS+HTTP)    +------------------+ |
|  | LaRuche Node A   |<--------------------->| LaRuche Node B   | |
|  | (Core, 7B model) |   Session/User sync   | (Pro, 30B model) | |
|  +--------+---------+                       +--------+---------+ |
|           |                                          |           |
|           +------- Swarm Intelligence ---------------+           |
|                          |                                       |
+-------------------------------------------------------------------+
|                       Essaim Engine                               |
|  +----------+  +--------+  +---------+  +-------+  +---------+  |
|  | Brain    |  | Tools  |  | Session |  | RAG   |  | Cron    |  |
|  | (ReAct)  |  | (23+)  |  | (Persist)| | (KB)  |  | (Sched) |  |
|  +----------+  +--------+  +---------+  +-------+  +---------+  |
+-------------------------------------------------------------------+
|                      Multi-Provider LLM                           |
|  +--------+  +---------+  +-----------+  +------+  +----------+ |
|  | Ollama |  | OpenAI  |  | Anthropic |  | Groq |  | OpenRou- | |
|  | (local)|  |         |  |           |  |      |  | ter      | |
|  +--------+  +---------+  +-----------+  +------+  +----------+ |
+-------------------------------------------------------------------+
|                        Interfaces                                 |
|  +------+ +-----+ +--------+ +-------+ +-------+ +------+ +---+ |
|  | Web  | | TUI | | VSCode | | Tele- | | Disc- | | Slack| | M | |
|  | SPA  | | CLI | | Ext.   | | gram  | | ord   | |      | | C | |
|  +------+ +-----+ +--------+ +-------+ +-------+ +------+ | P | |
+-------------------------------------------------------------------+
```

## Installation

### Prerequisites

- [Rust 1.75+](https://rustup.rs/)
- [Ollama](https://ollama.com/) running locally (optional if using cloud providers only)

### From source

```bash
git clone https://github.com/infinition/LaRuche.git
cd LaRuche

# Install both binaries to your PATH
cargo install --path laruche-node --force
cargo install --path laruche-cli --force
```

This installs two commands:

| Command | Binary | Description |
|---------|--------|-------------|
| `laruche-node` | `laruche-node.exe` | The server (API + Web UI + Agent engine) |
| `laruche` | `laruche.exe` | The client TUI (connects to a running server) |

Both are installed to `~/.cargo/bin/` which is in your system PATH.

### Pull a model

```bash
ollama pull gemma3:12b
```

### Data directory

`laruche-node` creates its data files in the **current working directory**:

```
your-folder/
  sessions/              # Saved conversations
  users/                 # User identities (auth)
  provider-profiles.json # LLM provider configs + API keys
  laruche-state.json     # Persistent state (default model, activity)
  channels-config.json   # Telegram/Discord/Slack config
  cron-tasks.json        # Scheduled tasks
  laruche.toml           # Server config (optional)
```

> **Tip:** Always launch `laruche-node` from the same directory so your data is found.

## Usage

### Option 1: Server with TUI (interactive)

```bash
cd ~/laruche-data
laruche-node
```

Shows a full terminal UI with live logs, CPU/RAM/GPU gauges, and session stats.
Press `q` to quit.

### Option 2: Server in background + Client TUI

```bash
# Start server headless
laruche-node --no-tui &

# Launch the client TUI
laruche
```

### Option 3: All from the Client TUI

```bash
laruche
# Then inside the TUI:
/server start       # Starts laruche-node in background
```

### Open the web UI

```
http://localhost:8419
```

The SPA includes: Chat, Dashboard, Sessions, Settings, Console.

## CLI Reference

```bash
# Interactive TUI chat (default)
laruche

# Classic REPL mode
laruche --classic

# One-shot question
laruche ask "Explain ownership in Rust"

# Start in a specific directory
laruche --cwd /path/to/project

# Network discovery
laruche discover

# System diagnostics
laruche doctor

# Server management
laruche server start|stop|restart|status|install|update|uninstall|logs

# MCP server (stdio, for Claude Desktop)
laruche mcp
```

### In-chat slash commands

| Command | Description |
|---------|-------------|
| `/help` | Show help |
| `/tools` | List available tools (Abeilles) |
| `/model [name]` | Switch model |
| `/cwd [path]` | Change working directory |
| `/clear` | New conversation |
| `/export` | Export session as Markdown |
| `/discover` | Scan network for LaRuche nodes |
| `/doctor` | Run diagnostics |
| `/server [cmd]` | Manage server (start/stop/restart/status/install/update/uninstall) |
| `/quit` | Exit |

## Build Workflow (for developers)

```bash
# Quick compile check (debug, fast)
cargo build

# Build optimized release
cargo build --release

# Install to PATH (does release build automatically)
cargo install --path laruche-node --force
cargo install --path laruche-cli --force

# Or use the CLI shortcut (from the LaRuche source directory):
laruche server install    # builds + installs laruche-node
```

| Command | What it does | Output |
|---------|-------------|--------|
| `cargo build` | Debug build (fast compile, slow exec) | `target/debug/*.exe` |
| `cargo build --release` | Release build (slow compile, fast exec) | `target/release/*.exe` |
| `cargo install --path X --force` | Release build + copy to PATH | `~/.cargo/bin/*.exe` |
| `laruche server install` | Same as above, from the TUI | `~/.cargo/bin/laruche-node.exe` |

## API Endpoints

### Core

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/` | SPA web interface |
| `GET` | `/api/status` | Node status (CPU, RAM, GPU, queue, capabilities) |
| `GET` | `/health` | Health check |
| `GET` | `/nodes` | Discovered peers (mDNS) |
| `GET` | `/swarm` | Collective swarm view |
| `GET` | `/swarm/models` | Models across swarm + cloud providers |
| `GET` | `/models` | Local Ollama models |
| `POST` | `/infer` | Raw inference request |
| `GET` | `/activity` | Activity log |
| `GET` | `/metrics/history` | Time-series metrics (CPU, RAM, GPU, tokens/s) |

### Agent (Essaim)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/ws/chat` | WebSocket chat (streaming tokens, tool calls) |
| `GET` | `/ws/audio` | WebSocket voice (STT/TTS) |
| `GET` | `/api/tools` | List registered tools |
| `GET` | `/api/sessions` | List sessions (filtered by authenticated user) |
| `GET` | `/api/sessions/search?q=` | Full-text search across sessions |
| `GET` | `/api/sessions/:id/messages` | Session messages |
| `GET` | `/api/sessions/:id/export` | Export session as Markdown |
| `POST` | `/api/sessions/:id/fork` | Fork a session |
| `DELETE` | `/api/sessions/:id` | Delete a session |
| `POST` | `/api/webhook` | HTTP webhook (non-streaming) |
| `POST` | `/api/rpc` | JSON-RPC agent calls |

### Authentication

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/auth/enroll` | Create user identity (returns QR SVG) |
| `GET` | `/api/auth/me` | Current user info (from cookie) |
| `GET` | `/api/auth/challenge` | Generate ephemeral login QR (60s) |
| `GET` | `/api/auth/status/:id` | Poll challenge status |
| `POST` | `/api/auth/logout` | Clear auth cookie |
| `GET` | `/auth/scan/:id` | Phone scans this to resolve login |
| `GET` | `/auth/link/:uid/:secret` | Permanent auth link (enrollment QR) |

### Provider Profiles

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/profiles` | List all provider profiles |
| `POST` | `/api/profiles` | Create/update a profile |
| `DELETE` | `/api/profiles/:id` | Delete a profile |
| `GET` | `/api/profiles/models` | Unified model list (all providers) |
| `POST` | `/api/profiles/active` | Set active model + provider |

### Configuration

| Method | Path | Description |
|--------|------|-------------|
| `GET/POST` | `/api/config/channels` | Channel bot config |
| `GET/POST` | `/api/cwd` | Working directory |
| `GET/POST` | `/config/default_model` | Default model |
| `GET` | `/api/onboarding` | Onboarding status |
| `GET` | `/api/doctor` | System diagnostics |

### Channels

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/channels/start` | Start a channel bot |
| `POST` | `/api/channels/stop` | Stop a channel bot |
| `GET` | `/api/channels/status` | Channel bot status |
| `POST` | `/api/channels/discord/webhook` | Discord Interactions endpoint |
| `POST` | `/api/channels/slack/events` | Slack Events API endpoint |

### Cross-Node Sync (internal)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/internal/sync/session` | Push session to peer |
| `POST` | `/api/internal/sync/user` | Push user to peer |
| `GET` | `/api/internal/sync/bulk` | Bulk sync (all sessions + users) |

### Other

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/mcp` | MCP JSON-RPC endpoint |
| `GET/POST` | `/api/knowledge` | RAG knowledge base (CRUD) |
| `GET/POST` | `/api/cron` | Cron scheduler (CRUD) |

## Configuration

### laruche.toml (optional)

```toml
node_name = "laruche-salon"
tier = "core"                          # nano | core | pro | max
ollama_url = "http://127.0.0.1:11434"
default_model = "gemma3:12b"
api_port = 8419

[[capabilities]]
capability = "llm"
model_name = "gemma3:12b"
model_size = "12B"
quantization = "Q4_K_M"
```

Most configuration is done via the **Settings page** in the web UI (provider profiles, channels, etc.).

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `LARUCHE_NAME` | `laruche-xxxxxx` | Node name |
| `LARUCHE_TIER` | `core` | Hardware tier |
| `LARUCHE_PORT` | `8419` | API port |
| `OLLAMA_URL` | `http://127.0.0.1:11434` | Ollama URL |
| `LARUCHE_MODEL` | `gemma3:12b` | Default model (fallback) |
| `LARUCHE_PROVIDER` | `ollama` | Default LLM provider |
| `LARUCHE_API_KEY` | | API key for cloud providers |
| `LARUCHE_TLS_CERT` | | Path to TLS certificate (enables HTTPS) |
| `LARUCHE_TLS_KEY` | | Path to TLS private key |

## Channel Setup

### Telegram

1. Create a bot via [@BotFather](https://t.me/BotFather)
2. In the web UI: Settings > Channels > paste token, enable, save
3. The bot starts automatically on server launch

### Discord

1. Create an app at [Discord Developer Portal](https://discord.com/developers/applications)
2. Add a slash command (e.g., `/ask` with a `prompt` option)
3. Set Interactions Endpoint: `https://your-server/api/channels/discord/webhook`

### Slack

1. Create an app at [api.slack.com](https://api.slack.com/apps)
2. Enable Event Subscriptions: `https://your-server/api/channels/slack/events`
3. Subscribe to `message.channels` and `app_mention`

### MCP (Claude Desktop / Cursor)

```json
{
  "mcpServers": {
    "laruche": {
      "command": "laruche",
      "args": ["mcp"]
    }
  }
}
```

## Workspace Structure

```text
LaRuche/
  miel-protocol/        # Miel protocol (mDNS discovery, auth, QoS, swarm)
  laruche-node/          # Server daemon (API, WebSocket, channels, auth, sync, TUI)
  laruche-essaim/        # Agent engine (ReAct brain, 23+ tools, sessions, RAG, providers)
  laruche-cli/           # Client CLI with TUI (Ratatui)
  laruche-client/        # Rust client SDK
  laruche-dashboard/     # Web UI templates (SPA HTML, embedded in laruche-node)
  laruche-vscode/        # VS Code extension
  laruche-voix/          # Voice module (Whisper STT + edge-tts/Kokoro TTS)
  laruche-channels/      # Legacy Python channel bots (Telegram now native Rust)
  plugins/               # Custom tool plugins (JSON)
```

## License

MPL-2.0
