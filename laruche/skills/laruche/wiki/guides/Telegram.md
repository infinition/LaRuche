# Telegram

Telegram is LaRuche's most mature remote channel: full agent runs from your phone,
persistent per-channel memory, voice messages in both directions, and notifications
from crons and watchers.

## Setup

1. Create a bot with [@BotFather](https://t.me/BotFather) and copy the token.
2. In **Settings > Channels**, paste the token and enable Telegram. No restart needed.
3. Message your bot. The first chat can be bound as home with `/sethome`, so crons and
   watchers know where to notify you.

Store the token as a secret (see [Secrets](Secrets)) rather than pasting it into chats.

## What works

- **Full agentic runs**: the same engine as the web UI, tools included. Ask for
  research, file checks, scheduled jobs, from the phone.
- **Per-channel memory**: the Telegram channel has its own persistent conversation
  identity, distinct from your web sessions.
- **Voice both ways**: send a voice message, it is transcribed locally (Whisper) and
  answered; the answer can come back as synthesized speech (see [Voice](Voice)).
- **Notifications**: crons and watchers deliver to your home chat, respecting
  night-silence windows if the watcher's rules include one.
- **Per-channel model**: assign a smaller, faster model to Telegram in Settings if you
  want snappier phone replies.

## Commands

| Command | Effect |
|---|---|
| `/help` | List commands |
| `/status` | Node status |
| `/crons` | List scheduled jobs |
| `/delcron <name|all>` | Remove a cron |
| `/sethome` | Bind this chat as the notification home |
| `/clear` | Clear the channel conversation |

## Discord and Slack

Both are wired in the channels layer with the same per-channel memory model. Telegram
is the reference implementation and the most battle-tested.
