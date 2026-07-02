# LaRuche Wiki

Welcome to the hive. LaRuche is a self-hosted AI agent node written in Rust: one binary,
fully local, with a real agent engine, a cognitive memory, a built-in supervisor, and
event reflexes.

## Getting started

- [Installation](Installation) - build from source, Windows launcher, Docker
- [Quick Start](Quick-Start) - first boot, onboarding, first conversations
- [Local Models](Local-Models) - llama.cpp, Ollama, cloud providers, embeddings

## Concepts

- [Architecture](Architecture) - the workspace, the crates, how a request flows
- [Butinage Engine](Butinage-Engine) - the ReAct loop and its safety rails
- [Cognitive Memory](Cognitive-Memory) - recall, decay, supersede, hebbian ranking, git time travel
- [LaReine](LaReine) - the built-in supervisor, redo loop, proposal queue, scorecards
- [Watchers](Watchers) - event reflexes with compiled rules
- [Skills and the Curator](Skills-and-Curator) - the skill library and how it grows itself
- [Automation](Automation) - crons, missions, kanban

## Guides

- [Telegram](Telegram) - connect the bot, commands, voice messages
- [Voice](Voice) - local TTS and STT, wake word, call mode
- [Secrets](Secrets) - the vault, substitution, output masking
- [MCP](MCP) - LaRuche as an MCP client and as an MCP server

## Reference

- [Configuration](Configuration) - every environment variable and setting
- [Tools](Tools) - the built-in abeilles by category
- [Brand Glossary](Brand-Glossary) - what the French words mean
- [Security](Security) - the threat model and the defaults
- [FAQ](FAQ) - common questions

## Where things live

| Thing | Location |
|---|---|
| Web UI | http://localhost:8419 |
| Memory database | `memoire.db` (SQLite, next to the binary) |
| Memory time-travel repo | `memoire-okf/` (dedicated git repository) |
| Skills | markdown files, editable from the UI or on disk |
| Roadmap | `ROADMAP.md` in the repository |
