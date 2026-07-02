# Voice

LaRuche speaks and listens, locally. No cloud speech APIs are involved: Whisper handles
transcription and Kokoro handles synthesis, both running on your machine through small
Python sidecars managed by the node.

## What it feels like

- **Streamed speech**: the reply starts being spoken at the first complete sentence,
  while the model is still generating the rest. No wait-for-the-full-answer pause.
- **Call mode**: a full-screen call UI in the web app for hands-free back-and-forth.
- **Barge-in**: interrupt the hive mid-sentence by speaking; it stops and listens.
- **Wake word**: hands-free activation when enabled.
- **Telegram voice notes**: send one, get transcribed and answered; replies can come
  back as audio.

## Backends

| Component | Default | Alternative |
|---|---|---|
| TTS | Kokoro (fast, good quality, multilingual) | Voicebox backend with a cloned voice |
| STT | Whisper | |

The cloned-voice backend lets the Reine speak with a voice you provide. Backend choice,
voice, speed, and wake word are all in **Settings > Voice** and persist across restarts.

## Setup

1. Open **Settings > Voice** and enable the components you want. The node manages the
   Python sidecars; the status indicators probe them for real.
2. Test from the microphone button in chat or the call screen.

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
