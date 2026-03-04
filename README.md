#  LaRuche - Networked Edge AI System

> **"Branchez l'IA. C'est tout."**

Branchez le boîtier LaRuche sur votre réseau, et l'IA devient disponible pour tout appareil connecté.
Zéro configuration, zéro cloud, zéro compromis sur la vie privée.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Réseau Local                             │
│                                                                 │
│  ┌──────────┐   LAND Protocol   ┌──────────┐                    │
│  │ LaRuche  │◄────────────────► │ LaRuche  │  Swarm             │
│  │  Core    │  _ai-inference    │   Pro    │  Intelligence      │
│  │ (LLM+RAG)│    ._tcp          │(VLM+Code)│                    │
│  └────┬─────┘                   └────┬─────┘                    │
│       │                              │                          │
│  ┌────┴──────────────────────────────┴────┐                     │
│  │          LAND (mDNS/DNS-SD)            │                     │
│  │    Cognitive Manifest + QoS + Auth     │                     │
│  └────┬──────────┬──────────┬─────────────┘                     │
│       │          │          │                                   │
│  ┌────┴───┐ ┌────┴───┐ ┌───┴────┐                               │
│  │ VS Code│ │  Web   │ │  IoT   │  Clients                      │
│  │ Plugin │ │  App   │ │ ESP32  │                               │
│  └────────┘ └────────┘ └────────┘                               │
└─────────────────────────────────────────────────────────────────┘
```

## Workspace Structure

```
laruche/
├── land-protocol/     #  Core LAND protocol library
│   └── src/
│       ├── lib.rs           # Module exports + constants
│       ├── capabilities.rs  # Model type differentiation (LLM, VLM, VLA, RAG...)
│       ├── manifest.rs      # Cognitive Manifest (broadcast data)
│       ├── discovery.rs     # mDNS broadcaster + listener
│       ├── auth.rs          # Proof of Proximity authentication
│       ├── qos.rs           # Quality of Service + priority queue
│       ├── swarm.rs         # Swarm Intelligence + resilience
│       └── error.rs         # Error types
│
├── laruche-node/      #  LaRuche Node daemon
│   └── src/main.rs          # API server + LAND broadcast + Ollama bridge
│
├── laruche-client/    #  Client SDK (3 lines to use AI)
│   └── src/lib.rs           # Auto-discover + ask + route by capability
│
├── laruche-cli/       #   CLI tool
│   └── src/main.rs          # discover, ask, chat, status commands
│
└── laruche-dashboard/ #  Web monitoring dashboard
    └── src/
        ├── main.rs          # Axum web server
        └── templates/
            └── dashboard.html  # Cybersecurity dashboard UI
