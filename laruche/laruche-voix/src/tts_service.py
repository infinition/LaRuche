"""LaRuche TTS Service: Text-to-Speech with fallback chain.

Priority: edge-tts (neural) > kokoro > pyttsx3 (robotic).
Runs as a FastAPI server, announces itself on Miel with capability:tts.
"""

import io
import re
import asyncio
import tempfile
import os
from contextlib import asynccontextmanager

import uvicorn
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import StreamingResponse
from pydantic import BaseModel

from .miel_announce import MielAnnouncer

PORT = 8422
MAX_TTS_CHARS = 8000  # reject larger synthesis requests (resource-exhaustion guard)
tts_backend = "none"
announcer = None

# edge-tts voice (French neural voices)
EDGE_VOICE = "fr-FR-DeniseNeural"  # Other options: fr-FR-HenriNeural, fr-FR-EloiseNeural

# Kokoro via kokoro-onnx (local neural TTS, 24kHz, free/offline, fast on CPU).
# Matches the validated bench (kokoro-v1.0.onnx + voices-v1.0.bin, voice ff_siwis).
# Point KOKORO_MODEL / KOKORO_VOICES at the downloaded files (or place them in cwd).
KOKORO_VOICE = os.environ.get("KOKORO_VOICE", "ff_siwis")  # French female
KOKORO_LANG = os.environ.get("KOKORO_LANG", "fr-fr")
KOKORO_MODEL = os.environ.get("KOKORO_MODEL", "kokoro-v1.0.onnx")
KOKORO_VOICES = os.environ.get("KOKORO_VOICES", "voices-v1.0.bin")
_kokoro = None  # cached Kokoro instance (loading the model is slow)

# Voicebox (voicebox.sh): a local-first voice studio exposing a REST API. Clone your own
# voice there, then point LaRuche at it to speak in that voice. Set TTS_BACKEND=voicebox
# and VOICEBOX_PROFILE to your cloned profile name (or pass it as the request voice).
VOICEBOX_URL = os.environ.get("VOICEBOX_URL", "http://127.0.0.1:17493").rstrip("/")
VOICEBOX_PROFILE = os.environ.get("VOICEBOX_PROFILE", "")  # cloned voice profile name
VOICEBOX_LANG = os.environ.get("VOICEBOX_LANG", "fr")


def detect_backend():
    """Detect best available TTS backend.

    Set TTS_BACKEND=kokoro|edge-tts|pyttsx3 to force one (otherwise the priority
    chain below picks the best available). Kokoro is local and free; force it to give
    LaReine her offline voice even when edge-tts is installed.
    """
    global tts_backend

    forced = os.environ.get("TTS_BACKEND", "").strip().lower()
    if forced in ("edge-tts", "kokoro", "pyttsx3", "voicebox"):
        tts_backend = forced
        print(f"[TTS] Forced backend via TTS_BACKEND: {forced}")
        return

    # Priority 1: edge-tts (Microsoft neural voices, best quality, free)
    try:
        import edge_tts
        tts_backend = "edge-tts"
        print(f"[TTS] Using edge-tts (voice: {EDGE_VOICE})")
        return
    except ImportError:
        print("[TTS] edge-tts not installed (pip install edge-tts)")

    # Priority 2: Kokoro 82M via kokoro-onnx (local, free, fast on CPU).
    try:
        import kokoro_onnx  # noqa: F401
        if os.path.exists(KOKORO_MODEL) and os.path.exists(KOKORO_VOICES):
            tts_backend = "kokoro"
            print(f"[TTS] Using Kokoro ({KOKORO_MODEL}, voice {KOKORO_VOICE})")
            return
        print(f"[TTS] Kokoro model files missing ({KOKORO_MODEL} / {KOKORO_VOICES})")
    except Exception as e:
        print(f"[TTS] Kokoro unavailable: {e} (pip install kokoro-onnx)")

    # Priority 3: pyttsx3 (Windows SAPI5, robotic but works offline)
    try:
        import pyttsx3
        tts_backend = "pyttsx3"
        print("[TTS] Using pyttsx3 (offline, robotic)")
        return
    except Exception as e:
        print(f"[TTS] pyttsx3 unavailable: {e}")

    print("[TTS] WARNING: No TTS backend available!")


async def synthesize_edge(text: str, voice: str = EDGE_VOICE) -> bytes:
    """Synthesize with edge-tts (Microsoft neural voices)."""
    import edge_tts

    communicate = edge_tts.Communicate(text, voice)
    buf = io.BytesIO()
    async for chunk in communicate.stream():
        if chunk["type"] == "audio":
            buf.write(chunk["data"])
    buf.seek(0)
    return buf.read()


def synthesize_pyttsx3(text: str) -> bytes:
    """Synthesize with pyttsx3 (Windows native)."""
    import pyttsx3
    import threading

    lock = threading.Lock()
    with lock:
        engine = pyttsx3.init()
        voices = engine.getProperty("voices")
        for v in voices:
            if "french" in v.name.lower() or "fr" in v.id.lower():
                engine.setProperty("voice", v.id)
                break
        engine.setProperty("rate", 175)

        tmp = tempfile.NamedTemporaryFile(suffix=".wav", delete=False)
        tmp_path = tmp.name
        tmp.close()
        try:
            engine.save_to_file(text, tmp_path)
            engine.runAndWait()
            with open(tmp_path, "rb") as f:
                return f.read()
        finally:
            try:
                os.unlink(tmp_path)
            except Exception:
                pass


