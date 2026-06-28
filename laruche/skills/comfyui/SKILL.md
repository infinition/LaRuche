---
type: skill
name: comfyui
description: "ComfyUI image/video/audio: install, launch, run workflows via REST/WS."
version: 5.1.0
author: [kshitijk4poor, alt-glitch, purzbeats]
license: MIT
platforms: [macos, linux, windows]
compatibility: "Requires ComfyUI (local, Comfy Desktop, or Comfy Cloud) and comfy-cli (auto-installed via pipx/uvx by the setup script)."
prerequisites:
  commands: ["python3"]
tools: [shell_exec, file_read, file_write, file_list, media_present]
scripts:
  - scripts/_common.py
  - scripts/hardware_check.py
  - scripts/comfyui_setup.sh
  - scripts/extract_schema.py
  - scripts/check_deps.py
  - scripts/auto_fix_deps.py
  - scripts/run_workflow.py
  - scripts/run_batch.py
  - scripts/ws_monitor.py
  - scripts/health_check.py
  - scripts/fetch_logs.py
setup:
  help: "Run scripts/hardware_check.py FIRST to decide local vs Comfy Cloud; then scripts/comfyui_setup.sh auto-installs locally (or use Cloud API key for platform.comfy.org)."
metadata:
  laruche:
    tags:
      - comfyui
      - image-generation
      - stable-diffusion
      - flux
      - sd3
      - wan-video
      - hunyuan-video
      - creative
      - generative-ai
      - video-generation
    category: creative
---

# ComfyUI

Two-layer architecture: **comfy-cli** for setup/lifecycle, **REST/WebSocket API + bundled scripts** for workflow execution.

## Bundled Scripts

| Script | Purpose |
|--------|---------|
| `_common.py` | Shared HTTP, cloud routing, node catalogs (library, do not run directly) |
| `hardware_check.py` | Probe GPU/VRAM/disk → recommend local vs Comfy Cloud |
| `comfyui_setup.sh` | Hardware check + comfy-cli + ComfyUI install + launch + verify |
| `extract_schema.py` | List controllable params + model deps for a workflow |
| `check_deps.py` | Check workflow against running server → list missing nodes/models |
| `auto_fix_deps.py` | Run check_deps then `comfy node install` / `comfy model download` |
| `run_workflow.py` | Inject params, submit, monitor, download outputs (HTTP or WS) |
| `run_batch.py` | Submit N times with sweeps, parallel up to your tier |
| `ws_monitor.py` | Real-time WebSocket viewer for an executing job |
| `health_check.py` | Full verification: comfy-cli + server + models + smoke test |
| `fetch_logs.py` | Pull traceback/status messages for a given prompt_id |

Example workflows in `workflows/` (SD 1.5, SDXL, Flux Dev, img2img, inpaint, upscale, AnimateDiff, Wan T2V). See `workflows/README.md`.

Reference docs in `references/`: `official-cli.md`, `rest-api.md`, `workflow-format.md`, `template-integrity.md` (load the last one when starting from official templates).

## Decision Table

| User says | Command |
|-----------|---------|
| **Lifecycle** | |
| "install ComfyUI" | `bash scripts/comfyui_setup.sh` |
| "start ComfyUI" | `comfy launch --background` |
| "stop ComfyUI" | `comfy stop` |
| "install X node" | `comfy node install <name>` |
| "download X model" | `comfy model download --url <url> --relative-path models/checkpoints` |
| "list models / nodes" | `comfy model list` / `comfy node show installed` |
| **Execution** | |
| "is everything ready?" | `python3 scripts/health_check.py` |
| "what params does this workflow have?" | `python3 scripts/extract_schema.py W.json` |
| "check deps" | `python3 scripts/check_deps.py W.json` |
| "fix missing deps" | `python3 scripts/auto_fix_deps.py W.json` |
| "generate an image" | `python3 scripts/run_workflow.py --workflow W --args '{...}'` |
| "use this image" (img2img) | `python3 scripts/run_workflow.py --input-image image=./x.png ...` |
| "N variations" | `python3 scripts/run_batch.py --count N --randomize-seed ...` |
| "show live progress" | `python3 scripts/ws_monitor.py --prompt-id <id>` |
| "fetch error from job X" | `python3 scripts/fetch_logs.py <prompt_id>` |
| **Direct REST** | |
| "what's in the queue?" | `curl http://HOST:8188/queue` |
| "cancel running job" | `curl -X POST http://HOST:8188/interrupt` |
| "free GPU memory" | `curl -X POST http://HOST:8188/free` |