```

## Quick Start

### Prerequisites

- **Rust** (1.75+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Ollama** (for inference backend): `curl -fsSL https://ollama.com/install.sh | sh`
- **Windows — Option 1 : MSVC (recommandé)**
  Installer [Build Tools for Visual Studio](https://visualstudio.microsoft.com/visual-cpp-build-tools/) avec le workload **"Développement Desktop en C++"**, puis :
  ```powershell
  rustup default stable-x86_64-pc-windows-msvc
  ```

- **Windows — Option 2 : GNU / MSYS2 (sans Visual Studio)**
  Installer [MSYS2](https://www.msys2.org/), puis dans un terminal **PowerShell** :
  ```powershell
  # 1. Installer les binutils MinGW via MSYS2 (dans le terminal MSYS2)
  pacman -S mingw-w64-x86_64-binutils

  # 2. Ajouter le bin MSYS2 au PATH (adapter le chemin si nécessaire)
  $env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH

  # 3. Passer Rust sur le toolchain GNU
  rustup default stable-x86_64-pc-windows-gnu
  ```
  Pour rendre le PATH permanent (PowerShell admin) :
  ```powershell
  [System.Environment]::SetEnvironmentVariable("Path", "C:\msys64\mingw64\bin;" + [System.Environment]::GetEnvironmentVariable("Path", "Machine"), "Machine")
  ```
  > **Note CMD** : Si tu utilises CMD au lieu de PowerShell, remplace `$env:PATH = ...` par :
  > ```cmd
  > set PATH=C:\msys64\mingw64\bin;%PATH%
  > ```

### 1. Pull a model

```bash
ollama pull mistral
```

### 2. Build the project

```bash
cargo fetch
```

```bash
cargo build --release
```

### 3. Start the LaRuche node

```bash
# With defaults (auto-detects network, uses Ollama on localhost)
cargo run -p laruche-node

# With a specific model
LARUCHE_NAME=laruche-salon LARUCHE_MODEL=mistral cargo run -p laruche-node

# With TWO capabilities on the same node (e.g. Mistral for chat + DeepSeek for code)
LARUCHE_CAP=llm LARUCHE_MODEL=mistral LARUCHE_CAP2=code LARUCHE_MODEL2=deepseek-coder cargo run -p laruche-node
```

### 4. Use the CLI

```bash
# Discover nodes on the network
cargo run -p laruche-cli -- discover

# Ask a question (auto-discovers and routes)
cargo run -p laruche-cli -- ask "Explique-moi la photosynthèse"

# Interactive chat
cargo run -p laruche-cli -- chat

# Or connect directly (skip discovery)
LARUCHE_URL=http://localhost:8419 cargo run -p laruche-cli -- ask "Hello"
```

### 5. Open the Dashboard

The dashboard is embedded in the node — no separate process needed:

```
http://localhost:8419/dashboard
```

It shows: active nodes, models per node, real CPU/RAM metrics, collective t/s, and a live event log.

## VS Code Extension

### Installation

```bash
cd laruche-vscode
npm install        # installs bonjour-service + dev deps
npm run compile    # or: npm run watch
```

Then press **F5** in VS Code to launch the Extension Development Host.

### Features

| Feature | Description |
|---|---|
| **Auto-discovery** | Finds LaRuche nodes on the LAN via LAND protocol (mDNS) — no URL needed |
| **Node picker** | `Ctrl+Shift+P` → `LaRuche: Select Active Node` — switch between nodes |
| **Model picker** | `Ctrl+Shift+P` → `LaRuche: Select Active Model` — choose Mistral, DeepSeek, etc. |
| **Chat** | Sidebar chat with markdown rendering, node/model shown in header |
| **File attach** | 📎 button attaches the active editor file as context |
| **Agent (Edit)** | `Ctrl+Shift+L` — sends file + instructions, applies diff-based edits |
| **Agent modes** | `auto` (apply immediately), `ask` (show diff first), `readonly` (suggest only) |
| **Explain/Refactor** | Right-click selection → LaRuche context menu |
| **Swarm status** | Status bar: `⬡ 3 nodes · 45 t/s · mistral` |

### How discovery works

The extension uses **bonjour-service** (pure-JS mDNS) to listen for `_ai-inference._tcp.local.`
announcements. When a LaRuche node starts, it broadcasts itself every 2 seconds.
The extension connects automatically — no IP address needed.

Fallback priority:
1. Manual URL in `laruche.nodeUrl` setting
2. First node discovered via LAND mDNS
3. `localhost:8419`

### Configuration

| Setting | Default | Description |
|---|---|---|
| `laruche.nodeUrl` | `""` | Override URL (bypasses auto-discovery) |
| `laruche.model` | `""` | Preferred model (empty = node default) |
| `laruche.agentMode` | `"ask"` | Agent mode: `auto`, `ask`, `readonly` |

## Node API

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/` | Node status + real CPU/RAM metrics |
| GET | `/health` | Health check |
| GET | `/nodes` | Discovered peer nodes (via LAND) |
| GET | `/swarm` | Collective swarm status (all nodes + models) |
| GET | `/models` | Available Ollama models on this node |
| POST | `/infer` | Run inference (`model` field optional override) |
| GET | `/dashboard` | Embedded web dashboard |
| POST | `/auth/request` | Request Proof-of-Proximity auth |
| POST | `/auth/approve` | Approve pending auth (POC button) |

### Multi-model inference

The `/infer` endpoint now accepts a `model` field to override the node's default:

```bash
curl -X POST http://localhost:8419/infer \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "Write a binary search in Rust",
    "capability": "code",
    "model": "deepseek-coder"
  }'
```

## LAND Protocol

### Capability Flags

| Type    | Flag              | Description                              |
|---------|-------------------|------------------------------------------|
| LLM     | `capability:llm`  | Text-to-text (Mistral, Llama, etc.)      |
| VLM     | `capability:vlm`  | Vision + Language (LLaVA, Qwen-VL)       |
| VLA     | `capability:vla`  | Vision-Language-Action / Robotics        |
| RAG     | `capability:rag`  | Retrieval Augmented Generation           |
| Audio   | `capability:audio` | Speech-to-Text / Text-to-Speech         |
| Image   | `capability:image` | Image generation / analysis             |
| Embed   | `capability:embed` | Vector embeddings                       |
| Code    | `capability:code`  | Code generation / analysis              |

### API Endpoints

| Method | Endpoint           | Description                     |
|--------|-------------------|---------------------------------|
| GET    | `/`               | Node status + capabilities      |
| GET    | `/health`         | Health check                    |
| GET    | `/nodes`          | List discovered peers           |
| POST   | `/infer`          | Send inference request          |
| POST   | `/auth/request`   | Request device authorization    |
| POST   | `/auth/approve`   | Approve pending auth (POC)      |

### Inference Request

```bash
curl -X POST http://localhost:8419/infer \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "Explain quantum computing",
    "capability": "llm",
    "qos": "normal"
  }'
