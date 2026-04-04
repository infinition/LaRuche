"""LaRuche STT Service — Speech-to-Text via Whisper.

Runs as a FastAPI server, announces itself on Miel with capability:stt.
Accepts audio via POST /transcribe (file upload — WAV, WebM, MP3, etc.).
"""

import subprocess
import tempfile
import shutil
from pathlib import Path
from contextlib import asynccontextmanager

import numpy as np
import torch
import uvicorn
from fastapi import FastAPI, UploadFile, File
from fastapi.middleware.cors import CORSMiddleware

from .miel_announce import MielAnnouncer

PORT = 8421
MODEL_ID = "openai/whisper-small"
DEVICE = "cuda" if torch.cuda.is_available() else "cpu"
SAMPLE_RATE = 16000

asr_pipeline = None
announcer = None
HAS_FFMPEG = shutil.which("ffmpeg") is not None


def load_model():
    global asr_pipeline
    from transformers import pipeline as hf_pipeline

    print(f"[STT] Loading model {MODEL_ID} on {DEVICE}...")
    asr_pipeline = hf_pipeline(
        "automatic-speech-recognition",
        model=MODEL_ID,
        device=DEVICE,
        torch_dtype=torch.float16 if DEVICE == "cuda" else torch.float32,
    )
    print(f"[STT] Model loaded on {DEVICE}")
    print(f"[STT] FFmpeg available: {HAS_FFMPEG}")


def convert_to_wav(input_path: str, output_path: str) -> bool:
    """Convert any audio format to 16kHz mono WAV using ffmpeg."""
    if not HAS_FFMPEG:
        return False
    try:
        subprocess.run(
            ["ffmpeg", "-y", "-i", input_path, "-ar", str(SAMPLE_RATE),
             "-ac", "1", "-f", "wav", output_path],
            capture_output=True, timeout=30,
        )
        return Path(output_path).exists() and Path(output_path).stat().st_size > 100
    except Exception:
        return False


def load_audio(file_path: str) -> np.ndarray | None:
    """Load audio from any supported format, return float32 numpy array at 16kHz."""
    import soundfile as sf

    # Try direct load with soundfile (works for WAV, FLAC, OGG)
    try:
        audio, sr = sf.read(file_path)
        if len(audio.shape) > 1:
            audio = audio.mean(axis=1)
        if sr != SAMPLE_RATE:
            num_samples = int(len(audio) / sr * SAMPLE_RATE)
            audio = np.interp(
                np.linspace(0, len(audio) - 1, num_samples),
                np.arange(len(audio)),
                audio,
            )
        return audio.astype(np.float32)
    except Exception:
        pass

    # Fallback: convert with ffmpeg (handles WebM, MP3, Opus, etc.)
    if HAS_FFMPEG:
        wav_path = file_path + ".converted.wav"
        if convert_to_wav(file_path, wav_path):
            try:
                audio, sr = sf.read(wav_path)
                if len(audio.shape) > 1:
                    audio = audio.mean(axis=1)
                return audio.astype(np.float32)
            except Exception:
                pass
            finally:
                Path(wav_path).unlink(missing_ok=True)

    return None


@asynccontextmanager
async def lifespan(app):
    global announcer
    load_model()
    announcer = MielAnnouncer(
        node_name="laruche-stt",
        capabilities=["stt"],
        port=PORT,
        model=MODEL_ID,
    )
    announcer.register()
    yield
    if announcer:
        announcer.unregister()


app = FastAPI(title="LaRuche STT", version="0.1.0", lifespan=lifespan)
app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_methods=["*"], allow_headers=["*"])


@app.get("/health")
async def health():
    return {"status": "ok", "model": MODEL_ID, "device": DEVICE, "ffmpeg": HAS_FFMPEG}


@app.post("/transcribe")
async def transcribe(file: UploadFile = File(...)):
    """Transcribe an audio file. Accepts WAV, WebM, MP3, FLAC, OGG, Opus."""
    if asr_pipeline is None:
        return {"error": "Model not loaded"}

    content = await file.read()
    if len(content) < 100:
        return {"error": "Audio too short"}

    suffix = Path(file.filename or "audio.webm").suffix or ".webm"

    with tempfile.NamedTemporaryFile(suffix=suffix, delete=False) as tmp:
        tmp.write(content)
        tmp_path = tmp.name

    try:
        audio = load_audio(tmp_path)
        if audio is None:
            return {"error": f"Could not decode audio (format: {suffix}). Install ffmpeg for WebM support."}

        if len(audio) < 1600:  # Less than 0.1s at 16kHz
            return {"error": "Audio too short (< 0.1s)"}

        result = asr_pipeline(audio, return_timestamps=False)
        text = result["text"].strip()
        print(f"[STT] Transcribed: '{text[:80]}...' ({len(audio)/SAMPLE_RATE:.1f}s)")
        return {"text": text}
    except Exception as e:
        return {"error": str(e)}
    finally:
        Path(tmp_path).unlink(missing_ok=True)


def main():
    print(f"[STT] Starting LaRuche STT service on port {PORT}")
    uvicorn.run(app, host="0.0.0.0", port=PORT, log_level="info")


if __name__ == "__main__":
    main()
