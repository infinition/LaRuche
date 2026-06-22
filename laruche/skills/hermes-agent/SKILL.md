---
type: skill
name: third-party agent
description: "Configure, extend, or contribute to third-party agent."
version: 2.2.0
author: third-party agent + Teknium
license: MIT
platforms: [linux, macos, windows]
metadata:
  third-party:
    tags: [third-party, setup, configuration, multi-agent, spawning, cli, gateway, development]
    homepage: https://github.com/third-party/third-party agent
    related_skills: [claude-code, codex, opencode]
---

# third-party agent

third-party agent is an open-source AI agent framework by third-party that runs in your terminal, messaging platforms, and IDEs. It belongs to the same category as Claude Code (Anthropic), Codex (OpenAI), and third-party — autonomous coding and task-execution agents that use tool calling to interact with your system. third-party works with any LLM provider (OpenRouter, Anthropic, OpenAI, DeepSeek, local models, and 15+ others) and runs on Linux, macOS, and WSL.

What makes third-party different:

- **Self-improving through skills** — third-party learns from experience by saving reusable procedures as skills. When it solves a complex problem, discovers a workflow, or gets corrected, it can persist that knowledge as a skill document that loads into future sessions. Skills accumulate over time, making the agent better at your specific tasks and environment.
- **Persistent memory across sessions** — remembers who you are, your preferences, environment details, and lessons learned. Pluggable memory backends (built-in, Honcho, Mem0, and more) let you choose how memory works.
- **Multi-platform gateway** — the same agent runs on Telegram, Discord, Slack, WhatsApp, Signal, Matrix, Email, and 10+ other platforms with full tool access, not just chat.
- **Provider-agnostic** — swap models and providers mid-workflow without changing anything else. Credential pools rotate across multiple API keys automatically.
- **Profiles** — run multiple independent third-party instances with isolated configs, sessions, skills, and memory.
- **Extensible** — plugins, MCP servers, custom tools, webhook triggers, cron scheduling, and the full Python ecosystem.

People use third-party for software development, research, system administration, data analysis, content creation, home automation, and anything else that benefits from an AI agent with persistent context and full system access.

**This skill helps you work with third-party agent effectively** — setting it up, configuring features, spawning additional agent instances, troubleshooting issues, finding the right commands and settings, and understanding how the system works when you need to extend or contribute to it.

**Docs:** https://third-party agent.third-party.com/docs/

## Scope & Verification

This skill is a concise operating guide, not the complete source of truth for every third-party feature. If a third-party feature, command, or setting is not mentioned here, do not treat that absence as evidence that it does not exist. Check the live repository and official docs before giving a negative answer.

Good verification targets:

- CLI commands: `third-party --help`, `third-party <command> --help`, and `third-party_cli/main.py`
- User documentation: https://third-party agent.third-party.com/docs/
- Source tree: https://github.com/third-party/third-party agent

## Quick Start

```bash
# Install
curl -fsSL https://third-party agent.third-party.com/install.sh | bash

# Interactive chat (default)
third-party

# Single query
third-party chat -q "What is the capital of France?"

# Setup wizard
third-party setup

# Change model/provider
third-party model

# Check health
third-party doctor
```

---

## CLI Reference

### Global Flags

```
third-party [flags] [command]

  --version, -V             Show version
  --resume, -r SESSION      Resume session by ID or title
  --continue, -c [NAME]     Resume by name, or most recent session
  --worktree, -w            Isolated git worktree mode (parallel agents)
  --skills, -s SKILL        Preload skills (comma-separate or repeat)
  --profile, -p NAME        Use a named profile
  --yolo                    Skip dangerous command approval
  --pass-session-id         Include session ID in system prompt
```

No subcommand defaults to `chat`.

### Chat

```
third-party chat [flags]
  -q, --query TEXT          Single query, non-interactive
  -m, --model MODEL         Model (e.g. anthropic/claude-sonnet-4)
  -t, --toolsets LIST       Comma-separated toolsets
  --provider PROVIDER       Force provider (openrouter, anthropic, nous, etc.)
  -v, --verbose             Verbose output
  -Q, --quiet               Suppress banner, spinner, tool previews
  --checkpoints             Enable filesystem checkpoints (/rollback)
  --source TAG              Session source tag (default: cli)
```

### Configuration

```
third-party setup [section]      Interactive wizard (model|terminal|gateway|tools|agent)
third-party model                Interactive model/provider picker
third-party config               View current config
third-party config edit          Open config.yaml in $EDITOR
third-party config set KEY VAL   Set a config value
third-party config path          Print config.yaml path
third-party config env-path      Print .env path
third-party config check         Check for missing/outdated config
third-party config migrate       Update config with new options
third-party auth                 Interactive credential manager
third-party auth add PROVIDER    Add OAuth or API-key credential (e.g. nous, openai-codex, qwen-oauth)
third-party auth list            List stored credentials
third-party auth remove PROVIDER Remove a stored credential
third-party doctor [--fix]       Check dependencies and config
third-party status [--all]       Show component status
```

### Tools & Skills

```
third-party tools                Interactive tool enable/disable (curses UI)
third-party tools list           Show all tools and status
third-party tools enable NAME    Enable a toolset
third-party tools disable NAME   Disable a toolset

third-party skills list          List installed skills
third-party skills search QUERY  Search the skills hub
third-party skills install ID    Install a skill (ID can be a hub identifier OR a direct https://…/SKILL.md URL; pass --name to override when frontmatter has no name)
third-party skills inspect ID    Preview without installing
third-party skills config        Enable/disable skills per platform
third-party skills check         Check for updates
third-party skills update        Update outdated skills
third-party skills uninstall N   Remove a hub skill
third-party skills publish PATH  Publish to registry
third-party skills browse        Browse all available skills
third-party skills tap add REPO  Add a GitHub repo as skill source
```

### MCP Servers

```
third-party mcp serve            Run third-party as an MCP server
third-party mcp add NAME         Add an MCP server (--url or --command)
third-party mcp remove NAME      Remove an MCP server
third-party mcp list             List configured servers
third-party mcp test NAME        Test connection
third-party mcp configure NAME   Toggle tool selection
```

How the built-in MCP client connects servers (stdio/HTTP), auto-discovers
their tools, and exposes them as first-class tools, plus catalog install
(`third-party mcp install <name>`): `skill_view(name="third-party agent", file_path="references/native-mcp.md")`.