```

### Client SDK Usage (Rust)

```rust
use laruche_client::LaRuche;

#[tokio::main]
async fn main() {
    // Auto-discover (zero config!)
    let laruche = LaRuche::discover().await.unwrap();

    // Ask anything
    let response = laruche.ask("Hello !").await.unwrap();
    println!("{}", response.text);

    // Route by capability
    let code = laruche.ask_with(
        "Write a sorting function",
        laruche_client::Cap::Code,
    ).await.unwrap();
}
```

## Environment Variables

| Variable             | Default                   | Description                |
|---------------------|---------------------------|----------------------------|
| `LARUCHE_NAME`      | `laruche-<random>`        | Node display name          |
| `LARUCHE_TIER`      | `core`                    | Hardware tier              |
| `LARUCHE_MODEL`     | `mistral`                 | Default Ollama model       |
| `LARUCHE_PORT`      | `8419`                    | API port                   |
| `LARUCHE_DASH_PORT` | `8420`                    | Dashboard port             |
| `LARUCHE_CAP`       | `llm`                     | Primary capability         |
| `LARUCHE_URL`       | *(auto-discover)*         | Direct connection (CLI)    |
| `OLLAMA_URL`        | `http://127.0.0.1:11434`  | Ollama backend URL         |

## Multi-Node Setup (Swarm)

Start nodes on different machines on the same network:

```bash
# Machine A
LARUCHE_NAME=laruche-salon LARUCHE_MODEL=mistral cargo run -p laruche-node

# Machine B
LARUCHE_NAME=laruche-bureau LARUCHE_MODEL=codellama LARUCHE_CAP=code cargo run -p laruche-node

# Machine C (with vision)
LARUCHE_NAME=laruche-lab LARUCHE_MODEL=llava LARUCHE_CAP=vlm cargo run -p laruche-node
```

They discover each other automatically via LAND. The CLI and SDK
route requests to the best node for each capability.

## Roadmap

- [x] LAND protocol core (mDNS discovery + Cognitive Manifest)
- [x] Capability differentiation (LLM, VLM, VLA, RAG, Audio, Image, Embed, Code)
- [x] Proof of Proximity authentication
- [x] QoS priority system
- [x] Swarm state management
- [x] Node daemon with Ollama bridge
- [x] Client SDK (3-line usage)
- [x] CLI tool
- [x] Web dashboard with cyber monitoring
- [ ] Tensor sharding over Ethernet (Swarm Intelligence)
- [ ] LaRuche Resilience (failover, hot-swap, mirroring)
- [ ] NFC hardware integration
- [ ] VS Code extension
- [ ] Home Assistant plugin
- [ ] Mobile app (iOS/Android)
- [ ] LAND v1.0 specification (RFC)

### Dépannage réseau (mDNS)

Si le CLI ne trouve pas votre nœud automatiquement :

1.  **Vérifiez le Firewall :** La découverte utilise le port UDP **5353**.
    ```bash
    sudo ufw allow 5353/udp
    ```
2.  **IP Locale :** Les nœuds détectent désormais automatiquement votre IP locale. Vérifiez dans les logs du nœud que l'IP affichée est correcte.
3.  **Variable d'environnement :** En dernier recours, forcez la connexion :
    ```bash
    export LARUCHE_URL=http://<IP_DU_NODE>:8419
    ```

## 📜 Licence

MPL-2.0 - See LICENSE for details.
```
