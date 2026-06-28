---
type: skill
name: ascii-video
title: ASCII Video Production Pipeline
version: "1.0"
platforms: [linux, macos, windows]
description: "Convert video/audio/generative input into ASCII art video via Python+ffmpeg."
dependencies:
  - python>=3.10
  - numpy
  - scipy
  - pillow
  - ffmpeg
tools:
  - shell_exec
  - execute_code
  - file_write
  - file_read
  - file_list
---

# ASCII Video Production Pipeline

## When to use

User requests: ASCII video, text art animation, terminal-style video, character art animation, retro text visualization, audio visualizer in ASCII, matrix effects, animated ASCII output.

## Modes

| Mode | Input | Output |
|------|-------|--------|
| **Video-to-ASCII** | Video file | ASCII recreation of source footage |
| **Audio-reactive** | Audio file | Generative visuals driven by audio features |
| **Generative** | None / seed params | Procedural ASCII animation |
| **Hybrid** | Video + audio | ASCII video with audio-reactive overlays |
| **Lyrics/text** | Audio + text/SRT | Timed text with visual effects |
| **TTS narration** | Text + TTS API | Narrated video with typed text |

## Stack

Single self-contained Python script per project. No GPU required.

| Layer | Tool | Purpose |
|-------|------|---------|
| Core | Python 3.10+, NumPy | Math, vectorized effects |
| Signal | SciPy | FFT, peak detection (audio modes) |
| Imaging | Pillow | Font rasterization, frame decoding |
| Video I/O | ffmpeg (CLI) | Decode input, encode output, mux audio |
| Parallel | concurrent.futures | N workers for batch/clip rendering |
| TTS | ElevenLabs API (optional) | Narration clips via `${ELEVENLABS_API_KEY}` |
| Optional | OpenCV | Video frame sampling, edge detection |

Install: `pip install numpy scipy pillow opencv-python` (ffmpeg must be in PATH).

## Pipeline

```
INPUT → ANALYZE → SCENE_FN → TONEMAP → SHADE → ENCODE
```

1. **INPUT** - Load/decode source (video frames, audio, images, or synthetic)
2. **ANALYZE** - Extract per-frame features (audio bands, luminance/edges, motion)
3. **SCENE_FN** - Render to pixel canvas (`uint8 H,W,3`); compose character grids via `_render_vf()` + blend modes
4. **TONEMAP** - Percentile-based adaptive brightness normalization
5. **SHADE** - Post-processing via `ShaderChain` + `FeedbackBuffer`
6. **ENCODE** - Pipe raw RGB frames to ffmpeg for H.264/GIF encoding

## Workflow

### Step 1: Creative Vision

Before writing code, define:
- **Mood**: energetic / meditative / chaotic / elegant / ominous
- **Visual story**: what changes over time (tension build, transformation, dissolution)
- **Color world**: warm/cool, neon, monochrome, earth tones
- **Character texture**: dense data, sparse stars, organic, geometric
- **Per-scene variation**: each scene must differ in background effect, character palette, color strategy, and shader intensity - never use the same config throughout

### Step 2: Technical Design

- **Mode** - pick from the 6 modes above
- **Resolution** - landscape 1920×1080 (default), portrait 1080×1920, square 1080×1080 @ 24fps
- **Hardware detection** - auto-detect cores/RAM, set quality profile
- **Section map** - map timestamps to scene functions with their own effect/palette/color/shader config
- **Output format** - MP4 (default), GIF (640×360 @ 15fps), PNG sequence

### Step 3: Build the Script

Single Python file. Components in order:

1. Hardware detection + quality profile
2. Input loader - mode-dependent
3. Feature analyzer - audio FFT, video luminance, or synthetic
4. Grid + renderer - multi-density grids with bitmap cache
5. Character palettes - multiple per project
6. Color system - HSV + discrete RGB + harmony generation
7. Scene functions - each returns `canvas (uint8 H,W,3)`
8. Tonemap - adaptive brightness normalization
9. Shader pipeline - `ShaderChain` + `FeedbackBuffer`
10. Scene table + dispatcher - time → scene function + config
11. Parallel encoder - N-worker clip rendering with ffmpeg pipes
12. Main - orchestrate full pipeline