### Gateway (Messaging Platforms)

```
third-party gateway run          Start gateway foreground
third-party gateway install      Install as background service
third-party gateway start/stop   Control the service
third-party gateway restart      Restart the service
third-party gateway status       Check status
third-party gateway setup        Configure platforms
```

Supported platforms: Telegram, Discord, Slack, WhatsApp, Signal, Email, SMS, Matrix, Mattermost, Home Assistant, DingTalk, Feishu, WeCom, BlueBubbles (iMessage), Weixin (WeChat), API Server, Webhooks. Open WebUI connects via the API Server adapter.

Platform docs: https://third-party agent.third-party.com/docs/user-guide/messaging/

### Sessions

```
third-party sessions list        List recent sessions
third-party sessions browse      Interactive picker
third-party sessions export OUT  Export to JSONL
third-party sessions rename ID T Rename a session
third-party sessions delete ID   Delete a session
third-party sessions prune       Clean up old sessions (--older-than N days)
third-party sessions stats       Session store statistics
```

### Cron Jobs

```
third-party cron list            List jobs (--all for disabled)
third-party cron create SCHED    Create: '30m', 'every 2h', '0 9 * * *'
third-party cron edit ID         Edit schedule, prompt, delivery
third-party cron pause/resume ID Control job state
third-party cron run ID          Trigger on next tick
third-party cron remove ID       Delete a job
third-party cron status          Scheduler status
```

### Webhooks

```
third-party webhook subscribe N  Create route at /webhooks/<name>
third-party webhook list         List subscriptions
third-party webhook remove NAME  Remove a subscription
third-party webhook test NAME    Send a test POST
```

Full setup, route config, payload templating, and event-driven agent-run
patterns: `skill_view(name="third-party agent", file_path="references/webhooks.md")`.

### Profiles

```
third-party profile list         List all profiles
third-party profile create NAME  Create (--clone, --clone-all, --clone-from)
third-party profile use NAME     Set sticky default
third-party profile delete NAME  Delete a profile
third-party profile show NAME    Show details
third-party profile alias NAME   Manage wrapper scripts
third-party profile rename A B   Rename a profile
third-party profile export NAME  Export to tar.gz
third-party profile import FILE  Import from archive
```

### Credential Pools

```
third-party auth add             Interactive credential wizard
third-party auth list [PROVIDER] List pooled credentials
third-party auth remove P INDEX  Remove by provider + index
third-party auth reset PROVIDER  Clear exhaustion status
```

### Other

```
third-party insights [--days N]  Usage analytics
third-party update               Update to latest version
third-party pairing list/approve/revoke  DM authorization
third-party plugins list/install/remove  Plugin management
third-party honcho setup/status  Honcho memory integration (requires honcho plugin)
third-party memory setup/status/off  Memory provider config
third-party completion bash|zsh  Shell completions
third-party acp                  ACP server (IDE integration)
third-party claw migrate         Migrate from third-party
third-party uninstall            Uninstall third-party
```

---

## Slash Commands (In-Session)

