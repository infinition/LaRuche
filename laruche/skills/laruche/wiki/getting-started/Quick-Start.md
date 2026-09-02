# Quick Start

You have opened the desktop application or started the node ([Installation](Installation)).
The same UI is available in the application window and at http://localhost:8419.

## First boot

The onboarding checklist walks you through the essentials:

1. **Model provider**: point LaRuche at your llama.cpp server, Ollama, or a cloud
   endpoint. The check probes the endpoint for real; a green checkmark means a model
   actually answered.
2. **Embeddings**: optional, enables semantic memory recall. `nomic-embed-text` via
   Ollama is the default expectation.
3. **Voice**: optional, can be set up later from Settings.

Everything in onboarding can be changed later, live, in **Settings**. Nothing requires
a restart.

## First conversation

Just talk. The agent has its tools from the first message: web research, file and git
access, shell and Python behind approval gates, memory, scheduling, computer control,
browser control and images.

Some things to try:

> "What can you do?"

> "Search the web for the latest release of X and summarize the changelog."

> "Remember that my home server is at 192.168.1.40 and runs the staging site."

> "Every morning at 9, check these three pages and tell me what changed."

> "Watch the folder `deploys/` and tell me when a file named release.zip appears."

> "Look at this screenshot and tell me which setting is wrong."

The last two create a cron and a watcher through the agent's own tools. You will see
the approval popup for anything sensitive; nothing destructive happens without you.

## The dashboard

- **Chat**: sessions persist, agent runs survive a page refresh, live steering lets you
  redirect a run in progress. Images can be attached, pasted or dropped.
- **Memory**: browse and edit the cognitive map directly, including the `system.*`
  entries that are the agent's hot-editable system prompts.
- **Automation**: crons, watchers, missions, kanban, all in one hub.
- **Capabilities**: the live tool registry, skills, plugins and MCP servers.
- **Table ronde**: a specialist debate can be opened from Chat when one answer is not
  enough.
- **Settings**: providers and per-channel models, context sizes, the curator, LaReine,
  secrets, MCP servers, channels, voice. All live.

## Where to go next

- Connect [Telegram](Telegram) so the hive reaches your phone.
- Read about the [Cognitive Memory](Cognitive-Memory) to understand what the agent
  remembers and why.
- Turn on [LaReine](LaReine) to get supervised, reviewed answers.
- Build your first real [watcher](Watchers).
