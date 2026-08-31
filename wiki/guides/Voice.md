# Voice

LaRuche can speak and listen through local or remote speech backends. Whisper provides
the optional local transcription service. Synthesis can stay local with Kokoro,
Voicebox or Voxtral, use the operating system, or call a service that exposes the
OpenAI speech format.

## What it feels like

- **Streamed speech**: the reply starts being spoken at the first complete sentence,
  while the model is still generating the rest. No wait-for-the-full-answer pause.
- **Call mode**: a full-screen call UI in the web app for hands-free back-and-forth.
- **Barge-in**: interrupt the hive mid-sentence by speaking; it stops and listens.
- **Wake word**: hands-free activation when enabled.
- **Telegram voice notes**: send one, get transcribed and answered; replies can come
  back as audio.

## Backends

| Component | Local options | Other options |
|---|---|---|
| TTS | Kokoro, Voicebox, Voxtral, pyttsx3 | Edge TTS or any OpenAI-compatible speech endpoint |
| STT | Whisper sidecar | Native transcription from a multimodal model |

Voicebox can use a cloned voice profile. Backend choice, voice, speed, external STT
preference and Telegram voice replies persist across restarts.

## Setup

1. Install and start the optional Python service from `laruche/laruche-voix` when using
   Whisper or a bundled TTS backend.
2. Open **Settings > Voice** and select the backend, voice and speed. Status indicators
   probe the configured service.
3. Test from the microphone button in chat or the call screen.

## Microphone from another device

Browsers only expose the microphone on secure origins. On `localhost` you are fine; to
talk to the hive from your phone or another machine on the LAN, enable HTTPS:

| Variable | Effect |
|---|---|
| `LARUCHE_HTTPS` | Serve with a self-signed certificate |
| `LARUCHE_TLS_CERT` / `LARUCHE_TLS_KEY` | Use your own certificate instead |
| `LARUCHE_BIND_LAN` | Required to expose the node beyond loopback at all |

Accept the self-signed certificate once on the device and the call screen works from
anywhere in the house.
