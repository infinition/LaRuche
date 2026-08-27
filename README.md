<div align="center">

#  LaRuche

**Your personal AI hive. One Rust binary, fully local, genuinely yours.**

An agentic AI node that lives on your machine: a real agent engine, a cognitive memory
with git time travel, a built-in supervisor that reviews the agent's work, event watchers
with compiled rules, voice, Telegram, and a mesh protocol to federate several hives on
your LAN. No cloud required, no runtime to install, no subscription.

[![License](https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![Windows](https://img.shields.io/badge/Windows-0078D6?style=flat&logo=windows&logoColor=white)
![Linux](https://img.shields.io/badge/Linux-FCC624?style=flat&logo=linux&logoColor=black)
![macOS](https://img.shields.io/badge/macOS-000000?style=flat&logo=apple&logoColor=white)
[![CI](https://github.com/infinition/LaRuche/actions/workflows/ci.yml/badge.svg)](https://github.com/infinition/LaRuche/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/infinition/LaRuche?style=flat)](https://github.com/infinition/LaRuche/releases)

[Quick start](#quick-start) · [Features](#what-makes-laruche-different) · [Architecture](#architecture) · [Wiki](../../wiki) · [Roadmap](ROADMAP.md)

</div>

---

## What is LaRuche

LaRuche ("the hive") is a self-hosted AI agent node written in Rust. You run one binary,
open a web page, and you have a full agentic assistant wired to your machine: it browses
the web, runs code and shell commands behind an approval gate, remembers what matters
across sessions, schedules its own work, watches your files and services, talks to you
on Telegram or out loud, and improves its own skills over time, with you holding the keys
at every step.

It is built local-first for real: works with llama.cpp and Ollama out of the box, ships
its own web assets (no CDN, works offline), binds to loopback by default, and keeps every
byte of memory in a SQLite file you can read, export, and diff.

## What makes LaRuche different

Plenty of projects give you a chat loop with tools. LaRuche's bet is that an agent you
run 24/7 needs an engine, a memory, a supervisor, and reflexes. Each one is a first-class
subsystem here.

### 🐝 The butinage engine

A hardened ReAct loop designed for imperfect local models: native tool calling per
provider with tolerant text fallbacks, client-side JSON Schema validation of every tool
call (bad arguments come back as corrective messages the model can act on), an anti-loop
sentinel, per-tool timeouts, token budgets, cooperative cancellation, live steering
mid-run, LLM compaction when the context fills up, and parallel sub-agents (the "scouts")
for deep research fan-out. An evals harness replays fixed missions against the real
engine so regressions get caught, not guessed at.

### 🧠 Cognitive memory with git time travel

Not a vector-store bolt-on: a cognitive map (nodes and facts) over SQLite with hybrid
recall (semantic embeddings fused with full-text search), importance decay, and
write-time supersede so an updated fact replaces the stale one instead of piling up next
to it. Contradictions in the ambiguity band are settled by a small LLM arbiter, never
destructively. Hebbian ranking reinforces only the memories that were actually used in
an answer, so recalled noise stops climbing by mere co-occurrence. A periodic dream pass
surfaces duplicates as cleanup proposals you approve or reject.

And the whole map is exported as plain markdown into a dedicated git repository on a
schedule: `git log` is your agent's learning timeline, `git diff` shows exactly what it
learned between two moments, and a checkout plus re-import is a targeted rollback.

### 👑 LaReine, the built-in supervisor

A judge that reviews the agent's answers against an editable charter before you see
them, with real leverage: it can send the agent back to redo the work (a fresh agentic
run, not a rephrase), it keeps the best-scoring draft so quality never regresses across
retries, and it escalates to you when it is not confident. Its methodology score is
grounded in facts: it sees which tools were actually called, not what the answer claims.

Every self-modification the agent proposes (new skills, memory edits, deletions) can be
routed through a durable proposal queue, pull-request style: the agent proposes, you
dispose. Every verdict lands in a scorecard journal with a dashboard, so you can watch
answer quality over time.

### 👁️ Watchers with compiled rules

Event reflexes that cost nothing at runtime. Describe what you want in plain language:

> "If it is Tuesday or Thursday and my local site has been down for 10 minutes, ping me
> on Telegram every 20 minutes."

The agent compiles it once into a deterministic predicate tree: weekdays, time windows,
date ranges, file appeared/deleted/modified, service down for N minutes, back online,
content changed, log pattern, file size, HTTP status, combined with and/or/not. The only
rule that ever calls an LLM is an explicit `llm_check` leaf, and only after the
deterministic prefix has already passed. Watcher cards in the UI render the whole loop
as an editable pipeline diagram: the schema is the form.

### 🛠️ Skills, tools, and self-improvement

Around 90 built-in tools: web fetch with pagination and PDF extraction, parallel deep
search, file read/write/search, shell and Python execution behind approval gates,
scheduling (crons, long-running missions, a kanban), memory CRUD, and a full MCP client
and server, so LaRuche both consumes and exposes Model Context Protocol tools.

A background curator reviews finished conversations and proposes new verified skills.
Politely: single-flight, cooldown, and it always yields the local model to your live
chats. Skills are plain markdown files you can read, edit, and version.

### 🔐 A security model that assumes the model will mess up

Loopback bind by default (LAN exposure is an explicit opt-in), strict CORS, an auth
guard on every mutating route, sanitized chat rendering (DOMPurify, vendored), and
sensitive tools gated behind human approval with a popup.

The secrets vault gives the model names, never values: you write `@@MY_KEY` in chat,
substitution happens at execution time, and if a tool output ever echoes a secret back,
the exact value is masked to `[SECRET:MY_KEY]` before it can reach the context or the
session file.

### 📡 The Miel mesh

Several hives on one network discover each other over mDNS, exchange messages, announce
capabilities (llm, vision, audio, rag), and can federate skills and memory facts with
provenance tags. Opt-in and signed. Your desktop hive and your homelab hive become one
swarm.

### 🖥️ Interfaces everywhere

A fast web SPA (installable PWA, offline shell, bilingual FR/EN), a terminal TUI with
five views and a slash-command palette, native Telegram (Discord and Slack wired), and
a local voice mode: streamed TTS that starts speaking at the first sentence, wake word,
barge-in interruption, and a full-screen call UI. Whisper STT and Kokoro TTS run locally
through small Python sidecars, with a cloned-voice backend option.

## Quick start

### Prerequisites

- [Rust](https://rustup.rs/) (stable) to build from source
- A local model server: [llama.cpp](https://github.com/ggerganov/llama.cpp) in server
  mode or [Ollama](https://ollama.com), or any OpenAI-compatible or Anthropic API
  endpoint (cloud keys work too, they are just not required)
- Optional but recommended: `ollama pull nomic-embed-text` for semantic memory recall

### Run

```bash
git clone https://github.com/infinition/LaRuche
cd LaRuche/laruche
cargo run --release -p laruche-node
```

Then open **http://localhost:8419**.

On Windows, `lancer_butinage.bat` at the repo root builds, starts the node, and opens
the browser once the server answers. Docker users: `docker compose up` inside `laruche/`.

First boot walks you through an onboarding checklist (model, embeddings, voice), each
step probed for real, no fake green checkmarks. Everything else is configured live in
Settings with no restart: providers and per-channel models, context sizes, the curator,
LaReine, secrets, MCP servers, channels.

### Talk to it

Ask it things. Then ask it to do things:

> "Watch `deploys/release.log` and ping me on Telegram when a line contains ERROR, but
> not at night."

> "Every morning at 9, check these three pages and tell me what changed."

> "Start a mission: research topic X in depth, iterate daily, keep your notes in memory."

It creates the watcher, the cron, or the mission itself, through its own tools, with
your approval.

## Configuration

Everything lives in Settings at runtime. The launcher-level knobs:

| Variable | Default | Purpose |
|---|---|---|
| `LARUCHE_PORT` | `8419` | Web UI and API port |
| `LARUCHE_MEMOIRE_BACKEND` | `sqlite` | Memory backend (`sqlite`, `native`, `sidecar`) |
| `LARUCHE_BIND_LAN` | off | Expose on the LAN (loopback only by default) |
| `LARUCHE_EMBED_URL` / `LARUCHE_EMBED_MODEL` | Ollama / `nomic-embed-text` | Embeddings for semantic recall |
| `LARUCHE_TAVILY_KEY` / `LARUCHE_BRAVE_KEY` / `LARUCHE_SEARXNG_URL` | none | Web search providers (free scrapers otherwise) |
| `LARUCHE_OKF_GIT_SECS` | `1800` | Memory time-travel snapshot interval, `0` disables |
| `LARUCHE_HTTPS` | off | Self-signed TLS (needed for the microphone from other devices) |

The full reference lives in the [wiki](../../wiki).

## Architecture

A Rust workspace, one process, no external runtime:

| Crate | Role |
|---|---|
| `laruche-node` | axum server: API, web UI, channel bots, MCP server, background jobs |
| `laruche-essaim` | Agent layer: tools ("abeilles"), providers, prompts, curator, LaReine |
| `laruche-butinage` | The pure, tested ReAct core: engine loop, compass, gauge, sentinel |
| `laruche-memoire` | Cognitive map: SQLite + FTS5 + embeddings, hebbian ranking, OKF export |
| `laruche-watchers` | Event watchers and the compiled rules DSL |
| `miel-protocol` | Mesh: mDNS discovery, manifests, federation |
| `laruche-dashboard` | Web SPA (vanilla JS, compiled into the binary) |
| `laruche-evals` | Evals harness: fixed missions replayed against the real engine |
| `laruche-cli`, `laruche-skills`, `laruche-kanban`, `laruche-permissions`, `laruche-voix` | TUI, skill store, kanban, permission engine, voice |

### The hive speaks French

The brand vocabulary is French and stays that way, it is the identity: **butinage** is
the agentic loop (bees foraging), the **essaim** is the swarm, an **abeille** is a tool,
the **éclaireuses** are the scout sub-agents, the **escale** is compaction, the
**curateur** grows the skill library, **LaReine** supervises, and **Miel** is what the
mesh trades. The code and the docs are English.

## Project status

Beta, moving fast, used daily by its author. 340+ tests across the workspace run green
on every change. The [ROADMAP](ROADMAP.md) is the single source of truth for what is
done and what is next.

## Contributing

Issues and PRs are welcome. Read the [wiki](../../wiki) for architecture notes, run
`cargo test --workspace` before submitting, and keep the brand vocabulary French and
the code English. Security reports: please open a private advisory rather than a public
issue.

## License

[MPL-2.0](LICENSE)