## Core Execution Flow

### Step 1: Detect environment

```bash
command -v comfy >/dev/null 2>&1 && echo "comfy-cli: installed"
curl -s http://127.0.0.1:8188/system_stats 2>/dev/null && echo "server: running"
python3 scripts/health_check.py
```

### Step 2: Get a workflow in API format

Workflows must be API format (`class_type` per node). Sources:
- ComfyUI web UI → **Workflow → Export (API)** (newer) or **Save (API Format)** (older)
- `workflows/` directory in this skill
- Community downloads - usually editor format (top-level `nodes`/`links` arrays); load into ComfyUI and re-export

### Step 3: Inspect the workflow

```bash
python3 scripts/extract_schema.py workflow_api.json --summary-only
python3 scripts/check_deps.py workflow_api.json
python3 scripts/auto_fix_deps.py workflow_api.json   # install missing deps
```

### Step 4: Run

```bash
# Local
python3 scripts/run_workflow.py \
  --workflow workflow_api.json \
  --args '{"prompt": "a sunset over mountains", "seed": -1, "steps": 30}' \
  --output-dir ./outputs

# Cloud (set COMFY_CLOUD_API_KEY in vault → substituted as ${COMFY_CLOUD_API_KEY})
python3 scripts/run_workflow.py \
  --workflow workflow_api.json \
  --args '{"prompt": "..."}' \
  --host https://cloud.comfy.org \
  --output-dir ./outputs

# WebSocket (real-time progress; requires pip install websocket-client)
python3 scripts/run_workflow.py --workflow flux_dev.json --args '{"prompt": "..."}' --ws

# img2img / inpainting
python3 scripts/run_workflow.py \
  --workflow sdxl_img2img.json \
  --input-image image=./photo.png \
  --args '{"prompt": "make it watercolor", "denoise": 0.6}'

# img2img + mask (inpaint)
python3 scripts/run_workflow.py \
  --workflow sdxl_inpaint.json \
  --input-image image=./photo.png \
  --input-image mask_image=./mask.png \
  --args '{"prompt": "fill with flowers"}'

# Batch sweep
python3 scripts/run_batch.py \
  --workflow sdxl.json --args '{"prompt": "abstract"}' \
  --count 8 --randomize-seed --parallel 3 --output-dir ./outputs/batch
```

`seed: -1` generates a fresh random seed per run. The actual seed is logged to stderr.

Output JSON (stdout):
```json
{"status": "success", "prompt_id": "abc-123",
 "outputs": [{"file": "./outputs/sdxl_00001_.png", "node_id": "9", "type": "image"}]}
```

## Setup & Onboarding

**FIRST: ask whether the user wants Comfy Cloud or Local.** Do not run installs until they've answered.

Hardware requirements for local:
- NVIDIA GPU ≥6 GB VRAM (≥8 GB for SDXL, ≥12 GB for Flux/video)
- AMD GPU with ROCm (Linux only)
- Apple Silicon M1+ with ≥16 GB unified memory (≥32 GB recommended)
- Intel Macs and no-GPU machines → use Cloud

```bash
# Hardware check (returns verdict: ok / marginal / cloud)
python3 scripts/hardware_check.py --json
```