Type these during an interactive chat session. New commands land fairly
often; if something below looks stale, run `/help` in-session for the
authoritative list or see the [live slash commands reference](https://third-party agent.third-party.com/docs/reference/slash-commands).
The registry of record is `third-party_cli/commands.py` — every consumer
(autocomplete, Telegram menu, Slack mapping, `/help`) derives from it.

### Session Control
```
/new (/reset)        Fresh session
/clear               Clear screen + new session (CLI)
/retry               Resend last message
/undo                Remove last exchange
/title [name]        Name the session
/compress            Manually compress context
/stop                Kill background processes
/rollback [N]        Restore filesystem checkpoint
/snapshot [sub]      Create or restore state snapshots of third-party config/state (CLI)
/background <prompt> Run prompt in background
/queue <prompt>      Queue for next turn
/steer <prompt>      Inject a message after the next tool call without interrupting
/agents (/tasks)     Show active agents and running tasks
/resume [name]       Resume a named session
/goal [text|sub]     Set a standing goal third-party works on across turns until achieved
                     (subcommands: status, pause, resume, clear)
/redraw              Force a full UI repaint (CLI)
```

### Configuration
```
/config              Show config (CLI)
/model [name]        Show or change model
/personality [name]  Set personality
/reasoning [level]   Set reasoning (none|minimal|low|medium|high|xhigh|show|hide)
/verbose             Cycle: off → new → all → verbose
/voice [on|off|tts]  Voice mode
/yolo                Toggle approval bypass
/busy [sub]          Control what Enter does while third-party is working (CLI)
                     (subcommands: queue, steer, interrupt, status)
/indicator [style]   Pick the TUI busy-indicator style (CLI)
                     (styles: kaomoji, emoji, unicode, ascii)
/footer [on|off]     Toggle gateway runtime-metadata footer on final replies
/skin [name]         Change theme (CLI)
/statusbar           Toggle status bar (CLI)
```

### Tools & Skills
```
/tools               Manage tools (CLI)
/toolsets            List toolsets (CLI)
/skills              Search/install skills (CLI)
/skill <name>        Load a skill into session
/reload-skills       Re-scan ~/.third-party/skills/ for added/removed skills
/reload              Reload .env variables into the running session (CLI)
/reload-mcp          Reload MCP servers
/cron                Manage cron jobs (CLI)
/curator [sub]       Background skill maintenance (status, run, pin, archive, …)
/kanban [sub]        Multi-profile collaboration board (tasks, links, comments)
/plugins             List plugins (CLI)
```

### Gateway
```
/approve             Approve a pending command (gateway)
/deny                Deny a pending command (gateway)
/restart             Restart gateway (gateway)
/sethome             Set current chat as home channel (gateway)
/update              Update third-party to latest (gateway)
/topic [sub]         Enable or inspect Telegram DM topic sessions (gateway)
/platforms (/gateway) Show platform connection status (gateway)
```

### Utility
```
/branch (/fork)      Branch the current session
/fast                Toggle priority/fast processing
/browser             Open CDP browser connection
/history             Show conversation history (CLI)
/save                Save conversation to file (CLI)
/copy [N]            Copy the last assistant response to clipboard (CLI)
/paste               Attach clipboard image (CLI)
/image               Attach local image file (CLI)
```

### Info
```
/help                Show commands
/commands [page]     Browse all commands (gateway)
/usage               Token usage
/insights [days]     Usage analytics
/status              Session info (gateway)
/profile             Active profile info
/debug               Upload debug report (system info + logs) and get shareable links
```

### Exit
```
/quit (/exit, /q)    Exit CLI
```

---

## Key Paths & Config

```
~/.third-party/config.yaml       Main configuration
~/.third-party/.env              API keys and secrets (under $THIRD_PARTY_HOME if set)
$THIRD_PARTY_HOME/skills/        Installed skills
~/.third-party/sessions/         Gateway routing index, request dumps, *.jsonl transcripts (and optional per-session JSON snapshots when sessions.write_json_snapshots: true)
~/.third-party/state.db          Canonical session store (SQLite + FTS5)
~/.third-party/logs/             Gateway and error logs
~/.third-party/auth.json         OAuth tokens and credential pools
~/.third-party/third-party agent/     Source code (if git-installed)
```

Profiles use `~/.third-party/profiles/<name>/` with the same layout.

### Config Sections

Edit with `third-party config edit` or `third-party config set section.key value`.

| Section | Key options |
|---------|-------------|
| `model` | `default`, `provider`, `base_url`, `api_key`, `context_length` |
| `agent` | `max_turns` (90), `tool_use_enforcement` |
| `terminal` | `backend` (local/docker/ssh/modal), `cwd`, `timeout` (180) |
| `compression` | `enabled`, `threshold` (0.50), `target_ratio` (0.20) |
| `display` | `skin`, `tool_progress`, `show_reasoning`, `show_cost` |
| `stt` | `enabled`, `provider` (local/groq/openai/mistral) |
| `tts` | `provider` (edge/elevenlabs/openai/minimax/mistral/neutts) |
| `memory` | `memory_enabled`, `user_profile_enabled`, `provider` |
| `security` | `tirith_enabled`, `website_blocklist` |
| `delegation` | `model`, `provider`, `base_url`, `api_key`, `max_iterations` (50), `reasoning_effort` |
| `checkpoints` | `enabled`, `max_snapshots` (50) |

Full config reference: https://third-party agent.third-party.com/docs/user-guide/configuration

### Providers

20+ providers supported. Set via `third-party model` or `third-party setup`.

| Provider | Auth | Key env var |
|----------|------|-------------|
| OpenRouter | API key | `OPENROUTER_API_KEY` |
| Anthropic | API key | `ANTHROPIC_API_KEY` |
| Nous Portal | OAuth | `third-party auth` |
| OpenAI Codex | OAuth | `third-party auth` |
| GitHub Copilot | Token | `COPILOT_GITHUB_TOKEN` |
| Google Gemini | API key | `GOOGLE_API_KEY` or `GEMINI_API_KEY` |
| DeepSeek | API key | `DEEPSEEK_API_KEY` |
| xAI / Grok | API key | `XAI_API_KEY` |
| Hugging Face | Token | `HF_TOKEN` |
| Z.AI / GLM | API key | `GLM_API_KEY` |
| MiniMax | API key | `MINIMAX_API_KEY` |
| MiniMax CN | API key | `MINIMAX_CN_API_KEY` |
| Kimi / Moonshot | API key | `KIMI_API_KEY` |
| Alibaba / DashScope | API key | `DASHSCOPE_API_KEY` |
| Xiaomi MiMo | API key | `XIAOMI_API_KEY` |
| Kilo Code | API key | `KILOCODE_API_KEY` |
| OpenCode Zen | API key | `OPENCODE_ZEN_API_KEY` |
| OpenCode Go | API key | `OPENCODE_GO_API_KEY` |
| Qwen OAuth | OAuth | `third-party auth add qwen-oauth` |
| Custom endpoint | Config | `model.base_url` + `model.api_key` in config.yaml |
| GitHub Copilot ACP | External | `COPILOT_CLI_PATH` or Copilot CLI |

Full provider docs: https://third-party agent.third-party.com/docs/integrations/providers

### Toolsets

Enable/disable via `third-party tools` (interactive) or `third-party tools enable/disable NAME`.

| Toolset | What it provides |
|---------|-----------------|
| `web` | Web search and content extraction |
| `search` | Web search only (subset of `web`) |
| `browser` | Browser automation (Browserbase, Camofox, or local Chromium) |
| `terminal` | Shell commands and process management |
| `file` | File read/write/search/patch |
| `code_execution` | Sandboxed Python execution |
| `vision` | Image analysis |
| `image_gen` | AI image generation |
| `video` | Video analysis and generation |
| `tts` | Text-to-speech |
| `skills` | Skill browsing and management |
| `memory` | Persistent cross-session memory |
| `session_search` | Search past conversations |
| `delegation` | Subagent task delegation |
| `cronjob` | Scheduled task management |
| `clarify` | Ask user clarifying questions |
| `messaging` | Cross-platform message sending |
| `todo` | In-session task planning and tracking |
| `kanban` | Multi-agent work-queue tools (gated to workers) |
| `debugging` | Extra introspection/debug tools (off by default) |
| `safe` | Minimal, low-risk toolset for locked-down sessions |
| `spotify` | Spotify playback and playlist control |
| `homeassistant` | Smart home control (off by default) |
| `discord` | Discord integration tools |
| `discord_admin` | Discord admin/moderation tools |
| `feishu_doc` | Feishu (Lark) document tools |
| `feishu_drive` | Feishu (Lark) drive tools |
| `yuanbao` | Yuanbao integration tools |
| `rl` | Reinforcement learning tools (off by default) |
| `moa` | Mixture of Agents (off by default) |

Full enumeration lives in `toolsets.py` as the `TOOLSETS` dict; `_third-party_CORE_TOOLS` is the default bundle most platforms inherit from.

Tool changes take effect on `/reset` (new session). They do NOT apply mid-conversation to preserve prompt caching.

---

## Project Context Files

third-party injects project-level instructions into the system prompt by reading context files from the working directory. The discovery order is **first match wins** — only one project context source is loaded per session.

| File (in priority order) | Discovery | Use when |
|---|---|---|
| `.third-party.md` / `third-party.md` | Walks parents up to the git root, stops at git root | You want hierarchical project rules (root + per-package overrides) |
| `AGENTS.md` / `agents.md` | **Cwd only** — subdirectory and parent copies are ignored | You want portable agent instructions that work the same in third-party, Claude Code, Codex, etc. |
| `CLAUDE.md` / `claude.md` | Cwd only | Same as AGENTS.md, Claude-flavored |
| `.cursorrules` / `.cursor/rules/*.mdc` | Cwd only | Migrating from Cursor |

`SOUL.md` (in `$THIRD_PARTY_HOME`) is independent and always loaded when present — it sets the agent's identity, not project rules.

### Pick the right one

- **Use `.third-party.md`** when you want third-party-specific behavior that lives above the cwd (root + subtree), or when you want rules to inherit from a parent directory. The parent walk stops at the git root, so a home-level `.third-party.md` won't leak into every project (a git repo's root is the boundary).
- **Use `AGENTS.md`** when the same project will also be worked on by other agents (Codex, Claude Code, OpenCode). Those tools all have their own conventions for `AGENTS.md`, and the "cwd only" contract keeps the file portable.
- **Don't put project rules in `~/.third-party/AGENTS.md`** (or any other home-level location). When third-party runs with that directory as cwd, the file loads — but only for that one directory. For cross-project context, use `SOUL.md` (in `$THIRD_PARTY_HOME`, identity-only) or install a skill via `third-party skills install`.

