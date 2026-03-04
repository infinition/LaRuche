# LaRuche

Local-first AI node + swarm stack built on the LAND protocol.

## What is in this repo

- `laruche-node`: main daemon (HTTP API, Ollama bridge, LAND mDNS announce/listen)
- `laruche-dashboard`: standalone dashboard binary (HTML UI)
- `laruche-client`: Rust SDK (`discover`, `connect`, `ask`, capability routing)
- `laruche-cli`: CLI wrapper for quick usage
- `laruche-vscode`: VS Code extension (chat, code actions, node/model selection)

## Runtime behavior (current)

- Node announces itself on LAND (`_ai-inference._tcp.local.`).
- Node re-announces mDNS metadata every `2s`.
- LAND listener stale timeout is `45s` (in `land-protocol`).
- Dashboard keeps transiently missing nodes for `10` swarm polls before removing.
- `/swarm` now returns `host` + `port` per node.
- Capability labels are normalized (`llm`, `code`, etc.), not `capability:llm` in API payloads.

## Prerequisites

- Rust stable toolchain
- Ollama running locally or reachable from the node (`OLLAMA_URL`)

## Quick start

1. Pull a model:

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
