---
type: skill
name: configure-laruche
description: Set up or change how this LaRuche is configured - LLM provider, Telegram/Discord/Slack, secrets vault, accounts, MCP server, memory, LaReine.
tools: [shell_exec, memory_read_node, memory_write, skill_view]
---

# Configuring this LaRuche

You are helping someone set up the node you are running on. Two rules decide whether you
are useful or dangerous here.

**Read the live state before saying anything.** This file describes WHERE things live and
WHAT the traps are. It never states what is currently configured, because that changes and
this file does not. Every answer starts with a `curl`.

**You cannot change the configuration yourself.** Reads are open on loopback; writes need
the user's browser session. So your job is: find out the real state, then tell the user the
exact clicks. Never claim you have configured something.

## Reaching the local API

The node serves on `LARUCHE_PORT`, default **8419**. Confirm before using it:

```bash
curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:8419/api/status
```

`200` means that is the port. Anything else: ask the user which port their node uses, do
not scan.

GET is unauthenticated on loopback. POST/PUT/DELETE are not, and your `curl` will get
`{"error":"authentication required"}`. That is expected, not a bug to work around.

## The one command to start with

```bash
curl -s http://127.0.0.1:8419/api/onboarding
```

Returns each setup step with `done`, `optional` and an `instruction` written for a human.
Steps marked `optional: true` are not failures: LaRuche runs without them. Say so, instead
of listing them as problems.

For a fuller picture of what is reachable right now:

```bash
curl -s http://127.0.0.1:8419/api/doctor
```

Every check carries a `detail` explaining its verdict. Read it. `"OpenAI-compatible"` is
the FALLBACK label for a provider type that is not recognised, so `error` there usually
means the address is wrong, not that a service is down.

## Where each thing is configured

All paths below are in the web interface, under **Settings** unless said otherwise.

| What | Where | Read it with |
|---|---|---|
| LLM provider, API key, models | Models & Providers | `/api/config/provider`, `/api/profiles` |
| What shows in the chat thread | Chat | `/api/config/runtime` |
| Generation parameters, context | Generation | `/api/config/runtime`, `/api/config/compaction` |
| Telegram, Discord, Slack | Channels | `/api/config/channels`, `/api/config/notify` |
| API keys and tokens vault | Secrets & Webhooks | `/api/secrets` (names only, never values) |
| LaReine review and dataset | LaReine | `/api/config/reine` |
| MCP server, IP firewall, token | Capabilities | `/api/config/curateur` |
| Voice STT/TTS | Voice | `/api/config/voice` |
| Accounts, roles, passwords | Admin | `/api/admin/users` |
| Enabled/disabled tools | Capabilities | `/api/tools` |

## The vault, and why it matters

Any API key or channel token can be stored as a reference instead of a literal:

```
${OPENAI_KEY}
```

The node expands it at call time. Recommend this over pasting a raw key, for one concrete
reason: the literal is written into `provider-profiles.json` or `channels-config.json` on
disk, the reference is not.

To set one up: **Secrets & Webhooks** → create the entry → then in Providers or Channels,
switch the field from "Type the key" to "Pick from the vault".

An unknown reference is left as-is rather than blanked, so a typo produces an
authentication error that still shows you the wrong name. That is deliberate.

## Traps, each of which cost someone a debugging session

**A cadence must be a cron expression.** `mission_create` and `cron_create` reject anything
else, but be explicit with the user: "every evening at 20:00" is not a schedule,
`0 20 * * *` is. Five fields: minute hour day month weekday.

**Disabling a tool disables it everywhere.** Settings > Capabilities cuts it for you, for
the agent, and for MCP clients. There is no per-surface toggle.

**The MCP server is off by default and exposes every tool, `shell_exec` included.** With no
token configured it trusts any caller on this machine. If the user turns it on, mention the
token option in the same breath.

**Embeddings are optional, and their absence is not an error.** Without them LaRuche still
works and still writes to memory, but recall becomes keyword-only: it cannot find a note
worded differently from the question. Say that, rather than "RAG is broken".

**Two accounts do not mean two memories.** Chat sessions are per-user; the cognitive memory,
the secrets vault, crons and missions are shared by everyone on the node. Do not promise
isolation that does not exist.

## Answering a configuration question

1. Run the `curl` that covers it, from the table above.
2. State what IS configured, quoting the field you read.
3. Give the click path, in the order the user will need it.
4. Name the one trap that applies, if any. Not all of them.

If a read fails or returns something you do not recognise, say so and stop. A confident
wrong answer about configuration costs more than an admission: the user follows it, it does
not work, and they cannot tell whether the fault is theirs or the product's.

## What you must not do

Do not edit `laruche-reine.json`, `channels-config.json`, `provider-profiles.json` or any
other config file with `file_write`. They are read into memory at boot and rewritten by the
API; an edit on disk is either ignored or silently overwritten. Configuration changes go
through the interface, by the user.

Do not read `secrets.enc`, `secret.key` or `mesh-secret.json`. `/api/secrets` returns entry
NAMES, which is all you ever need to help someone.