Run via `shell_exec`: `python render.py`

### Step 4: Quality Verification

- **Test frames first**: render single frames at key timestamps before full render
- **Brightness check**: `canvas.mean() > 8` for all ASCII content; if dark, lower gamma
- **Visual coherence**: all scenes must feel connected by unified color temperature, character palette family, and motion vocabulary
- **Creative check**: if output looks generic or flat, revise scene functions before shipping
- **Verify output**: use `file_list` to confirm output files exist and have non-zero size

## Critical Pitfalls

### Brightness - Use `tonemap()`, Not Linear Multipliers

ASCII on black is inherently dark. Never use `canvas * N` multipliers - they clip highlights. Use adaptive tonemap:

```python
def tonemap(canvas, gamma=0.75):
    f = canvas.astype(np.float32)
    lo, hi = np.percentile(f[::4, ::4], [1, 99.5])
    if hi - lo < 10: hi = lo + 10
    f = np.clip((f - lo) / (hi - lo), 0, 1) ** gamma
    return (f * 255).astype(np.uint8)
```

Pipeline order: `scene_fn() → tonemap() → FeedbackBuffer → ShaderChain → ffmpeg`

Per-scene gamma defaults: 0.75 standard, 0.55 solarize, 0.50 posterize, 0.85 bright scenes. Use `screen` blend (not `overlay`) for dark layers.

### Font Cell Height

macOS Pillow: `textbbox()` returns wrong height. Use `font.getmetrics()`: `cell_height = ascent + descent`.

### ffmpeg Pipe Deadlock

Never `stderr=subprocess.PIPE` with long-running ffmpeg - buffer fills at 64KB and deadlocks. Redirect stderr to a file:

```python
ffmpeg_proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stderr=open("ffmpeg.log", "w"))
```

### Font Compatibility

Not all Unicode chars render in all fonts. Validate palettes at init - render each char, check for blank output.

### Per-Clip Architecture

For segmented videos (quotes, scenes, chapters), render each as a separate clip for parallel rendering and selective re-rendering.

## Performance Targets

| Component | Budget |
|-----------|--------|
| Feature extraction | 1–5ms |
| Effect function | 2–15ms |
| Character render | 80–150ms (bottleneck) |
| Shader pipeline | 5–25ms |
| **Total** | ~100–200ms/frame |

## References

| File | Contents |
|------|----------|
| `references/architecture.md` | Grid system, resolution presets, font selection, character palettes (20+), color system, `_render_vf()`, GridLayer |
| `references/composition.md` | Blend modes (20), `blend_canvas()`, multi-grid composition, `tonemap()`, `FeedbackBuffer`, masking/stencil |
| `references/effects.md` | Noise/fBM/domain warp, voronoi, reaction-diffusion, cellular automata, SDFs, attractors, particles |
| `references/shaders.md` | `ShaderChain`, 38-shader catalog, audio-reactive scaling, transitions, tint presets, output encoding |
| `references/scenes.md` | Scene protocol, `Renderer`, `SCENES` table, beat-synced cutting, parallel rendering, design patterns |
| `references/inputs.md` | Audio FFT/bands/beats, video sampling, image conversion, text/lyrics, TTS integration (ElevenLabs) |
| `references/optimization.md` | Hardware detection, quality profiles, vectorized patterns, parallel rendering, memory management |
| `references/troubleshooting.md` | NumPy broadcasting, blend pitfalls, multiprocessing/pickling, brightness diagnostics, ffmpeg/font issues |

## Creative Strategies (for open-ended requests)

- **Forced Connections**: map a domain unrelated to the visual goal (weather, microbiology, textiles) onto ASCII characters and motion patterns.
- **Conceptual Blending**: name two distinct spaces (e.g., ocean waves + sheet music), map correspondences, develop emergent properties.
- **Oblique Strategies**: pick one directive - "Honor thy error", "Use an old idea", "Emphasize the flaws", "Turn it upside down" - and interpret it against the visual challenge before writing code.
