# Installation

The normal entry point is the desktop application. The server node can also run alone
on a workstation or homelab machine. Neither path needs Node.js or a separate frontend
build. Optional voice services use Python.

## Release builds

Download the latest build from the project releases page.

- Windows: full NSIS or MSI installer, plus a lightweight client installer.
- Linux: full AppImage or deb package, plus client-only variants.
- macOS: portable archive. A signed DMG is not currently published.
- Every platform: a portable archive containing `laruche`, `laruche-node` and
  `laruche-cli`.

Use `laruche` for the desktop application. See [Desktop App](Desktop-App) for how it
starts a bundled node or connects to another hive.

## Build prerequisites

- [Rust](https://rustup.rs/) stable
- `git`, for the source checkout and memory snapshots
- A model server such as llama.cpp, Ollama, LM Studio or vLLM, or a supported API
  provider
- Optional: `ollama pull nomic-embed-text` for semantic memory recall

Linux desktop builds also need the WebKitGTK, GTK, X11 and PipeWire development
packages listed in `.github/workflows/ci.yml`.

## Desktop application from source

```bash
git clone https://github.com/infinition/LaRuche
cd LaRuche/laruche
cargo build --release -p laruche-node -p laruche-bureau
cargo run --release -p laruche-bureau
```

On Windows, `lancer_bureau.bat` performs these steps and opens the application.
`lancer_bureau_client.bat` runs the client-only mode.

## Server only

```bash
git clone https://github.com/infinition/LaRuche
cd LaRuche/laruche
cargo run --release -p laruche-node
```

Then open `http://localhost:8419`. The dashboard is compiled into the node and works
without a CDN or frontend build step.

`lancer_butinage.bat` remains available on Windows for the browser-based server path.

## Docker

```bash
cd LaRuche/laruche
docker compose up
```

The compose file builds the node in a multi-stage image and exposes port 8419. Desktop
control is not the intended Docker use case.

## Data on disk

Installed builds use one hive home:

| Platform | Default location |
|---|---|
| Windows | `%APPDATA%\LaRuche` |
| macOS | `~/Library/Application Support/LaRuche` |
| Linux | `$XDG_DATA_HOME/laruche`, normally `~/.local/share/laruche` |

`LARUCHE_DATA_DIR` overrides the location. A source directory that already contains
hive state remains in use for compatibility.

| File or folder | Content |
|---|---|
| `memoire.db` | Cognitive memory in SQLite |
| `memoire-okf/` | Optional markdown snapshots in a dedicated git repository |
| `sessions/` | Conversation history |
| `skills/` | User and bundled markdown skills |
| `plugins/` | Runtime tool plugins |
| configuration and journals | Providers, channels, secrets, activity and supervision state |

Back up the hive home to preserve the full installation state.

## Updating

Installed applications can be replaced by a newer release. From source:

```bash
git pull
cargo build --release -p laruche-node -p laruche-bureau
```

Database migrations run automatically at startup.

## Next step

Continue with the [Quick Start](Quick-Start).
