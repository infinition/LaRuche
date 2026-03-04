<div align="center">
<img width="3533" height="997" alt="image" src="https://github.com/user-attachments/assets/458f9822-8c9e-44da-896c-20ba238925d3" />
</div>



#  LaRuche - Networked Edge AI System

<div align="center">
  <img width="287" height="317" alt="icon-removebg-preview" src="https://github.com/user-attachments/assets/a6a37836-7e97-4203-b041-1edb7ec36263" />
</div>

**"Branchez l'IA. C'est tout."**

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

2. Build:

```bash
cargo check --workspace
```

3. Run a node:

```bash
cargo run -p laruche-node
```

4. Open dashboard:

```text
http://localhost:8419/dashboard
```

## Node configuration

The node loads:

1. `laruche.toml` (or `LARUCHE_CONFIG=<path>`)
2. Environment variables (override file values)

### Environment variables

- `LARUCHE_NAME` (default random `laruche-xxxxxx`)
- `LARUCHE_TIER` (`nano|core|pro|max`)
- `LARUCHE_PORT` (default `8419`)
- `LARUCHE_DASH_PORT` (default `8420`)
- `OLLAMA_URL` (default `http://127.0.0.1:11434`)
- `LARUCHE_MODEL` (default model name)
- `LARUCHE_CAP` (primary capability, ex: `llm`)
- `LARUCHE_CAP2` + `LARUCHE_MODEL2` (optional second capability/model)

### Example `laruche.toml`

```toml
node_name = "laruche-salon"
tier = "core"
ollama_url = "http://127.0.0.1:11434"
default_model = "mistral"
api_port = 8419
dashboard_port = 8420

[[capabilities]]
capability = "llm"
model_name = "mistral"
model_size = "7B"
quantization = "Q4_K_M"

[[capabilities]]
capability = "code"
model_name = "deepseek-coder"
```

## API

### Core endpoints

- `GET /` -> node status (CPU/RAM + queue + capabilities)
- `GET /health` -> plain text `OK`
- `GET /nodes` -> discovered peers (mDNS view)
- `GET /swarm` -> merged swarm state (self + peers, includes node `port`)
- `GET /models` -> local Ollama models
- `GET /swarm/models` -> models across swarm
- `POST /infer` -> inference request
- `GET /activity` -> recent node activity log
- `POST /auth/request` -> auth request (POC)
- `POST /auth/approve` -> approve pending auth (POC)
- `GET /dashboard` -> embedded dashboard HTML

### Inference request

```json
{
  "prompt": "Explain ownership in Rust",
  "capability": "code",
  "model": "deepseek-coder",
  "qos": "normal",
  "max_tokens": 1024,
  "temperature": 0.7
}
```

## VS Code extension

Build extension:

```bash
cd laruche-vscode
npm install
npm run compile
```

Run in extension host with `F5` from VS Code.

## Client SDK (Rust)

```rust
use laruche_client::{Cap, LaRuche};

#[tokio::main]
async fn main() {
    let client = LaRuche::discover().await.unwrap();
    let answer = client.ask_with("Write a Rust iterator", Cap::Code).await.unwrap();
    println!("{}", answer.text);
}
```

## Notes

- This workspace currently depends on `land-protocol` from GitHub (`workspace.dependencies`).
- If you are iterating locally on protocol changes, update dependency resolution accordingly before release.