| Verdict | Action |
|---------|--------|
| `ok` | Local install, use `comfy_cli_flag` from report |
| `marginal` | Local OK for SD1.5; else Cloud |
| `cloud` | Switch to Cloud (or force local - will OOM on modern models) |

The script also surfaces `wsl: true` and `rosetta: true` (x86_64 Python on Apple Silicon - reinstall as ARM64).

### Path A: Comfy Cloud

Docs: https://docs.comfy.org/get_started/cloud

1. Sign up at https://comfy.org/cloud
2. Generate API key at https://platform.comfy.org/login
3. `export COMFY_CLOUD_API_KEY="comfyui-xxxxxxxxxxxx"`
4. Run with `--host https://cloud.comfy.org`

Pricing: https://www.comfy.org/cloud/pricing
Concurrent jobs: Free/Standard 1, Creator 3, Pro 5.
**Free tier cannot run workflows via API** (403 on `/api/prompt`, `/api/upload/*`, `/api/view`, `/api/object_info`). Paid required.

### Path B: ComfyUI Desktop (Windows/macOS, non-technical users)

Docs: https://docs.comfy.org/installation/desktop
- Windows (NVIDIA): https://download.comfy.org/windows/nsis/x64
- macOS (Apple Silicon): https://comfy.org
- Linux: not supported - use Path D.

### Path C: ComfyUI Portable (Windows only)

Docs: https://docs.comfy.org/installation/comfyui_portable_windows
Download from https://github.com/comfyanonymous/ComfyUI/releases, extract, run `run_nvidia_gpu.bat`.

### Path D: comfy-cli (All Platforms - recommended for agents/headless)

Docs: https://docs.comfy.org/comfy-cli/getting-started

```bash
pipx install comfy-cli                  # recommended
# or: uvx --from comfy-cli comfy --help (no install)
# or: pip install --user comfy-cli

comfy --skip-prompt tracking disable

comfy --skip-prompt install --nvidia    # NVIDIA (CUDA)
comfy --skip-prompt install --amd      # AMD (ROCm, Linux)
comfy --skip-prompt install --m-series # Apple Silicon (MPS)
comfy --skip-prompt install --cpu      # CPU only (slow)

comfy launch --background              # daemon on :8188
curl -s http://127.0.0.1:8188/system_stats
```

Default workspace: `~/comfy/ComfyUI` (Linux), `~/Documents/comfy/ComfyUI` (macOS/Win).
Override: `comfy --workspace /custom/path install`.

The setup script automates everything above:
```bash
bash scripts/comfyui_setup.sh
# Overrides: --m-series --port=8190 --workspace=/data/comfy
```

### Path E: Manual Install (unsupported hardware: Ascend, Cambricon, Intel Arc)

Docs: https://docs.comfy.org/installation/manual_install

```bash
git clone https://github.com/comfyanonymous/ComfyUI.git
cd ComfyUI
pip install torch torchvision torchaudio --extra-index-url https://download.pytorch.org/whl/cu130
pip install -r requirements.txt
python main.py
```

### Post-Install: Models

```bash
# SDXL (~6.5 GB)
comfy model download \
  --url "https://huggingface.co/stabilityai/stable-diffusion-xl-base-1.0/resolve/main/sd_xl_base_1.0.safetensors" \
  --relative-path models/checkpoints

# SD 1.5 (~4 GB, good for 6 GB cards)
comfy model download \
  --url "https://huggingface.co/stable-diffusion-v1-5/stable-diffusion-v1-5/resolve/main/v1-5-pruned-emaonly.safetensors" \
  --relative-path models/checkpoints

# Flux Dev fp8 (~12 GB)
comfy model download \
  --url "https://huggingface.co/Comfy-Org/flux1-dev/resolve/main/flux1-dev-fp8.safetensors" \
  --relative-path models/checkpoints

# CivitAI (CIVITAI_TOKEN from vault → substituted as ${CIVITAI_TOKEN})
comfy model download \
  --url "https://civitai.com/api/download/models/128713" \
  --relative-path models/checkpoints \
  --set-civitai-api-token "${CIVITAI_TOKEN}"

comfy model list   # verify
```

