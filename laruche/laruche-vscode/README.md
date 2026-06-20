# LaRuche VS Code Extension

Local AI copilot for VS Code using LAND discovery and LaRuche nodes.

## Features

- Auto-discovery of LAND nodes over mDNS (`_ai-inference._tcp.local.`)
- Local node probe (`127.0.0.1:<apiPort>`) fallback
- Swarm merge from `/swarm` for stable node list and metadata
- Chat panel + sidebar chat
- Selection actions: explain/refactor
- Agent file edit flow with modes (`auto`, `ask`, `readonly`)
- Node picker + model picker
- Status bar with node count and aggregated t/s

## Discovery strategy

The extension combines three sources:

1. `local-probe`: direct health/status probe of localhost
2. `mdns`: LAND announcements via `bonjour-service`
3. `swarm`: periodic `/swarm` polling from active node

Stability protections:

- Grace delay before removing a node after transient mDNS `down`
- Endpoint dedupe (host/port normalization)
- Grace polls for temporary `/swarm` misses

Current defaults used by the extension:

- mDNS node-loss grace: `12s`
- Swarm stale grace: `6` polls
- Local probe timeout: `2000ms`
- Health endpoint accepted as plain text (`OK`) or JSON-like success
- `/swarm` node `port` is used when present (no forced `8419`)

## Commands

- `LaRuche: Ask`
- `LaRuche: Explain Selection`
- `LaRuche: Refactor Selection`
- `LaRuche: Open Chat Panel`
- `LaRuche: Show Swarm Status`
- `LaRuche: Select Active Node`
- `LaRuche: Select Active Model`
- `LaRuche: Agent: Edit File`
- `LaRuche: Agent: Undo Last Edit`
- `LaRuche: Agent: Edit History`

## Settings

- `laruche.nodeUrl` (string, default `""`): manual node URL override
- `laruche.apiPort` (number, default `8419`): localhost probe port
- `laruche.model` (string, default `""`): model override
- `laruche.agentMode` (`auto|ask|readonly`, default `ask`)

## Node API assumptions

Expected endpoints on active node:

- `GET /health` (plain text `OK` or JSON-like success)
- `GET /` (node status)
- `GET /swarm` (swarm aggregate)
- `GET /models` (model list)
- `POST /infer` (inference)

## Development

```bash
npm install
npm run compile
# optional
npm run watch
```

Run extension in dev host from VS Code (`F5`).

## Packaging

```bash
npm install -g @vscode/vsce
vsce package
```

Install:

```bash
code --install-extension laruche-vscode-<version>.vsix
```