### Size and truncation

Each context file is capped at 20,000 characters. Files longer than that get **head + tail** truncated (the middle is dropped, with a `[...truncated...]` marker). For large project rules, prefer splitting into multiple skills over cramming one file.

### Security

All context files pass through the threat-pattern scanner before reaching the system prompt. Patterns matching prompt injection or promptware are replaced with a `[BLOCKED: ...]` placeholder. This means an `AGENTS.md` containing obvious injection attempts won't reach the model — the scanner blocks the content, not the file, so the rest of the file still loads.

### Disable for one session

`third-party --ignore-rules` skips auto-injection of all project context files (`.third-party.md`, `AGENTS.md`, `CLAUDE.md`, `.cursorrules`) **and** `SOUL.md` identity, plus user config, plugins, and MCP servers. Use it to isolate whether a problem is your setup or third-party itself.

### Example: a small `.third-party.md`

```markdown
# My Project

third-party: when working in this repo, follow these rules.

## Build
- Always run `make test` before declaring a change done.
- Use `uv run` for Python, not `pip install`.

## Style
- Prefer `pathlib.Path` over `os.path`.
- No `print()` in production code — use the `logger`.
```

That file at `/home/me/projects/myrepo/.third-party.md` is auto-loaded when third-party runs in any subdirectory of `/home/me/projects/myrepo`, but not when it runs in `/home/me/other-project`.

## Security & Privacy Toggles

Common "why is third-party doing X to my output / tool calls / commands?" toggles — and the exact commands to change them. Most of these need a fresh session (`/reset` in chat, or start a new `third-party` invocation) because they're read once at startup.

### Secret redaction in tool output

Secret redaction is **on by default** — tool output (terminal stdout, `read_file`, web content, subagent summaries, etc.) is scanned for strings that look like API keys, tokens, and secrets before it enters the conversation context and logs. Leave it enabled for normal use:

```bash
third-party config set security.redact_secrets true       # keep enabled globally
```

**Restart required.** `security.redact_secrets` is snapshotted at import time — toggling it mid-session (e.g. via `export third-party_REDACT_SECRETS=false` from a tool call) will NOT take effect for the running process. Tell the user to change it in config from a terminal, then start a new session. This is deliberate — it prevents an LLM from flipping the toggle on itself mid-task.

Disable only when you deliberately need raw credential-like strings for debugging or redactor development:
```bash
third-party config set security.redact_secrets false
```

### PII redaction in gateway messages

Separate from secret redaction. When enabled, the gateway hashes user IDs and strips phone numbers from the session context before it reaches the model:

```bash
third-party config set privacy.redact_pii true    # enable
third-party config set privacy.redact_pii false   # disable (default)
```

### Command approval prompts

By default (`approvals.mode: manual`), third-party prompts the user before running shell commands flagged as destructive (`rm -rf`, `git reset --hard`, etc.). The modes are:

- `manual` — always prompt (default)
- `smart` — use an auxiliary LLM to auto-approve low-risk commands, prompt on high-risk
- `off` — skip all approval prompts (equivalent to `--yolo`)

```bash
third-party config set approvals.mode smart       # recommended middle ground
third-party config set approvals.mode off         # bypass everything (not recommended)
```

Per-invocation bypass without changing config:
- `third-party --yolo …`
- `export third-party_YOLO_MODE=1`

Note: YOLO / `approvals.mode: off` does NOT turn off secret redaction. They are independent.

### Shell hooks allowlist

Some shell-hook integrations require explicit allowlisting before they fire. Managed via `~/.third-party/shell-hooks-allowlist.json` — prompted interactively the first time a hook wants to run.

### Disabling the web/browser/image-gen tools

To keep the model away from network or media tools entirely, open `third-party tools` and toggle per-platform. Takes effect on next session (`/reset`). See the Tools & Skills section above.

---

## Voice & Transcription

### STT (Voice → Text)

Voice messages from messaging platforms are auto-transcribed.

Provider priority (auto-detected):
1. **Local faster-whisper** — free, no API key: `pip install faster-whisper`
2. **Groq Whisper** — free tier: set `GROQ_API_KEY`
3. **OpenAI Whisper** — paid: set `VOICE_TOOLS_OPENAI_KEY`
4. **Mistral Voxtral** — set `MISTRAL_API_KEY`

Config:
```yaml
stt:
  enabled: true
  provider: local        # local, groq, openai, mistral
  local:
    model: base          # tiny, base, small, medium, large-v3
```

### TTS (Text → Voice)

| Provider | Env var | Free? |
|----------|---------|-------|
| Edge TTS | None | Yes (default) |
| ElevenLabs | `ELEVENLABS_API_KEY` | Free tier |
| OpenAI | `VOICE_TOOLS_OPENAI_KEY` | Paid |
| MiniMax | `MINIMAX_API_KEY` | Paid |
| Mistral (Voxtral) | `MISTRAL_API_KEY` | Paid |
| NeuTTS (local) | None (`pip install neutts[all]` + `espeak-ng`) | Free |