def _phrases(t: str):
    """Split text into sentences so synthesis streams phrase by phrase (low latency)."""
    import re
    parts = re.split(r"(?<=[.!?…:;])\s+", t.strip())
    return [p for p in parts if p.strip()] or [t.strip()]


def synthesize_kokoro(text: str, speed: float = 1.0, voice: str = None) -> bytes:
    """Synthesize with Kokoro via kokoro-onnx -> mono WAV bytes.

    Mirrors the validated bench: `Kokoro(model, voices).create(text, voice, speed, lang)`
    returns (float32 samples, sample_rate). We synthesize sentence by sentence and
    concatenate, then wrap as PCM16 WAV (wave is stdlib; numpy ships with kokoro-onnx).
    """
    import wave
    import numpy as np
    from kokoro_onnx import Kokoro

    global _kokoro
    if _kokoro is None:
        if not os.path.exists(KOKORO_MODEL) or not os.path.exists(KOKORO_VOICES):
            raise FileNotFoundError(
                f"Kokoro model files not found: '{KOKORO_MODEL}' / '{KOKORO_VOICES}'. "
                f"Set KOKORO_MODEL and KOKORO_VOICES to the full paths (e.g. your kokoro-test "
                f"folder), or place kokoro-v1.0.onnx + voices-v1.0.bin in the working directory."
            )
        _kokoro = Kokoro(KOKORO_MODEL, KOKORO_VOICES)

    kvoice = voice or KOKORO_VOICE
    chunks = []
    sr = 24000
    for phrase in _phrases(text):
        samples, sr = _kokoro.create(phrase, voice=kvoice, speed=speed, lang=KOKORO_LANG)
        chunks.append(np.asarray(samples, dtype=np.float32))
    if not chunks:
        return b""

    samples = np.concatenate(chunks)
    pcm16 = (np.clip(samples, -1.0, 1.0) * 32767.0).astype(np.int16)
    buf = io.BytesIO()
    with wave.open(buf, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(sr)
        w.writeframes(pcm16.tobytes())
    buf.seek(0)
    return buf.read()


@asynccontextmanager
async def lifespan(app):
    global announcer
    detect_backend()
    announcer = MielAnnouncer(
        node_name="laruche-tts",
        capabilities=["tts"],
        port=PORT,
        model=f"tts-{tts_backend}",
    )
    announcer.register()
    yield
    if announcer:
        announcer.unregister()


app = FastAPI(title="LaRuche TTS", version="0.2.0", lifespan=lifespan)
app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_methods=["*"], allow_headers=["*"])


@app.get("/health")
async def health():
    voice = {
        "kokoro": KOKORO_VOICE,
        "edge-tts": EDGE_VOICE,
        "voicebox": VOICEBOX_PROFILE or "(default)",
    }.get(tts_backend, EDGE_VOICE)
    return {
        "status": "ok" if tts_backend != "none" else "no_engine",
        "backend": tts_backend,
        "voice": voice,
    }


def synthesize_voicebox(text: str, voice: str = None) -> tuple:
    """Synthesize via a local Voicebox (voicebox.sh) REST server, using a cloned voice.

    Voicebox exposes POST /speak {text, profile, language}. `profile` matches a cloned
    voice by name (case-insensitive). Returns (audio_bytes, media_type).
    """
    import urllib.request
    import json as _json
    import base64

    profile = voice or VOICEBOX_PROFILE
    payload = {"text": text, "language": VOICEBOX_LANG}
    if profile:
        payload["profile"] = profile
    data = _json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        f"{VOICEBOX_URL}/speak",
        data=data,
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=60) as resp:
        ctype = (resp.headers.get("Content-Type") or "").lower()
        body = resp.read()
    # Most likely: raw audio bytes. Fall back to common JSON shapes.
    if "application/json" in ctype:
        obj = _json.loads(body.decode("utf-8"))
        if isinstance(obj, dict):
            if obj.get("audio"):
                return base64.b64decode(obj["audio"]), "audio/wav"
            if obj.get("path") and os.path.exists(obj["path"]):
                with open(obj["path"], "rb") as f:
                    return f.read(), "audio/wav"
        raise RuntimeError(f"Voicebox returned JSON without audio: {str(obj)[:200]}")
    media = "audio/mpeg" if ("mpeg" in ctype or "mp3" in ctype) else "audio/wav"
    return body, media


class SynthesizeRequest(BaseModel):
    text: str
    voice: str = EDGE_VOICE
    speed: float = 1.0
    format: str = "wav"  # "wav"/"mp3" (native) or "ogg" (opus, for voice notes)
    backend: str = ""  # override the active backend for this request (kokoro/voicebox/...)


