---
type: skill
name: touchdesigner-mcp
description: "Control TouchDesigner via twozero MCP: create ops, set params, run Python, capture visuals."
version: 1.1.0
author: kshitijk4poor
license: MIT
platforms: [linux, macos, windows]
tools: [tool_call, shell_exec, file_write]
scripts: [scripts/setup.sh]
metadata:
  laruche:
    tags: [TouchDesigner, MCP, twozero, creative-coding, real-time-visuals, generative-art, audio-reactive, VJ, installation, GLSL]
    related_skills: [ascii-video, manim-video]

---

# TouchDesigner Integration (twozero MCP)

## CRITICAL RULES

1. **NEVER guess parameter names.** Call `td_get_par_info` for the op type FIRST. Training data is wrong for TD 2025.32.
2. **If `tdAttributeError` fires, STOP.** Call `td_get_operator_info` on the failing node before continuing.
3. **NEVER hardcode absolute paths** in script callbacks. Use `me.parent()` / `scriptOp.parent()`.
4. **Prefer native MCP tools over `td_execute_python`.** Use `td_create_operator`, `td_set_operator_pars`, `td_get_errors` etc. Fall back to `td_execute_python` only for complex multi-step logic.
5. **Call `td_get_hints` before building.** Returns patterns specific to the op type you're working with.

## Architecture

```
LaRuche (tool_call) -> MCP (Streamable HTTP) -> twozero.tox (port 40404) -> TD Python
```

36 native tools. Free plugin (no payment/license - confirmed April 2026).
Context-aware: knows selected OP and current network.
Health check: `GET http://localhost:40404/mcp` returns JSON with instance PID, project name, TD version.

## Setup

Run the bundled setup script (idempotent):

```bash
bash "skills/touchdesigner-mcp/scripts/setup.sh"
```

The script will:
1. Check if TD is running
2. Download `twozero.tox` to `~/Downloads/` if missing
3. Test the MCP connection on port 40404
4. Report remaining manual steps

### Manual steps (one-time, cannot be automated)

1. **Drag `~/Downloads/twozero.tox` into the TD network editor** → click Install
2. **Enable MCP:** twozero icon → Settings → mcp → "auto start MCP" → Yes
3. **Reconnect LaRuche MCP session** to pick up the new server

Verify connection:
```bash
nc -z 127.0.0.1 40404 && echo "twozero MCP: READY"
```

## Environment Notes

- **Non-Commercial TD** caps resolution at 1280×1280. Use `outputresolution = 'custom'` and set width/height explicitly.
- **Codecs:** `prores` (preferred on macOS) or `mjpa` as fallback. H.264/H.265/AV1 require a Commercial license.
- Always call `td_get_par_info` before setting params - names vary by TD version.

## Workflow

### Step 0: Discover (before building anything)

```
td_get_par_info(op_type=<type>)   # for each op type you plan to use
td_get_hints(topic=<topic>)        # patterns for "glsl", "audio reactive", "feedback", etc.
td_get_focus()                     # see current network + selected OP
td_get_network(path="/project1")   # inspect what already exists
```

### Step 1: Clean + Build

**Split cleanup and creation into SEPARATE MCP calls.** Destroying and recreating same-named nodes in one `td_execute_python` causes "Invalid OP object" errors.

Use `td_create_operator` per node (handles viewport positioning automatically):

```
td_create_operator(type="noiseTOP", parent="/project1", name="bg", parameters={"resolutionw": 1280, "resolutionh": 720})
td_create_operator(type="levelTOP", parent="/project1", name="brightness")
td_create_operator(type="nullTOP",  parent="/project1", name="out")
```

For bulk creation + wiring, use `td_execute_python`:

```python
root = op('/project1')
nodes = []
for name, optype in [('bg', noiseTOP), ('fx', levelTOP), ('out', nullTOP)]:
    n = root.create(optype, name)
    nodes.append(n.path)
for i in range(len(nodes) - 1):
    op(nodes[i]).outputConnectors[0].connect(op(nodes[i+1]).inputConnectors[0])
result = {'created': nodes}
```

