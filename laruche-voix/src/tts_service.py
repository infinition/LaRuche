"""LaRuche TTS Service — Text-to-Speech with fallback chain.

Priority: edge-tts (neural) > kokoro > pyttsx3 (robotic).
Runs as a FastAPI server, announces itself on Miel with capability:tts.
"""

import io
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
tts_backend = "none"
announcer = None

# edge-tts voice (French neural voices)
EDGE_VOICE = "fr-FR-DeniseNeural"  # Other options: fr-FR-HenriNeural, fr-FR-EloiseNeural


def detect_backend():
    """Detect best available TTS backend."""
    global tts_backend

    # Priority 1: edge-tts (Microsoft neural voices — best quality, free)
    try:
        import edge_tts
        tts_backend = "edge-tts"
        print(f"[TTS] Using edge-tts (voice: {EDGE_VOICE})")
        return
    except ImportError:
        print("[TTS] edge-tts not installed (pip install edge-tts)")

    # Priority 2: Kokoro 82M
    try:
        from kokoro import KPipeline
        tts_backend = "kokoro"
        print("[TTS] Using Kokoro 82M")
        return
    except Exception as e:
        print(f"[TTS] Kokoro unavailable: {e}")

    # Priority 3: pyttsx3 (Windows SAPI5 — robotic but works offline)
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
    return {"status": "ok" if tts_backend != "none" else "no_engine", "backend": tts_backend, "voice": EDGE_VOICE}


class SynthesizeRequest(BaseModel):
    text: str
    voice: str = EDGE_VOICE
    speed: float = 1.0


@app.post("/synthesize")
async def synthesize(req: SynthesizeRequest):
    if tts_backend == "none":
        return {"error": "No TTS backend available"}
    if not req.text.strip():
        return {"error": "Empty text"}

    try:
        if tts_backend == "edge-tts":
            audio_bytes = await synthesize_edge(req.text, req.voice)
            media_type = "audio/mpeg"  # edge-tts outputs MP3
        elif tts_backend == "pyttsx3":
            audio_bytes = synthesize_pyttsx3(req.text)
            media_type = "audio/wav"
        else:
            return {"error": "No backend"}

        if not audio_bytes:
            return {"error": "No audio generated"}

        return StreamingResponse(
            io.BytesIO(audio_bytes),
            media_type=media_type,
            headers={"Content-Disposition": "inline; filename=speech.mp3"},
        )
    except Exception as e:
        return {"error": str(e)}


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