### Post-Install: Custom Nodes

```bash
comfy node install comfyui-impact-pack
comfy node install comfyui-animatediff-evolved
comfy node install comfyui-controlnet-aux
comfy node install comfyui-essentials
comfy node update all
comfy node install-deps --workflow=workflow.json   # install all deps a workflow needs
```

## Cloud Specifics

- Auth: `X-API-Key` header (or `?token=KEY` for WebSocket)
- `/api/view` returns 302 to signed URL; scripts follow it and strip the API key before fetching (no key leak to S3/CloudFront)
- `/history` → `/history_v2` on cloud (scripts route automatically)
- `/models/<folder>` → `/experiment/models/<folder>` on cloud (scripts route automatically)
- `clientId` in WebSocket is ignored on cloud - filter by `prompt_id` client-side
- `subfolder` on uploads accepted but ignored (flat namespace)
- `run_batch.py --parallel N` saturates your concurrent-job tier

## Queue Management

```bash
# Local
curl -s http://127.0.0.1:8188/queue | python3 -m json.tool
curl -X POST http://127.0.0.1:8188/queue -d '{"clear": true}'
curl -X POST http://127.0.0.1:8188/interrupt
curl -X POST http://127.0.0.1:8188/free \
  -H "Content-Type: application/json" \
  -d '{"unload_models": true, "free_memory": true}'

# Cloud logs
python3 scripts/fetch_logs.py --tail-queue --host https://cloud.comfy.org
```

## Pitfalls

1. **API format required** - scripts and `/api/prompt` reject editor format (top-level `nodes`/`links` arrays). Re-export via "Workflow → Export (API)" or "Save (API Format)".
2. **Server must be running** - `comfy launch --background`, then verify with `curl http://127.0.0.1:8188/system_stats`.
3. **Model names are exact** - case-sensitive, include file extension. Use `comfy model list` to discover canonical names; `check_deps.py` does fuzzy matching.
4. **Missing custom nodes** - "class_type not found" = node not installed. `check_deps.py` identifies the package; `auto_fix_deps.py` installs it.
5. **Workspace not found** - use `comfy --workspace /path/to/ComfyUI <command>` or `comfy set-default /path/to/ComfyUI`.
6. **Cloud free-tier 403** - `/api/prompt`, `/api/view`, `/api/upload/*`, `/api/object_info` all return 403 on free accounts. `health_check.py` surfaces a clear message.
7. **Video/audio timeout** - auto-detected (output nodes `VHS_VideoCombine`, `SaveVideo`, etc.) and extended to 900 s. Override with `--timeout 1800`.
8. **Path traversal protection** - server-supplied filenames pass through `safe_path_join`; do not disable this - custom save nodes can emit arbitrary paths.
9. **Workflow trust** - custom nodes run arbitrary Python. Inspect unknown workflows before executing.
10. **Tracking prompt** - first `comfy` run may prompt for analytics. Use `comfy --skip-prompt tracking disable` (or `comfyui_setup.sh` does it for you).
11. **Rosetta Python on Apple Silicon** - `rosetta: true` in hardware check means x86_64 Python. Reinstall Python as ARM64 before proceeding.

## Verification Checklist

```bash
python3 scripts/health_check.py   # runs all checks at once
```

Manual:
- [ ] `hardware_check.py` verdict is `ok` OR user chose Comfy Cloud
- [ ] `comfy --version` works (or `uvx --from comfy-cli comfy --help`)
- [ ] `curl http://HOST:PORT/system_stats` returns JSON
- [ ] At least one checkpoint installed (`comfy model list` or cloud `/api/experiment/models/checkpoints`)
- [ ] Workflow JSON is in API format
- [ ] `check_deps.py` reports `is_ready: true`
- [ ] Test run completes; outputs land in `--output-dir`