### Step 2: Set Parameters

Prefer native tool (validates params, won't crash):

```
td_set_operator_pars(path="/project1/bg", parameters={"roughness": 0.6, "monochrome": true})
```

For expressions or modes, use `td_execute_python`:

```python
op('/project1/time_driver').par.colorr.expr = "absTime.seconds % 1000.0"
```

### Step 3: Wire

No native wire tool exists - use `td_execute_python`:

```python
op('/project1/bg').outputConnectors[0].connect(op('/project1/fx').inputConnectors[0])
```

### Step 4: Verify

```
td_get_errors(path="/project1", recursive=true)
td_get_perf()
td_get_operator_info(path="/project1/out", detail="full")
```

### Step 5: Display / Capture

```
td_get_screenshot(path="/project1/out")
```

Or open a window via script:

```python
win = op('/project1').create(windowCOMP, 'display')
win.par.winop  = op('/project1/out').path
win.par.winw   = 1280
win.par.winh   = 720
win.par.winopen.pulse()
```

## MCP Tool Quick Reference

**Core (use these most):**
| Tool | What |
|------|------|
| `td_execute_python` | Run arbitrary Python in TD. Full API access. |
| `td_create_operator` | Create node with params + auto-positioning |
| `td_set_operator_pars` | Set params safely (validates, won't crash) |
| `td_get_operator_info` | Inspect one node: connections, params, errors |
| `td_get_operators_info` | Inspect multiple nodes in one call |
| `td_get_network` | See network structure at a path |
| `td_get_errors` | Find errors/warnings recursively |
| `td_get_par_info` | Get param names for an OP type |
| `td_get_hints` | Get patterns/tips before building |
| `td_get_focus` | What network is open, what's selected |

**Read/Write:**
| Tool | What |
|------|------|
| `td_read_dat` | Read DAT text content |
| `td_write_dat` | Write/patch DAT content |
| `td_read_chop` | Read CHOP channel values |
| `td_read_textport` | Read TD console output |

**Visual:**
| Tool | What |
|------|------|
| `td_get_screenshot` | Capture one OP viewer to file |
| `td_get_screenshots` | Capture multiple OPs at once |
| `td_get_screen_screenshot` | Capture actual screen via TD |
| `td_navigate_to` | Jump network editor to an OP |

**Search:**
| Tool | What |
|------|------|
| `td_find_op` | Find ops by name/type across project |
| `td_search` | Search code, expressions, string params |

**System:**
| Tool | What |
|------|------|
| `td_get_perf` | Performance profiling (FPS, slow ops) |
| `td_list_instances` | List all running TD instances |
| `td_get_docs` | In-depth docs on a TD topic |
| `td_agents_md` | Read/write per-COMP markdown docs |
| `td_reinit_extension` | Reload extension after code edit |
| `td_clear_textport` | Clear console before debug session |

**Input Automation:**
| Tool | What |
|------|------|
| `td_input_execute` | Send mouse/keyboard to TD |
| `td_input_status` | Poll input queue status |
| `td_input_clear` | Stop input automation |
| `td_op_screen_rect` | Get screen coords of a node |
| `td_click_screen_point` | Click a point in a screenshot |
| `td_screen_point_to_global` | Convert screenshot pixel to absolute screen coords |

Admin/dev utilities (`td_project_quit`, `td_test_session`, `td_dev_log`, `td_clear_dev_log`) - see `references/mcp-tools.md` for full 36-tool reference with parameter schemas.

## Key Implementation Rules

**GLSL time:** No `uTDCurrentTime` in GLSL TOP. Use the Values page:
```python
# Confirm param names first: td_get_par_info(op_type="glslTOP")
td_set_operator_pars(path="/project1/shader", parameters={"value0name": "uTime"})
# op('/project1/shader').par.value0.expr = "absTime.seconds"
# In GLSL: uniform float uTime;
```

Fallback: Constant TOP in `rgba32float` format (8-bit clamps to 0-1, freezing the shader).

**Feedback TOP:** Use `top` parameter reference, not direct input wire. "Not enough sources" resolves after first cook. "Cook dependency loop" warning is expected.

**Resolution:** Non-Commercial caps at 1280×1280. Use `outputresolution = 'custom'`.

**Large shaders:** Write GLSL via `file_write` to a temp path, then load with `td_write_dat` or `td_execute_python`.

**Vertex/Point access (TD 2025.32):** `point.P[0]`, `point.P[1]`, `point.P[2]` - NOT `.x`, `.y`, `.z`.

**Extensions:** `ext0object` format is `"op('./datName').module.ClassName(me)"` in CONSTANT mode. After editing extension code with `td_write_dat`, call `td_reinit_extension`.

**Script callbacks:** ALWAYS use relative paths via `me.parent()` / `scriptOp.parent()`.

**Cleaning nodes:** Always `list(root.children)` before iterating + `child.valid` check.

## Recording / Exporting Video

```python
# via td_execute_python:
root = op('/project1')
rec = root.create(moviefileoutTOP, 'recorder')
op('/project1/out').outputConnectors[0].connect(rec.inputConnectors[0])
rec.par.type       = 'movie'
rec.par.file       = '/tmp/output.mov'
rec.par.videocodec = 'prores'   # Apple ProRes - not license-restricted on macOS
rec.par.record     = True       # start; set False in a separate call to stop
```

H.264/H.265/AV1 need Commercial license. Use `prores` on macOS or `mjpa` as fallback.
Extract frames: `ffmpeg -i /tmp/output.mov -vframes 120 /tmp/frames/frame_%06d.png`

**TOP.save() is useless for animation** - captures the same GPU texture every time. Always use MovieFileOut.

### Before Recording: Checklist

1. **Verify FPS > 0** via `td_get_perf`. FPS=0 → recording will be empty.
2. **Verify output is not black** via `td_get_screenshot`. Black = shader error or missing input.
3. **If recording with audio:** cue audio first, then delay recording start by 3 frames.
4. **Set output path before `rec.par.record = True`** - setting both in the same script can race.

## Audio-Reactive GLSL (Proven Recipe)

### Signal chain (tested April 2026)

```
AudioFileIn CHOP (playmode=sequential)
  → AudioSpectrum CHOP (FFT=512, outputmenu=setmanually, outlength=256, timeslice=ON)
  → Math CHOP (gain=10)
  → CHOP to TOP (dataformat=r, layout=rowscropped)
  → GLSL TOP input 1   (spectrum texture, 256×2)

Constant TOP (rgba32float, time) → GLSL TOP input 0
GLSL TOP → Null TOP → MovieFileOut
```

### Critical audio-reactive rules

1. **TimeSlice must stay ON** for AudioSpectrum. OFF = processes entire file → 24000+ samples → CHOP to TOP overflow.
2. **Set Output Length manually** to 256: `outputmenu='setmanually'`, `outlength=256`. Default outputs 22050 samples.
3. **DO NOT use Lag CHOP for spectrum smoothing.** Lag CHOP timeslice expansion averages 256 samples to near-zero (~1e-06). Shader receives no usable data. This was the #1 audio sync failure in testing.
4. **DO NOT use Filter CHOP either** - same timeslice expansion problem.
5. **Smooth in the GLSL shader** via temporal lerp with a feedback texture: `mix(prevValue, newValue, 0.3)`. Frame-perfect sync, zero pipeline latency.
6. **CHOP to TOP:** `dataformat='r'`, `layout='rowscropped'`. Spectrum output is 256×2 (stereo). Sample at `y=0.25` for first channel.
7. **Math gain = 10** (not 5). Raw bass spectrum values are ~0.19. Gain of 10 → usable ~5.0 in shader.
8. **No Resample CHOP needed.** Control output size via AudioSpectrum's `outlength` directly.

### GLSL spectrum sampling

```glsl
// Input 0 = time (1x1 rgba32float), Input 1 = spectrum (256x2)
float iTime = texture(sTD2DInputs[0], vec2(0.5)).r;

// Average two points per band for stability
float bass = (texture(sTD2DInputs[1], vec2(0.02, 0.25)).r +
              texture(sTD2DInputs[1], vec2(0.05, 0.25)).r) / 2.0;
float mid  = (texture(sTD2DInputs[1], vec2(0.20, 0.25)).r +
              texture(sTD2DInputs[1], vec2(0.35, 0.25)).r) / 2.0;
float hi   = (texture(sTD2DInputs[1], vec2(0.60, 0.25)).r +
              texture(sTD2DInputs[1], vec2(0.80, 0.25)).r) / 2.0;
```

See `references/network-patterns.md` for complete build scripts + full shader code.

## Operator Quick Reference

| Family | Color | Python class / MCP type | Suffix |
|--------|-------|-------------------------|--------|
| TOP | Purple | noiseTOP, glslTOP, compositeTOP, levelTop, blurTOP, textTOP, nullTOP | TOP |
| CHOP | Green | audiofileinCHOP, audiospectrumCHOP, mathCHOP, lfoCHOP, constantCHOP | CHOP |
| SOP | Blue | gridSOP, sphereSOP, transformSOP, noiseSOP | SOP |
| DAT | White | textDAT, tableDAT, scriptDAT, webserverDAT | DAT |
| MAT | Yellow | phongMAT, pbrMAT, glslMAT, constMAT | MAT |
| COMP | Gray | geometryCOMP, containerCOMP, cameraCOMP, lightCOMP, windowCOMP | COMP |

## Security Notes

- MCP runs on localhost only (port 40404). No authentication - any local process can send commands.
- `td_execute_python` has unrestricted access to TD Python environment and filesystem as the TD process user.
- `scripts/setup.sh` downloads twozero.tox from the official `404zero.com` URL. Verify the download hash if security matters.
- All MCP communication is local - no data leaves the machine.

## References

| File | What |
|------|------|
| `references/pitfalls.md` | Hard-won lessons from real sessions |
| `references/operators.md` | All operator families with params and use cases |
| `references/network-patterns.md` | Recipes: audio-reactive, generative, GLSL, instancing |
| `references/mcp-tools.md` | Full twozero MCP tool parameter schemas |
| `references/python-api.md` | TD Python: op(), scripting, extensions |
| `references/troubleshooting.md` | Connection diagnostics, debugging |
| `references/glsl.md` | GLSL uniforms, built-in functions, shader templates |
| `references/postfx.md` | Post-FX: bloom, CRT, chromatic aberration, feedback glow |
| `references/layout-compositor.md` | HUD layout patterns, panel grids, BSP-style layouts |
| `references/operator-tips.md` | Wireframe rendering, feedback TOP setup |
| `references/geometry-comp.md` | Geometry COMP: instancing, POP vs SOP, morphing |
| `references/audio-reactive.md` | Audio band extraction, beat detection, envelope following |
| `references/animation.md` | LFOs, timers, keyframes, easing, expression-driven motion |
| `references/midi-osc.md` | MIDI/OSC controllers, TouchOSC, multi-machine sync |
| `references/particles.md` | POPs and legacy particleSOP - emission, forces, collisions |
| `references/projection-mapping.md` | Multi-window output, corner pin, mesh warp, edge blending |
| `references/external-data.md` | HTTP, WebSocket, MQTT, Serial, TCP, webserverDAT |
| `references/panel-ui.md` | Custom params, panel COMPs, button/slider/field, panelExecuteDAT |
| `references/replicator.md` | replicatorCOMP - data-driven cloning, layouts, callbacks |
| `references/dat-scripting.md` | Execute DAT family - chop/dat/parameter/panel/op/executeDAT |
| `references/3d-scene.md` | Lighting rigs, shadows, IBL/cubemaps, multi-camera, PBR |
| `scripts/setup.sh` | Automated setup script |