Voice commands: `/voice on` (voice-to-voice), `/voice tts` (always voice), `/voice off`.

---

## Spawning Additional third-party Instances

Run additional third-party processes as fully independent subprocesses — separate sessions, tools, and environments.

### When to Use This vs delegate_task

| | `delegate_task` | Spawning `third-party` process |
|-|-----------------|--------------------------|
| Isolation | Separate conversation, shared process | Fully independent process |
| Duration | Minutes (bounded by parent loop) | Hours/days |
| Tool access | Subset of parent's tools | Full tool access |
| Interactive | No | Yes (PTY mode) |
| Use case | Quick parallel subtasks | Long autonomous missions |

### One-Shot Mode

```
shell_exec(command="third-party chat -q 'Research GRPO papers and write summary to ~/research/grpo.md'", timeout=300)

# Background for long tasks:
shell_exec(command="third-party chat -q 'Set up CI/CD for ~/myapp'", background=true)
```

### Interactive PTY Mode (via tmux)

third-party uses prompt_toolkit, which requires a real terminal. Use tmux for interactive spawning:

```
# Start
shell_exec(command="tmux new-session -d -s agent1 -x 120 -y 40 'third-party'", timeout=10)

# Wait for startup, then send a message
shell_exec(command="sleep 8 && tmux send-keys -t agent1 'Build a FastAPI auth service' Enter", timeout=15)

# Read output
shell_exec(command="sleep 20 && tmux capture-pane -t agent1 -p", timeout=5)

# Send follow-up
shell_exec(command="tmux send-keys -t agent1 'Add rate limiting middleware' Enter", timeout=5)

# Exit
shell_exec(command="tmux send-keys -t agent1 '/exit' Enter && sleep 2 && tmux kill-session -t agent1", timeout=10)
```

### Multi-Agent Coordination

```
# Agent A: backend
shell_exec(command="tmux new-session -d -s backend -x 120 -y 40 'third-party -w'", timeout=10)
shell_exec(command="sleep 8 && tmux send-keys -t backend 'Build REST API for user management' Enter", timeout=15)

# Agent B: frontend
shell_exec(command="tmux new-session -d -s frontend -x 120 -y 40 'third-party -w'", timeout=10)
shell_exec(command="sleep 8 && tmux send-keys -t frontend 'Build React dashboard for user management' Enter", timeout=15)

# Check progress, relay context between them
shell_exec(command="tmux capture-pane -t backend -p | tail -30", timeout=5)
shell_exec(command="tmux send-keys -t frontend 'Here is the API schema from the backend agent: ...' Enter", timeout=5)
```

### Session Resume

```
# Resume most recent session
shell_exec(command="tmux new-session -d -s resumed 'third-party --continue'", timeout=10)

# Resume specific session
shell_exec(command="tmux new-session -d -s resumed 'third-party --resume 20260225_143052_a1b2c3'", timeout=10)
```

### Tips

- **Prefer `delegate_task` for quick subtasks** — less overhead than spawning a full process
- **Use `-w` (worktree mode)** when spawning agents that edit code — prevents git conflicts
- **Set timeouts** for one-shot mode — complex tasks can take 5-10 minutes
- **Use `third-party chat -q` for fire-and-forget** — no PTY needed
- **Use tmux for interactive sessions** — raw PTY mode has `\r` vs `\n` issues with prompt_toolkit
- **For scheduled tasks**, use the `cronjob` tool instead of spawning — handles delivery and retry

---

## Durable & Background Systems

Four systems run alongside the main conversation loop. Quick reference
here; full developer notes live in `AGENTS.md`, user-facing docs under
`website/docs/user-guide/features/`.

### Delegation (`delegate_task`)

Synchronous subagent spawn — the parent waits for the child's summary
before continuing its own loop. Isolated context + terminal session.

- **Single:** `delegate_task(goal, context, toolsets)`.
- **Batch:** `delegate_task(tasks=[{goal, ...}, ...])` runs children in
  parallel, capped by `delegation.max_concurrent_children` (default 3).
- **Roles:** `leaf` (default; cannot re-delegate) vs `orchestrator`
  (can spawn its own workers, bounded by `delegation.max_spawn_depth`).
- **Not durable.** If the parent is interrupted, the child is
  cancelled. For work that must outlive the turn, use `cronjob` or
  `shell_exec(background=True, notify_on_complete=True)`.

Config: `delegation.*` in `config.yaml`.

### Cron (scheduled jobs)

Durable scheduler — `cron/jobs.py` + `cron/scheduler.py`. Drive it via
the `cronjob` tool, the `third-party cron` CLI (`list`, `add`, `edit`,
`pause`, `resume`, `run`, `remove`), or the `/cron` slash command.

- **Schedules:** duration (`"30m"`, `"2h"`), "every" phrase
  (`"every monday 9am"`), 5-field cron (`"0 9 * * *"`), or ISO timestamp.
