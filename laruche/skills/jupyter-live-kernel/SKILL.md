---
type: skill
name: jupyter-live-kernel
description: "Stateful Python REPL via live Jupyter kernel (hamelnb)."
version: 1.0.0
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec, file_write]
metadata:
  laruche:
    tags: [jupyter, notebook, repl, data-science, exploration, iterative]
    category: data-science
---

# Jupyter Live Kernel (hamelnb)

A **stateful Python REPL** backed by a live Jupyter kernel. Variables, imports,
and objects persist across executions. Use this instead of `execute_code` when
you need incremental state, DataFrame inspection, or "try-and-check" iteration.

All shell commands run via **`shell_exec`**.

## When to Use

| Tool | Use when |
|------|----------|
| **This skill** | Iterative exploration, persistent state, data science, ML |
| `execute_code` | One-shot scripts needing LaRuche tools (web_search, file ops). Stateless. |
| `shell_exec` | Shell commands, installs, git, process management |

Rule of thumb: if you'd reach for a Jupyter notebook, use this skill.

## Prerequisites

1. **uv** installed - `which uv`
2. **JupyterLab** installed - `uv tool install jupyterlab`
3. **hamelnb** helper script (clone once):
   ```
   git clone https://github.com/hamelsmu/hamelnb.git ~/.agent-skills/hamelnb
   ```

Set the script path once per session:
```
SCRIPT="$HOME/.agent-skills/hamelnb/skills/jupyter-live-kernel/scripts/jupyter_live_kernel.py"
```

## Setup

### Start JupyterLab

Check for a running server first:
```
uv run "$SCRIPT" servers
```

If none found, start one (token/password disabled for local agent access):
```
jupyter-lab --no-browser --port=8888 --notebook-dir=$HOME/notebooks \
  --IdentityProvider.token='' --ServerApp.password='' > /tmp/jupyter.log 2>&1 &
sleep 3
```

### Create a Kernel Session

```
mkdir -p ~/notebooks
```

Write a minimal `.ipynb` (one empty code cell) via `file_write`, then create a session:
```
curl -s -X POST http://127.0.0.1:8888/api/sessions \
  -H "Content-Type: application/json" \
  -d '{"path":"scratch.ipynb","type":"notebook","name":"scratch.ipynb","kernel":{"name":"python3"}}'
```

## Core Workflow

Always pass `--compact` to save tokens. All commands return structured JSON.

### 1. Discover servers and notebooks

```
uv run "$SCRIPT" servers --compact
uv run "$SCRIPT" notebooks --compact
```

### 2. Execute code (primary operation)

```
uv run "$SCRIPT" execute --path <notebook.ipynb> --code '<python code>' --compact
```

State persists across calls. Multi-line code - use `$'...'` quoting:
```
uv run "$SCRIPT" execute --path scratch.ipynb \
  --code $'import os\nfiles = os.listdir(".")\nprint(f"Found {len(files)} files")' --compact
```

### 3. Inspect live variables

```
uv run "$SCRIPT" variables --path <notebook.ipynb> list --compact
uv run "$SCRIPT" variables --path <notebook.ipynb> preview --name <varname> --compact
```

### 4. Edit notebook cells

```
# View current cells
uv run "$SCRIPT" contents --path <notebook.ipynb> --compact

# Insert a cell
uv run "$SCRIPT" edit --path <notebook.ipynb> insert \
  --at-index <N> --cell-type code --source '<code>' --compact

# Replace cell source (cell-id from contents output)
uv run "$SCRIPT" edit --path <notebook.ipynb> replace-source \
  --cell-id <id> --source '<new code>' --compact

# Delete a cell
uv run "$SCRIPT" edit --path <notebook.ipynb> delete --cell-id <id> --compact
```

### 5. Clean verification (restart + run all)

Only when you need to confirm the notebook runs top-to-bottom cleanly:
```
uv run "$SCRIPT" restart-run-all --path <notebook.ipynb> --save-outputs --compact
```

## Pitfalls

1. **Argument order matters** - subcommand flags like `--path` go BEFORE the
   sub-subcommand: `variables --path nb.ipynb list`, NOT `variables list --path nb.ipynb`.

2. **First execution after server start may timeout** - kernel needs a moment to
   initialize. Retry once. Same for websocket timeouts after a kernel restart.

3. **Packages must be in JupyterLab's environment** - install into the
   `jupyterlab` tool env before importing in the kernel:
   ```
   uv tool run --from jupyterlab pip install <package>
   ```

4. **Errors return JSON** - read `ename` and `evalue` fields for the traceback.
   Reconstruct with `traceback` array for full context.

5. **Long-running operations** - pass `--timeout 120` (default 30 s).
   Use 60+ for heavy computation or initial kernel setup.

6. **Port conflict** - if 8888 is taken, pass `--port=8889` to `jupyter-lab`
   and update the `curl` base URL accordingly.
