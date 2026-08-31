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

## What is the license?

MPL-2.0. Use it, modify it, embed it; changes to MPL-licensed files must stay open.

## Why the French names?

Because it is a hive and hives deserve poetry. The [Brand Glossary](Brand-Glossary)
translates everything.