- **Per-job knobs:** `skills`, `model`/`provider` override, `script`
  (pre-run data collection; `no_agent=True` makes the script the whole
  job), `context_from` (chain job A's output into job B), `workdir`
  (run in a specific dir with its `AGENTS.md` / `CLAUDE.md` loaded),
  multi-platform delivery.
- **Invariants:** 3-minute hard interrupt per run, `.tick.lock` file
  prevents duplicate ticks across processes, cron sessions pass
  `skip_memory=True` by default, and cron deliveries are framed with a
  header/footer instead of being mirrored into the target gateway
  session (keeps role alternation intact).

User docs: https://third-party agent.third-party.com/docs/user-guide/features/cron

### Curator (skill lifecycle)

Background maintenance for agent-created skills. Tracks usage, marks
idle skills stale, archives stale ones, keeps a pre-run tar.gz backup
so nothing is lost.

- **CLI:** `third-party curator <verb>` — `status`, `run`, `pause`, `resume`,
  `pin`, `unpin`, `archive`, `restore`, `prune`, `backup`, `rollback`.
- **Slash:** `/curator <subcommand>` mirrors the CLI.
- **Scope:** only touches skills with `created_by: "agent"` provenance.
  Bundled + hub-installed skills are off-limits. **Never deletes** —
  max destructive action is archive. Pinned skills are exempt from
  every auto-transition and every LLM review pass.
- **Telemetry:** sidecar at `~/.third-party/skills/.usage.json` holds
  per-skill `use_count`, `view_count`, `patch_count`,
  `last_activity_at`, `state`, `pinned`.

Config: `curator.*` (`enabled`, `interval_hours`, `min_idle_hours`,
`stale_after_days`, `archive_after_days`, `backup.*`).
User docs: https://third-party agent.third-party.com/docs/user-guide/features/curator

### Kanban (multi-agent work queue)

Durable SQLite board for multi-profile / multi-worker collaboration.
Users drive it via `third-party kanban <verb>`; dispatcher-spawned workers
see a focused `kanban_*` toolset gated by `third-party_KANBAN_TASK`, and
orchestrator profiles can opt into the broader `kanban` toolset. Normal
sessions still have zero `kanban_*` schema footprint unless configured.

- **CLI verbs (common):** `init`, `create`, `list` (alias `ls`),
  `show`, `assign`, `link`, `unlink`, `comment`, `complete`, `block`,
  `unblock`, `archive`, `tail`. Less common: `watch`, `stats`, `runs`,
  `log`, `dispatch`, `daemon`, `gc`.
- **Worker/orchestrator toolset:** `kanban_show`, `kanban_complete`,
  `kanban_block`, `kanban_heartbeat`, `kanban_comment`, `kanban_create`,
  `kanban_link`; profiles that explicitly enable the `kanban` toolset
  outside a dispatcher-spawned task also get `kanban_list` and
  `kanban_unblock` for board routing.
- **Dispatcher** runs inside the gateway by default
  (`kanban.dispatch_in_gateway: true`) — reclaims stale claims,
  promotes ready tasks, atomically claims, spawns assigned profiles.
  Auto-blocks a task after `failure_limit` consecutive spawn failures
  (default 2; configurable via `kanban.failure_limit` or per-task
  `max_retries`).
- **Isolation:** board is the hard boundary (workers get
  `third-party_KANBAN_BOARD` pinned in env); tenant is a soft namespace
  within a board for workspace-path + memory-key isolation.

User docs: https://third-party agent.third-party.com/docs/user-guide/features/kanban

---

## Windows-Specific Quirks

third-party runs natively on Windows (PowerShell, cmd, Windows Terminal, git-bash
mintty, VS Code integrated terminal). Most of it just works, but a handful
of differences between Win32 and POSIX have bitten us — document new ones
here as you hit them so the next person (or the next session) doesn't
rediscover them from scratch.

### Input / Keybindings

**Alt+Enter doesn't insert a newline.** Windows Terminal intercepts Alt+Enter
at the terminal layer to toggle fullscreen — the keystroke never reaches
prompt_toolkit. Use **Ctrl+Enter** instead. Windows Terminal delivers
Ctrl+Enter as LF (`c-j`), distinct from plain Enter (`c-m` / CR), and the
CLI binds `c-j` to newline insertion on `win32` only (see
`_bind_prompt_submit_keys` + the Windows-only `c-j` binding in `cli.py`).
Side effect: the raw Ctrl+J keystroke also inserts a newline on Windows —
unavoidable, because Windows Terminal collapses Ctrl+Enter and Ctrl+J to
the same keycode at the Win32 console API layer. No conflicting binding
existed for Ctrl+J on Windows, so this is a harmless side effect.

mintty / git-bash behaves the same (fullscreen on Alt+Enter) unless you
disable Alt+Fn shortcuts in Options → Keys. Easier to just use Ctrl+Enter.

**Diagnosing keybindings.** Run `python scripts/keystroke_diagnostic.py`
(repo root) to see exactly how prompt_toolkit identifies each keystroke
in the current terminal. Answers questions like "does Shift+Enter come
through as a distinct key?" (almost never — most terminals collapse it
to plain Enter) or "what byte sequence is my terminal sending for
Ctrl+Enter?" This is how the Ctrl+Enter = c-j fact was established.

### Config / Files

**HTTP 400 "No models provided" on first run.** `config.yaml` was saved
with a UTF-8 BOM (common when Windows apps write it). Re-save as UTF-8
without BOM. `third-party config edit` writes without BOM; manual edits in
Notepad are the usual culprit.

### `execute_code` / Sandbox

