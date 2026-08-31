# LaRuche Wiki

Welcome to the hive. LaRuche is a local-first AI agent with a desktop application, a
server node and a lightweight LAN client. Its main systems are written in Rust: the
agent engine, cognitive memory, automation, supervision, native computer control and
the Miel mesh.

## Current inventory

| Area | Included |
|---|---|
| Rust workspace | 17 packages |
| Default node registry | 89 tools |
| Bundled skill library | 38 skills |
| Executables | `laruche`, `laruche-node`, `laruche-cli` |
| Optional companions | Python voice service, Chrome extension, VS Code extension |
| Cargo test inventory | 741 tests |

Plugins and MCP servers can add tools at runtime. The Capabilities page in a running
node shows the effective registry after feature flags, disabled tools and extensions
have been applied.

## Getting started

- [Installation](Installation) - installers, portable builds, source and Docker
- [Desktop App](Desktop-App) - full application, LAN client and the three executables
- [Quick Start](Quick-Start) - first boot, onboarding, first conversations
- [Local Models](Local-Models) - llama.cpp, Ollama, cloud providers, embeddings

## Concepts

- [Architecture](Architecture) - the workspace, the crates, how a request flows
- [Butinage Engine](Butinage-Engine) - the ReAct loop and its safety rails
- [Cognitive Memory](Cognitive-Memory) - recall, decay, supersede, hebbian ranking, git time travel
- [LaReine](LaReine) - autonomous, hybrid and human supervision, rework, live intervention and proposal gates
- [Table Ronde](Table-Ronde) - multi-agent debate, specialist teams, tools and arbitration
- [Watchers](Watchers) - event reflexes with compiled rules
- [Skills and the Curator](Skills-and-Curator) - the skill library and how it grows itself
- [Automation](Automation) - crons, missions, kanban

## Guides

- [Telegram](Telegram) - connect the bot, commands, voice messages
- [Voice](Voice) - local TTS and STT, wake word, call mode
- [Secrets](Secrets) - the vault, substitution, output masking
- [MCP](MCP) - LaRuche as an MCP client and as an MCP server
- [Computer and Browser](Computer-and-Browser) - desktop control, Chrome, images and camera
- [Chrome Extension](Chrome-Extension) - installation, visible control, recording and security
- [Training Datasets](Training-Datasets) - SFT, DPO pairs and judge distillation from LaReine reviews

## Reference

- [Configuration](Configuration) - every environment variable and setting
- [Tools](Tools) - the complete built-in tool registry
- [Brand Glossary](Brand-Glossary) - what the French words mean
- [Security](Security) - the threat model and the defaults
- [FAQ](FAQ) - common questions

## Where things live

| Thing | Location |
|---|---|
| Web UI | http://localhost:8419 |
| Desktop application | `laruche` in a portable archive, or an installed application |
| Server | `laruche-node`, serving the UI and API on port 8419 |
| Memory database | `memoire.db` in the hive data directory |
| Memory time-travel repo | `memoire-okf/` (dedicated git repository) |
| Skills | markdown files in the hive data directory, editable from the UI or on disk |
| Roadmap | `ROADMAP.md` in the repository |
