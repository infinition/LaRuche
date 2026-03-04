<div align="center">
<img width="3533" height="997" alt="image" src="https://github.com/user-attachments/assets/458f9822-8c9e-44da-896c-20ba238925d3" />
</div>

# LaRuche - Networked Edge AI System

<div align="center">
  <img width="287" height="317" alt="icon-removebg-preview" src="https://github.com/user-attachments/assets/a6a37836-7e97-4203-b041-1edb7ec36263" />
</div>

**"Plug in AI. That's it."**

Plug a LaRuche node into your local network and AI becomes available to connected devices.
Zero configuration, zero cloud dependency, and privacy-first by design.

## Architecture

```text
Local network

  LaRuche Core (LLM/RAG) <---- LAND protocol (mDNS + HTTP) ----> LaRuche Pro (VLM/Code)
            |                                                           |
            +---------------------- Swarm intelligence ------------------+

Clients
  - VS Code extension
  - Web UI
  - CLI / SDK
  - IoT integrations
```

## Workspace structure

```text
laruche/
|-- land-protocol/        # Core LAND protocol library
|   `-- src/
|       |-- lib.rs
|       |-- capabilities.rs
|       |-- manifest.rs
|       |-- discovery.rs
|       |-- auth.rs
|       |-- qos.rs
|       |-- swarm.rs
|       `-- error.rs
|
|-- laruche-node/         # Node daemon
|   `-- src/main.rs
|
|-- laruche-client/       # Rust client SDK
|   `-- src/lib.rs
|
|-- laruche-cli/          # CLI tool
|   `-- src/main.rs
|
`-- laruche-dashboard/    # Web dashboard
    `-- src/
        |-- main.rs
        `-- templates/dashboard.html
```

## Quick start

### Prerequisites

- Rust 1.75+
- Ollama running locally or on your LAN

Windows notes:

- Option 1 (recommended): MSVC toolchain
  - Install Visual Studio Build Tools with "Desktop development with C++"
  - `rustup default stable-x86_64-pc-windows-msvc`
- Option 2: GNU/MSYS2
  - Install MSYS2 and MinGW binutils
  - Add `C:\msys64\mingw64\bin` to `PATH`
  - `rustup default stable-x86_64-pc-windows-gnu`

### 1. Pull a model

```bash
ollama pull mistral
```

### 2. Build

```bash
cargo check --workspace
```

### 3. Run a node

```bash
cargo run -p laruche-node
```

### 4. Open the dashboard

```text
http://localhost:8419/dashboard
```

## Current implementation notes

- LAND mDNS re-announce is periodic (`2s`) to keep nodes visible.
- LAND listener stale timeout is `45s` (from `land-protocol`).
- `/swarm` includes node `port` and merged capabilities (HTTP + mDNS fallback).
- `/health` returns plain text `OK`.
- Dashboard keeps transiently missing nodes for `10` polls before removal.
- VS Code extension uses:
  - mDNS node-loss grace: `12s`
  - swarm stale grace: `6` polls

## Node configuration

The node loads configuration in this order:

1. `laruche.toml` (or `LARUCHE_CONFIG=<path>`)
2. Environment variables (override file values)

### Environment variables

- `LARUCHE_NAME` (default random `laruche-xxxxxx`)
- `LARUCHE_TIER` (`nano|core|pro|max`)
- `LARUCHE_PORT` (default `8419`)
- `LARUCHE_DASH_PORT` (default `8420`)
- `OLLAMA_URL` (default `http://127.0.0.1:11434`)
- `LARUCHE_MODEL` (default model name)
- `LARUCHE_CAP` (primary capability, example: `llm`)
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

- `GET /` - Node status (CPU/RAM, queue, capabilities)
- `GET /health` - Health check (`OK`)
- `GET /nodes` - Discovered peers (mDNS view)
- `GET /swarm` - Collective view (self + peers, with `port`)
- `GET /models` - Local Ollama models
- `GET /swarm/models` - Models across swarm
- `POST /infer` - Inference request
- `GET /activity` - Recent node activity log
- `POST /auth/request` - Auth request (POC)
- `POST /auth/approve` - Approve auth request (POC)
- `GET /dashboard` - Embedded dashboard

### Example inference request

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

```bash
cd laruche-vscode
npm install
npm run compile
```

Then launch the Extension Development Host with `F5` from VS Code.

## Rust client SDK

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
- If you iterate locally on protocol changes, update dependency resolution before release.