**WinError 10106** ("The requested service provider could not be loaded
or initialized") from the sandbox child process — it can't create an
`AF_INET` socket, so the loopback-TCP RPC fallback fails before
`connect()`. Root cause is usually **not** a broken Winsock LSP; it's
third-party's own env scrubber dropping `SYSTEMROOT` / `WINDIR` / `COMSPEC`
from the child env. Python's `socket` module needs `SYSTEMROOT` to locate
`mswsock.dll`. Fixed via the `_WINDOWS_ESSENTIAL_ENV_VARS` allowlist in
`tools/code_execution_tool.py`. If you still hit it, echo `os.environ`
inside an `execute_code` block to confirm `SYSTEMROOT` is set. Full
diagnostic recipe in `references/execute-code-sandbox-env-windows.md`.

### Testing / Contributing

**`scripts/run_tests.sh` doesn't work as-is on Windows** — it looks for
POSIX venv layouts (`.venv/bin/activate`). The third-party-installed venv at
`venv/Scripts/` has no pip or pytest either (stripped for install size).
Workaround: install `pytest + pytest-xdist + pyyaml` into a system Python
3.11 user site, then invoke pytest directly with `PYTHONPATH` set:

```bash
"/c/Program Files/Python311/python" -m pip install --user pytest pytest-xdist pyyaml
export PYTHONPATH="$(pwd)"
"/c/Program Files/Python311/python" -m pytest tests/foo/test_bar.py -v --tb=short -n 0
```

Use `-n 0`, not `-n 4` — `pyproject.toml`'s default `addopts` already
includes `-n`, and the wrapper's CI-parity guarantees don't apply off POSIX.

**POSIX-only tests need skip guards.** Common markers already in the codebase:
- Symlinks — elevated privileges on Windows
- `0o600` file modes — POSIX mode bits not enforced on NTFS by default
- `signal.SIGALRM` — Unix-only (see `tests/conftest.py::_enforce_test_timeout`)
- Winsock / Windows-specific regressions — `@pytest.mark.skipif(sys.platform != "win32", ...)`

Use the existing skip-pattern style (`sys.platform == "win32"` or
`sys.platform.startswith("win")`) to stay consistent with the rest of the
suite.

### Path / Filesystem

**Line endings.** Git may warn `LF will be replaced by CRLF the next time
Git touches it`. Cosmetic — the repo's `.gitattributes` normalizes. Don't
let editors auto-convert committed POSIX-newline files to CRLF.

**Forward slashes work almost everywhere.** `C:/Users/...` is accepted by
every third-party tool and most Windows APIs. Prefer forward slashes in code
and logs — avoids shell-escaping backslashes in bash.

---

## Troubleshooting

### Voice not working
1. Check `stt.enabled: true` in config.yaml
2. Verify provider: `pip install faster-whisper` or set API key
3. In gateway: `/restart`. In CLI: exit and relaunch.

### Tool not available
1. `third-party tools` — check if toolset is enabled for your platform
2. Some tools need env vars (check `.env`)
3. `/reset` after enabling tools

### Model/provider issues
1. `third-party doctor` — check config and dependencies
2. `third-party auth` — re-authenticate OAuth providers (or `third-party auth add <provider>`)
3. Check `.env` has the right API key
4. **Copilot 403**: `gh auth login` tokens do NOT work for Copilot API. You must use the Copilot-specific OAuth device code flow via `third-party model` → GitHub Copilot.

### Changes not taking effect
- **Tools/skills:** `/reset` starts a new session with updated toolset
- **Config changes:** In gateway: `/restart`. In CLI: exit and relaunch.
- **Code changes:** Restart the CLI or gateway process

### Skills not showing
1. `third-party skills list` — verify installed
2. `third-party skills config` — check platform enablement
3. Load explicitly: `/skill name` or `third-party -s name`

### Gateway issues
Check logs first:
```bash
grep -i "failed to send\|error" ~/.third-party/logs/gateway.log | tail -20
```

Common gateway problems:
- **Gateway dies on SSH logout**: Enable linger: `sudo loginctl enable-linger $USER`
- **Gateway dies on WSL2 close**: WSL2 requires `systemd=true` in `/etc/wsl.conf` for systemd services to work. Without it, gateway falls back to `nohup` (dies when session closes).
- **Gateway crash loop**: Reset the failed state: `systemctl --user reset-failed third-party-gateway`

### Platform-specific issues
- **Discord bot silent**: Must enable **Message Content Intent** in Bot → Privileged Gateway Intents.
- **Slack bot only works in DMs**: Must subscribe to `message.channels` event. Without it, the bot ignores public channels.
- **Windows-specific issues** (`Alt+Enter` newline, WinError 10106, UTF-8 BOM config, test suite, line endings): see the dedicated **Windows-Specific Quirks** section above.

### Auxiliary models not working
If `auxiliary` tasks (vision, compression, session_search) fail silently, the `auto` provider can't find a backend. Either set `OPENROUTER_API_KEY` or `GOOGLE_API_KEY`, or explicitly configure each auxiliary task's provider:
```bash
third-party config set auxiliary.vision.provider <your_provider>
third-party config set auxiliary.vision.model <model_name>
```

---

## Where to Find Things

| Looking for... | Location |
|----------------|----------|
| Config options | `third-party config edit` or [Configuration docs](https://third-party agent.third-party.com/docs/user-guide/configuration) |
| Available tools | `third-party tools list` or [Tools reference](https://third-party agent.third-party.com/docs/reference/tools-reference) |
| Slash commands | `/help` in session or [Slash commands reference](https://third-party agent.third-party.com/docs/reference/slash-commands) |
| Skills catalog | `third-party skills browse` or [Skills catalog](https://third-party agent.third-party.com/docs/reference/skills-catalog) |
| Provider setup | `third-party model` or [Providers guide](https://third-party agent.third-party.com/docs/integrations/providers) |
| Platform setup | `third-party gateway setup` or [Messaging docs](https://third-party agent.third-party.com/docs/user-guide/messaging/) |
| MCP servers | `third-party mcp list` or [MCP guide](https://third-party agent.third-party.com/docs/user-guide/features/mcp) |
| Profiles | `third-party profile list` or [Profiles docs](https://third-party agent.third-party.com/docs/user-guide/profiles) |
| Cron jobs | `third-party cron list` or [Cron docs](https://third-party agent.third-party.com/docs/user-guide/features/cron) |
| Memory | `third-party memory status` or [Memory docs](https://third-party agent.third-party.com/docs/user-guide/features/memory) |
| Env variables | `third-party config env-path` or [Env vars reference](https://third-party agent.third-party.com/docs/reference/environment-variables) |
| CLI commands | `third-party --help` or [CLI reference](https://third-party agent.third-party.com/docs/reference/cli-commands) |
| Gateway logs | `~/.third-party/logs/gateway.log` |
| Session files | `third-party sessions browse` (reads state.db) |
| Source code | `~/.third-party/third-party agent/` |

---

## Contributor Quick Reference

For occasional contributors and PR authors. Full developer docs: https://third-party agent.third-party.com/docs/developer-guide/

### Project Layout

```
third-party agent/
├── run_agent.py          # AIAgent — core conversation loop
├── model_tools.py        # Tool discovery and dispatch
├── toolsets.py           # Toolset definitions
├── cli.py                # Interactive CLI (third-partyCLI)
├── third-party_state.py       # SQLite session store
├── agent/                # Prompt builder, context compression, memory, model routing, credential pooling, skill dispatch
├── third-party_cli/           # CLI subcommands, config, setup, commands
│   ├── commands.py       # Slash command registry (CommandDef)
│   ├── config.py         # DEFAULT_CONFIG, env var definitions
│   └── main.py           # CLI entry point and argparse
├── tools/                # One file per tool
│   └── registry.py       # Central tool registry
├── gateway/              # Messaging gateway
│   └── platforms/        # Platform adapters (telegram, discord, etc.)
├── cron/                 # Job scheduler
├── tests/                # ~3000 pytest tests
└── website/              # Docusaurus docs site
```

Config: `~/.third-party/config.yaml` (settings), `~/.third-party/.env` (API keys) — both under `$THIRD_PARTY_HOME` when it is set.

### Adding a Tool (3 files)

**1. Create `tools/your_tool.py`:**
```python
import json, os
from tools.registry import registry

def check_requirements() -> bool:
    return bool(os.getenv("EXAMPLE_API_KEY"))

def example_tool(param: str, task_id: str = None) -> str:
    return json.dumps({"success": True, "data": "..."})

registry.register(
    name="example_tool",
    toolset="example",
    schema={"name": "example_tool", "description": "...", "parameters": {...}},
    handler=lambda args, **kw: example_tool(
        param=args.get("param", ""), task_id=kw.get("task_id")),
    check_fn=check_requirements,
    requires_env=["EXAMPLE_API_KEY"],
)
```

**2. Add to `toolsets.py`** → `_third-party_CORE_TOOLS` list.

Auto-discovery: any `tools/*.py` file with a top-level `registry.register()` call is imported automatically — no manual list needed.

All handlers must return JSON strings. Use `get_THIRD_PARTY_HOME()` for paths, never hardcode `~/.third-party`.

### Adding a Slash Command

1. Add `CommandDef` to `COMMAND_REGISTRY` in `third-party_cli/commands.py`
2. Add handler in `cli.py` → `process_command()`
3. (Optional) Add gateway handler in `gateway/run.py`

All consumers (help text, autocomplete, Telegram menu, Slack mapping) derive from the central registry automatically.

### Agent Loop (High Level)

```
run_conversation():
  1. Build system prompt
  2. Loop while iterations < max:
     a. Call LLM (OpenAI-format messages + tool schemas)
     b. If tool_calls → dispatch each via handle_function_call() → append results → continue
     c. If text response → return
  3. Context compression triggers automatically near token limit
```

### Testing

```bash
python -m pytest tests/ -o 'addopts=' -q   # Full suite
python -m pytest tests/tools/ -q            # Specific area
```

- Tests auto-redirect `THIRD_PARTY_HOME` to temp dirs — never touch real `~/.third-party/`
- Run full suite before pushing any change
- Use `-o 'addopts='` to clear any baked-in pytest flags

**Windows contributors:** `scripts/run_tests.sh` currently looks for POSIX venvs (`.venv/bin/activate` / `venv/bin/activate`) and will error out on Windows where the layout is `venv/Scripts/activate` + `python.exe`. The third-party-installed venv at `venv/Scripts/` also has no `pip` or `pytest` — it's stripped for end-user install size. Workaround: install pytest + pytest-xdist + pyyaml into a system Python 3.11 user site (`/c/Program Files/Python311/python -m pip install --user pytest pytest-xdist pyyaml`), then run tests directly:

```bash
export PYTHONPATH="$(pwd)"
"/c/Program Files/Python311/python" -m pytest tests/tools/test_foo.py -v --tb=short -n 0
```

Use `-n 0` (not `-n 4`) because `pyproject.toml`'s default `addopts` already includes `-n`, and the wrapper's CI-parity story doesn't apply off-POSIX.

**Cross-platform test guards:** tests that use POSIX-only syscalls need a skip marker. Common ones already in the codebase:
- Symlink creation → `@pytest.mark.skipif(sys.platform == "win32", reason="Symlinks require elevated privileges on Windows")` (see `tests/cron/test_cron_script.py`)
- POSIX file modes (0o600, etc.) → `@pytest.mark.skipif(sys.platform.startswith("win"), reason="POSIX mode bits not enforced on Windows")` (see `tests/third-party_cli/test_auth_toctou_file_modes.py`)
- `signal.SIGALRM` → Unix-only (see `tests/conftest.py::_enforce_test_timeout`)
- Live Winsock / Windows-specific regression tests → `@pytest.mark.skipif(sys.platform != "win32", reason="Windows-specific regression")`

**Monkeypatching `sys.platform` is not enough** when the code under test also calls `platform.system()` / `platform.release()` / `platform.mac_ver()`. Those functions re-read the real OS independently, so a test that sets `sys.platform = "linux"` on a Windows runner will still see `platform.system() == "Windows"` and route through the Windows branch. Patch all three together:

```python
monkeypatch.setattr(sys, "platform", "linux")
monkeypatch.setattr(platform, "system", lambda: "Linux")
monkeypatch.setattr(platform, "release", lambda: "6.8.0-generic")
```

See `tests/agent/test_prompt_builder.py::TestEnvironmentHints` for a worked example.

### Extending the system prompt's execution-environment block

Factual guidance about the host OS, user home, cwd, terminal backend, and shell (bash vs. PowerShell on Windows) is emitted from `agent/prompt_builder.py::build_environment_hints()`. This is also where the WSL hint and per-backend probe logic live. The convention:

- **Local terminal backend** → emit host info (OS, `$HOME`, cwd) + Windows-specific notes (hostname ≠ username, `terminal` uses bash not PowerShell).
- **Remote terminal backend** (anything in `_REMOTE_TERMINAL_BACKENDS`: `docker, singularity, modal, daytona, ssh, managed_modal`) → **suppress** host info entirely and describe only the backend. A live `uname`/`whoami`/`pwd` probe runs inside the backend via `tools.environments.get_environment(...).execute(...)`, cached per process in `_BACKEND_PROBE_CACHE`, with a static fallback if the probe times out.
- **Key fact for prompt authoring:** when `TERMINAL_ENV != "local"`, *every* file tool (`read_file`, `write_file`, `patch`, `search_files`) runs inside the backend container, not on the host. The system prompt must never describe the host in that case — the agent can't touch it.

Full design notes, the exact emitted strings, and testing pitfalls:
`references/prompt-builder-environment-hints.md`.

**Refactor-safety pattern (POSIX-equivalence guard):** when you extract inline logic into a helper that adds Windows/platform-specific behavior, keep a `_legacy_<name>` oracle function in the test file that's a verbatim copy of the old code, then parametrize-diff against it. Example: `tests/tools/test_code_execution_windows_env.py::TestPosixEquivalence`. This locks in the invariant that POSIX behavior is bit-for-bit identical and makes any future drift fail loudly with a clear diff.

### Commit Conventions

```
type: concise subject line

Optional body.
```

Types: `fix:`, `feat:`, `refactor:`, `docs:`, `chore:`

### Key Rules

- **Never break prompt caching** — don't change context, tools, or system prompt mid-conversation
- **Message role alternation** — never two assistant or two user messages in a row
- Use `get_THIRD_PARTY_HOME()` from `third-party_constants` for all paths (profile-safe)
- Config values go in `config.yaml`, secrets go in `.env`
- New tools need a `check_fn` so they only appear when requirements are met