_EMOJI_RE = re.compile(
    "[\U0001F000-\U0001FFFF\U00002600-\U000027BF\U00002B00-\U00002BFF"
    "\U00002190-\U000021FF\U00002300-\U000023FF\U0000FE00-\U0000FE0F\U0000200D\U000020E3]",
    flags=re.UNICODE,
)


def clean_for_speech(text: str) -> str:
    """Strip emoji and collapse whitespace so they are not pronounced."""
    return re.sub(r"\s{2,}", " ", _EMOJI_RE.sub("", text)).strip()


def to_ogg_opus(audio_bytes: bytes) -> bytes | None:
    """Transcode audio (wav/mp3) to OGG/Opus via ffmpeg, for Telegram voice notes.
    Returns None when ffmpeg is unavailable or fails."""
    import shutil
    import subprocess

    if not shutil.which("ffmpeg"):
        return None
    try:
        p = subprocess.run(
            ["ffmpeg", "-y", "-i", "pipe:0", "-c:a", "libopus", "-b:a", "32k", "-f", "ogg", "pipe:1"],
            input=audio_bytes,
            capture_output=True,
        )
        return p.stdout if p.returncode == 0 and p.stdout else None
    except Exception:
        return None


@app.post("/synthesize")
async def synthesize(req: SynthesizeRequest):
    # A request may override the active backend (Settings TTS-backend selector), as long as
    # it is one we know; otherwise fall back to the service default.
    active = req.backend.strip().lower()
    if active not in ("edge-tts", "kokoro", "pyttsx3", "voicebox"):
        active = tts_backend
    from fastapi.responses import JSONResponse

    if active == "none":
        return JSONResponse(status_code=503, content={"error": "No TTS backend available"})
    # Cap the input so a huge payload cannot exhaust CPU/memory on this open endpoint.
    if len(req.text) > MAX_TTS_CHARS:
        return JSONResponse(
            status_code=413,
            content={"error": f"text too long (max {MAX_TTS_CHARS} chars)"},
        )
    text = clean_for_speech(req.text)
    if not text.strip():
        return JSONResponse(status_code=400, content={"error": "Empty text"})
    req.text = text

    try:
        if active == "edge-tts":
            audio_bytes = await synthesize_edge(req.text, req.voice)
            media_type = "audio/mpeg"  # edge-tts outputs MP3
        elif active == "kokoro":
            # Run the (blocking) neural synthesis off the event loop. A caller-supplied
            # Kokoro voice (e.g. "ff_siwis") overrides the default; the edge default name
            # is treated as "unset" so the configured KOKORO_VOICE is used.
            kvoice = req.voice if (req.voice and req.voice != EDGE_VOICE) else None
            audio_bytes = await asyncio.to_thread(synthesize_kokoro, req.text, req.speed, kvoice)
            media_type = "audio/wav"
        elif active == "pyttsx3":
            audio_bytes = synthesize_pyttsx3(req.text)
            media_type = "audio/wav"
        elif active == "voicebox":
            # A caller-supplied profile name overrides the default; the edge default name
            # is treated as "unset" so VOICEBOX_PROFILE is used.
            vprofile = req.voice if (req.voice and req.voice != EDGE_VOICE) else None
            audio_bytes, media_type = await asyncio.to_thread(
                synthesize_voicebox, req.text, vprofile
            )
        else:
            return JSONResponse(status_code=503, content={"error": "No backend"})

        if not audio_bytes:
            return JSONResponse(status_code=500, content={"error": "No audio generated"})

        filename = "speech"
        if req.format.lower() == "ogg":
            ogg = await asyncio.to_thread(to_ogg_opus, audio_bytes)
            if ogg:
                audio_bytes, media_type, filename = ogg, "audio/ogg", "voice.ogg"
            # If ffmpeg is missing, fall through with the native format.

        return StreamingResponse(
            io.BytesIO(audio_bytes),
            media_type=media_type,
            headers={"Content-Disposition": f"inline; filename={filename}"},
        )
    except Exception as e:
        # Surface the real cause (a 200 + JSON would make the browser try to play JSON
        # as audio -> NotSupportedError). Log it and return a proper error status.
        import traceback
        print(f"[TTS] synthesize failed ({tts_backend}): {e}")
        traceback.print_exc()
        from fastapi.responses import JSONResponse
        return JSONResponse(status_code=500, content={"error": str(e), "backend": tts_backend})


@app.get("/voices")
async def list_voices():
    """List available voices (edge-tts only)."""
    if tts_backend != "edge-tts":
        return {"voices": [], "backend": tts_backend}
    try:
        import edge_tts
        voices = await edge_tts.list_voices()
        # Filter French voices
        fr_voices = [v for v in voices if v["Locale"].startswith("fr-")]
        return {
            "voices": [{"id": v["ShortName"], "name": v["FriendlyName"], "gender": v["Gender"]} for v in fr_voices],
            "current": EDGE_VOICE,
            "backend": tts_backend,
        }
    except Exception as e:
        return {"error": str(e)}


def main():
    print(f"[TTS] Starting LaRuche TTS service on port {PORT}")
    uvicorn.run(app, host="0.0.0.0", port=PORT, log_level="info")


if __name__ == "__main__":
    main()
