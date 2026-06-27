---
type: skill
name: audiocraft-audio-generation
description: "Generate music/sounds via Meta AudioCraft (MusicGen, AudioGen)."
version: 1.1.0
author: Orchestra Research
license: MIT
dependencies: [audiocraft, torch>=2.0.0, torchaudio, transformers>=4.30.0]
platforms: [linux, macos]
tools: [execute_code, shell_exec, file_write]
metadata:
  laruche:
    tags: [Multimodal, Audio Generation, Text-to-Music, Text-to-Audio, MusicGen]

---

# AudioCraft: Audio Generation

Meta's AudioCraft library: **MusicGen** (text-to-music), **AudioGen** (text-to-sound), **EnCodec** (neural codec). Run Python snippets via `execute_code`, or write to a file and run with `shell_exec`.

## Installation

```bash
pip install audiocraft torch torchaudio
# Latest from source:
pip install git+https://github.com/facebookresearch/audiocraft.git
```

Models are cached in `~/.cache/huggingface/hub/` on first load — subsequent runs skip the download.

## Model variants

| Model | Size | VRAM (FP16) | Use case |
|-------|------|-------------|----------|
| `facebook/musicgen-small` | 300M | ~2 GB | Fast prototyping |
| `facebook/musicgen-medium` | 1.5B | ~4 GB | Balanced quality |
| `facebook/musicgen-large` | 3.3B | ~8 GB | Best quality |
| `facebook/musicgen-melody` | 1.5B | ~4 GB | Melody-conditioned |
| `facebook/musicgen-stereo-medium` | 1.5B | ~4 GB | Stereo output |
| `facebook/musicgen-style` | 1.5B | ~4 GB | Reference-style transfer |
| `facebook/audiogen-medium` | 1.5B | ~4 GB | Sound effects (16 kHz) |

## Key generation parameters

| Parameter | Default | Notes |
|-----------|---------|-------|
| `duration` | 8.0 | Seconds, max ~120 |
| `top_k` | 250 | Sampling diversity |
| `temperature` | 1.0 | Higher = more varied |
| `cfg_coef` | 3.0 | Text adherence; raise for stricter prompt following |

## Device setup (put this first in every script)

```python
import torch
device = "cuda" if torch.cuda.is_available() else "cpu"
# Optional: reduce VRAM on GPU
# model.half()  # FP16 after loading
```

## Text-to-music (MusicGen)

```python
import torchaudio
from audiocraft.models import MusicGen

model = MusicGen.get_pretrained('facebook/musicgen-medium')
model.set_generation_params(duration=30, top_k=250, temperature=1.0, cfg_coef=3.0)

descriptions = [
    "epic orchestral soundtrack with strings and brass",
    "chill lo-fi hip hop beat with jazzy piano",
]
wav = model.generate(descriptions)  # [batch, channels, samples]

for i, audio in enumerate(wav):
    torchaudio.save(f"music_{i}.wav", audio.cpu(), sample_rate=32000)
```

**Batch all prompts in a single `generate()` call — much faster than looping.**

## Melody-conditioned generation

```python
from audiocraft.models import MusicGen
import torchaudio

model = MusicGen.get_pretrained('facebook/musicgen-melody')
model.set_generation_params(duration=30)

melody, sr = torchaudio.load("melody.wav")
wav = model.generate_with_chroma(["acoustic guitar folk song"], melody, sr)
torchaudio.save("melody_out.wav", wav[0].cpu(), sample_rate=32000)
```

## Stereo generation

```python
model = MusicGen.get_pretrained('facebook/musicgen-stereo-medium')
model.set_generation_params(duration=15)
wav = model.generate(["ambient electronic music with wide stereo panning"])
# wav.shape → [1, 2, samples]
torchaudio.save("stereo.wav", wav[0].cpu(), sample_rate=32000)
```

## Style-conditioned generation (MusicGen-Style)

```python
from audiocraft.models import MusicGen
import torchaudio

model = MusicGen.get_pretrained('facebook/musicgen-style')
model.set_generation_params(duration=30, cfg_coef=3.0, cfg_coef_beta=5.0)
model.set_style_conditioner_params(eval_q=3, excerpt_length=3.0)

style_audio, sr = torchaudio.load("reference_style.wav")
wav = model.generate_with_style(["upbeat dance track"], style_audio, sr)

# Style-only (ignore text prompt):
# model.set_generation_params(duration=30, cfg_coef=3.0, cfg_coef_beta=None)
# wav = model.generate_with_style([None], style_audio, sr)
```

## Text-to-sound (AudioGen)

```python
from audiocraft.models import AudioGen
import torchaudio

model = AudioGen.get_pretrained('facebook/audiogen-medium')
model.set_generation_params(duration=5)

descriptions = [
    "thunderstorm with heavy rain",
    "busy city traffic with car horns",
    "crackling campfire in forest",
]
wav = model.generate(descriptions)

for i, audio in enumerate(wav):
    torchaudio.save(f"sound_{i}.wav", audio.cpu(), sample_rate=16000)
```

## Audio compression with EnCodec

```python
from audiocraft.models import CompressionModel
import torch, torchaudio

model = CompressionModel.get_pretrained('facebook/encodec_32khz')
wav, sr = torchaudio.load("audio.wav")

if sr != 32000:
    wav = torchaudio.transforms.Resample(sr, 32000)(wav)

with torch.no_grad():
    codes = model.encode(wav.unsqueeze(0))[0]
    decoded = model.decode(codes)

torchaudio.save("reconstructed.wav", decoded[0].cpu(), sample_rate=32000)
```

## Audio continuation (HuggingFace Transformers path)

```python
from transformers import AutoProcessor, MusicgenForConditionalGeneration
import torchaudio

processor = AutoProcessor.from_pretrained("facebook/musicgen-medium")
model = MusicgenForConditionalGeneration.from_pretrained("facebook/musicgen-medium").to("cuda")

audio, sr = torchaudio.load("intro.wav")
inputs = processor(
    audio=audio.squeeze().numpy(), sampling_rate=sr,
    text=["continue with an epic chorus"],
    padding=True, return_tensors="pt"
).to("cuda")

output = model.generate(**inputs, do_sample=True, guidance_scale=3, max_new_tokens=512)
```

## Pitfalls & fixes

| Problem | Fix |
|---------|-----|
| CUDA OOM | Use `musicgen-small`, reduce `duration`, or call `model.half()` after loading |
| Poor prompt adherence | Raise `cfg_coef` to 5–7 |
| Audio artifacts | Lower `temperature` to 0.8 |
| Stereo not working | Must use a `stereo` model variant |
| Short output | Check `duration`; for Transformers path increase `max_new_tokens` |
| Slow first run | Models download once to `~/.cache/huggingface/hub/`; subsequent runs are fast |

## Resources

- GitHub: https://github.com/facebookresearch/audiocraft
- HuggingFace: https://huggingface.co/facebook/musicgen-small
- Demo: https://huggingface.co/spaces/facebook/MusicGen
