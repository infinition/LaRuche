# LaRuche for VS Code

**Local Edge AI Copilot powered by the LAND Protocol.**

Zero cloud. Zero API keys. Your AI, your network, your data.

## Features

### Chat Panel
Open a dedicated chat panel to interact with your local AI directly from VS Code. The conversation is powered by your LaRuche node running locally.

- **Sidebar Chat:** Available in the activity bar for quick access.
- **Panel Chat:** Open a floating chat panel with `LaRuche: Open Chat Panel`.

### Code Intelligence
Right-click on selected code to access:

- **Explain Selection:** Get a clear explanation of the selected code.
- **Refactor Selection:** Automatically refactor and improve code in-place.

### Swarm Status
The status bar shows your collective AI power in real-time:

- Number of active nodes on your network.
- Combined tokens per second.
- Click to see detailed Swarm status with per-node metrics.

### Quick Ask
Use the command palette (`Ctrl+Shift+P`) and type `LaRuche: Ask` to send any prompt to your local AI.

## Commands

| Command | Description |
|---------|-------------|
| `LaRuche: Ask` | Send a prompt to the best available node |
| `LaRuche: Explain Selection` | Explain the selected code |
| `LaRuche: Refactor Selection` | Refactor and improve the selected code |
| `LaRuche: Open Chat Panel` | Open the chat interface |
| `LaRuche: Show Swarm Status` | View all nodes and collective power |

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `laruche.nodeUrl` | `""` | Direct URL to a LaRuche node (leave empty for localhost) |
| `laruche.apiPort` | `8419` | Default API port for LaRuche nodes |
| `laruche.model` | `""` | Preferred model name (leave empty for node default) |

## Requirements

- A LaRuche node running on your machine or network.
- Install the node: `cargo run -p laruche-node`
- Ollama or another LLM backend configured on the node.

## Architecture

```
  VS Code Extension
       |
       | HTTP (localhost:8419)
       |
  LaRuche Node (laruche-node)
       |
       | Ollama API
       |
  Local LLM (Mistral, Llama, etc.)
```

The extension communicates directly with LaRuche nodes via HTTP.
No data leaves your local network.

---

*Part of the [LaRuche](https://github.com/infinition/LaRuche) ecosystem.
Powered by the [LAND Protocol](https://github.com/infinition/land-protocol).*

## Development

### Setup
Install dependencies:
```bash
npm install
```

### Build
Compile the extension:
```bash
# One-time build
npm run compile

# Watch mode (auto-rebuild on changes)
npm run watch
```

### Packaging & Installation
To create a `.vsix` package and install it:

1. Install the packaging tool:
```bash
npm install -g @vscode/vsce
```

2. Generate the package:
```bash
vsce package
```

3. Install the extension in VS Code:
```bash
code --install-extension laruche-vscode-0.1.0.vsix
```
