# Local Models

LaRuche is designed local-first, and specifically for the reality of local models: they
are slower, they misformat tool calls, and they have small contexts. The engine is built
around those constraints instead of pretending they do not exist.

## Supported providers

| Provider | Notes |
|---|---|
| llama.cpp (server mode) | The reference local setup. OpenAI-compatible endpoint. |
| Ollama | Works out of the box, also used for embeddings. |
| Any OpenAI-compatible API | LM Studio, vLLM, TGI, OpenRouter, DeepSeek, etc. |
| Anthropic API | Native support for the Anthropic message format and tool calling. |

Providers are configured in **Settings > Providers**, live, no restart. You can define
several and assign a different model per channel: a big cloud model for deep missions,
a small local one for Telegram quick replies, for example.

## Tool calling with imperfect models

The engine uses native tool calling when the provider supports it, and falls back to
tolerant text parsing when the model emits tool calls as text. Every tool call is
validated client-side against the tool's JSON Schema before execution; invalid arguments
are returned to the model as a corrective message it can act on, instead of crashing the
run. See [Butinage Engine](Butinage-Engine).

In practice this means 7B-class models can drive real multi-step tool work.

## Embeddings

Semantic memory recall needs an embedding model:

```bash
ollama pull nomic-embed-text
```

Configuration:

| Variable | Default |
|---|---|
| `LARUCHE_EMBED_URL` | the local Ollama endpoint |
| `LARUCHE_EMBED_MODEL` | `nomic-embed-text` |

Without embeddings, memory recall falls back to full-text search only. Everything still
works; recall is just less clever about synonyms and phrasing.

## Context size

Set the context window per provider in Settings so the gauge (the engine's token
accountant) can budget correctly. When a run approaches the limit, the engine performs
an escale: an LLM-driven compaction that summarizes the conversation so far and
continues with the summary. Long missions survive small contexts this way.

## Practical picks

- **Small and fast (8 to 12 GB VRAM)**: a 7B/8B instruct model in llama.cpp. Good for
  chat, Telegram, watchers with `llm_check` leaves.
- **Balanced (16 to 24 GB)**: a 12B to 32B model. Comfortable for real agentic missions.
- **Hybrid**: local model as default plus a cloud key for the heavy channels. The
  secrets vault keeps the key out of the model's sight ([Secrets](Secrets)).
