---
type: skill
name: heartmula
title: HeartMuLa Music Generation
description: "Generate full songs from lyrics + tags (open-source Suno, 3B/7B)."
version: 1.0.0
license: Apache-2.0
platforms: [linux, macos, windows]
tools: [shell_exec, file_edit]
metadata:
  laruche:
    tags: [music, audio, generation, ai, heartmula, heartcodec, lyrics, songs]
    homepage: https://github.com/HeartMuLa/heartlib
---

# HeartMuLa — Open-Source Music Generation

HeartMuLa (Apache-2.0) generates full songs conditioned on lyrics and tags, multilingual. Comparable to Suno for open-source. Components:
- **HeartMuLa** — Music LM (3B/7B), generates from lyrics + tags
- **HeartCodec** — 12.5Hz music codec for high-fidelity audio
- **HeartTranscriptor** — Whisper-based lyrics transcription
- **HeartCLAP** — Audio-text alignment model

## Hardware

| Setup | VRAM |
|-------|------|
| Minimum (3B + `--lazy_load true`) | ~6.2 GB peak |
| Recommended (single GPU) | 16 GB+ |
| Multi-GPU | `--mula_device cuda:0 --codec_device cuda:1` |

No GPU: use `--mula_device cpu --codec_device cpu` — expect 30-60+ min/song, ~12 GB+ RAM. Prefer Colab T4 or the online demo: https://heartmula.github.io/

## Installation

```bash
git clone https://github.com/HeartMuLa/heartlib.git
cd heartlib
uv venv --python 3.10 .venv
. .venv/bin/activate
uv pip install -e .
```

### Fix dependency conflicts (as of Feb 2026)

```bash
uv pip install --upgrade datasets transformers
```

- Old `datasets` conflicts with current `pyarrow`
- Old `transformers` conflicts with `huggingface-hub` 1.x

### Patch 1 — RoPE cache fix

File: `src/heartlib/heartmula/modeling_heartmula.py`

In `HeartMuLa.setup_caches()`, insert after the `reset_caches` try/except block and before the `with device:` block:

```python
# Re-initialize RoPE caches skipped during meta-device loading
from torchtune.models.llama3_1._position_embeddings import Llama3ScaledRoPE
for module in self.modules():
    if isinstance(module, Llama3ScaledRoPE) and not module.is_cache_built:
        module.rope_init()
        module.to(device)
```

**Why:** `from_pretrained` places model on meta device first; `rope_init()` skips building on meta tensors and never rebuilds after weights load to real device.

### Patch 2 — HeartCodec loading fix

File: `src/heartlib/pipelines/music_generation.py`

Add `ignore_mismatched_sizes=True` to **both** `HeartCodec.from_pretrained()` calls (eager load in `__init__` and lazy load in `codec` property).

**Why:** VQ codebook `initted` buffers have shape `[1]` in checkpoint vs `[]` in model — same data, scalar vs 0-d tensor, safe to ignore.

### Download model checkpoints

Run in parallel — total size is several GB:

```bash
hf download --local-dir './ckpt' 'HeartMuLa/HeartMuLaGen'
hf download --local-dir './ckpt/HeartMuLa-oss-3B' 'HeartMuLa/HeartMuLa-oss-3B-happy-new-year'
hf download --local-dir './ckpt/HeartCodec-oss' 'HeartMuLa/HeartCodec-oss-20260123'
```

## Usage

### Basic generation

```bash
cd heartlib && . .venv/bin/activate
python ./examples/run_music_generation.py \
  --model_path=./ckpt \
  --version="3B" \
  --lyrics="./assets/lyrics.txt" \
  --tags="./assets/tags.txt" \
  --save_path="./assets/output.mp3" \
  --lazy_load true
```

### Input format

**Tags** — comma-separated, no spaces:
```
piano,happy,wedding,synthesizer,romantic
rock,energetic,guitar,drums,male-vocal
```

**Lyrics** — use bracketed structural tags:
```
[Intro]

[Verse]
Your lyrics here...

[Chorus]
Chorus lyrics...

[Bridge]
Bridge lyrics...

[Outro]
```

### Key parameters

| Parameter | Default | Notes |
|-----------|---------|-------|
| `--max_audio_length_ms` | 240000 | Max 4 min |
| `--topk` | 50 | Top-k sampling |
| `--temperature` | 1.0 | Sampling temp |
| `--cfg_scale` | 1.5 | Classifier-free guidance |
| `--lazy_load` | false | Sequential load/unload (saves VRAM) |
| `--mula_dtype` | bfloat16 | bf16 recommended for HeartMuLa |
| `--codec_dtype` | float32 | fp32 required for HeartCodec quality |

**Performance:** RTF ≈ 1.0 (4-min song ≈ 4 min to generate). Output: MP3, 48kHz stereo, 128kbps.

## Pitfalls

1. **Never use bf16 for HeartCodec** — degrades audio quality; always use fp32 (default).
2. **Tags may be ignored** — known upstream issue (#90). Lyrics dominate; experiment with tag ordering.
3. **Triton not available on macOS** — GPU acceleration is Linux/CUDA only.
4. **RTX 5080 incompatibility** — reported in upstream issues; avoid or test carefully.

## Links

- Repo: https://github.com/HeartMuLa/heartlib
- Models: https://huggingface.co/HeartMuLa
- Paper: https://arxiv.org/abs/2601.10547
