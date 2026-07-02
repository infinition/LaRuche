# Installation

LaRuche is a single Rust binary. There is no Node, no Python, no runtime to install for
the core (the optional voice sidecars use Python, see [Voice](Voice)).

## Prerequisites

- [Rust](https://rustup.rs/) stable, to build from source
- `git`, used for cloning and for the memory time-travel feature
- A model server (see [Local Models](Local-Models)):
  - [llama.cpp](https://github.com/ggerganov/llama.cpp) in server mode, or
  - [Ollama](https://ollama.com), or
  - any OpenAI-compatible or Anthropic API endpoint
- Optional but recommended for semantic memory recall:

```bash
ollama pull nomic-embed-text
```

## From source (Linux, macOS, Windows)

```bash
git clone https://github.com/infinition/laruche
cd laruche/laruche
cargo run --release -p laruche-node
```

Then open http://localhost:8419.

The first build takes a few minutes; after that, incremental builds are fast. The web
dashboard is compiled into the binary, so there is no separate frontend build step and
the UI works offline.

## Windows launcher

`lancer_butinage.bat` at the repository root builds the node, starts it, waits for the
server to answer, and opens the browser. It is the recommended entry point on Windows.

## Docker

```bash
cd laruche
docker compose up
```

The compose file builds the binary in a multi-stage image and exposes port 8419.

## Data on disk

LaRuche keeps its state next to the binary, in plain inspectable formats:

| File / folder | Content |
|---|---|
| `memoire.db` | The cognitive memory (SQLite, readable with any SQLite tool) |
| `memoire-okf/` | Periodic markdown exports of the memory, versioned in their own git repo |
| session files | Chat history per session |
| skills | Markdown skill files |

Back up `memoire.db` and you have backed up the agent's brain.

## Updating

```bash
git pull
cargo run --release -p laruche-node
```

The SQLite schema migrates automatically on startup.

## Next step

Continue with the [Quick Start](Quick-Start).
