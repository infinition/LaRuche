# FAQ

## Does it need the cloud?

No. With llama.cpp or Ollama, a local embedding model and local speech backends, the
engine, memory, automation and voice run on your machine. Telegram, cloud models,
hosted search APIs and hosted speech backends are optional network services.

## What hardware do I need?

The node itself is a lightweight Rust binary; the real cost is your model server. A
7B/8B model on 8 to 12 GB of VRAM is enough for chat, watchers, and Telegram. Bigger
models make longer agentic missions noticeably better. See
[Local Models](Local-Models).

## How is this different from other agent frameworks?

Different bets. Those projects are strong at what they do; LaRuche invests in
subsystems that matter when an agent runs 24/7 on your own hardware:

- **A Rust node with an embedded interface**, no Node.js or Python runtime for the core,
  plus desktop and terminal entry points.
- **A cognitive memory** with decay, supersede, hebbian ranking, and git time travel,
  not a chat log with embeddings.
- **A built-in supervisor** (LaReine) that judges answers against a charter, can force
  redos, and routes self-modifications through a proposal queue.
- **Watchers with compiled rules** that cost zero tokens until their deterministic
  conditions pass.
- **A two-way secrets firewall**: names in, values substituted at execution, exact
  values masked in outputs.

## Which models work best?

Anything with decent instruction following. The engine's schema validation and tolerant
parsing were built precisely so mid-size local models can do real tool work. Native
Anthropic, Codex and OpenAI-compatible tool calling are supported. Local compatible
servers include llama.cpp, LM Studio and vLLM.

## Can I read what the agent knows?

Yes, all of it. `memoire.db` is plain SQLite, the Memory tab offers full CRUD, and the
OKF export in `memoire-okf/` is markdown under git. `git log` in that folder is your
agent's learning history.

## Can I undo something it learned?

Yes. Facts are superseded rather than destroyed, the dream pass only ever proposes
cleanups, and the git time travel lets you restore any earlier state of any part of the
map: checkout the file from the snapshot you want and re-import it.

## Does the curator slow down my chats?

No. It is single-flight, has a cooldown, and yields whenever a chat or agent run is
active. Your live conversation always has priority on the model.

## Is my API key safe if I use a cloud provider?

The key lives in the encrypted vault and is substituted into requests at execution
time. The model, the context, and the session files only ever see `@@KEY_NAME` or
`[SECRET:KEY_NAME]`. See [Secrets](Secrets).

## What is ready, and what is not?

The core works and is what the rest of this wiki documents: the engine and its loop,
cognitive memory, watchers, crons and missions, native computer and browser control, the
skill library, Apps and Forged Tools, the secrets vault.

Treat these as experimental. The Miel mesh, which federates several hives, is usable but
young. The advanced tiers of LaReine's supervision, and the training exports built on its
reviews, are still moving. Backend declarations in an App manifest, for MCP stdio and
WASI, parse but supervise nothing yet.

Two limits are worth knowing before trusting a long unattended run. A tool whose outcome
became unknown, after a crash or a timeout, blocks the mutations that would follow it and
waits for a human to reconcile it; that is deliberate, but it is not automatic. And the
token budget is a reservation, not a billing guarantee.

## What extends it?

Four paths, none of which require rebuilding the engine. Skills are markdown and teach a
procedure. Forged Tools are a JSON manifest plus a command, and add one callable tool.
MCP servers bring their own tools into the registry at runtime. Apps are installable
interfaces that can also declare actions an agent may call. See
[Apps and Forged Tools](Apps-and-Forged-Tools) and [MCP](MCP).

## What is the license?

MPL-2.0. Use it, modify it, embed it; changes to MPL-licensed files must stay open.

## Why the French names?

Because it is a hive and hives deserve poetry. The [Brand Glossary](Brand-Glossary)
translates everything.
