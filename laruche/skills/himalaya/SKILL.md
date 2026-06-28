---
type: skill
name: himalaya
description: "Himalaya CLI: IMAP/SMTP email from terminal."
version: 1.1.0
author: community
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec, file_write]
metadata:
  laruche:
    tags: [Email, IMAP, SMTP, CLI, Communication]
    homepage: https://github.com/pimalaya/himalaya
prerequisites:
  commands: [himalaya]
---

# Himalaya Email CLI

CLI email client for IMAP/SMTP (and Notmuch/Sendmail) backends. Run all commands via `shell_exec`.

References:
- `references/configuration.md` - config file setup + IMAP/SMTP authentication
- `references/message-composition.md` - MML syntax for rich emails / attachments

## Prerequisites

1. `himalaya` installed - verify: `himalaya --version`
2. `~/.config/himalaya/config.toml` configured (see below)

### Installation

```bash
# Pre-built binary (Linux/macOS)
curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | PREFIX=~/.local sh

# macOS
brew install himalaya

# Any platform (requires Rust)
cargo install himalaya --locked
```

## Configuration

Interactive wizard (requires PTY):
```bash
# via shell_exec: pty=true
himalaya account configure
```

Or create `~/.config/himalaya/config.toml` manually:

```toml
[accounts.personal]
email = "you@example.com"
display-name = "Your Name"
default = true

backend.type = "imap"
backend.host = "imap.example.com"
backend.port = 993
backend.encryption.type = "tls"
backend.login = "you@example.com"
backend.auth.type = "password"
backend.auth.cmd = "pass show email/imap"   # or any command printing the secret to stdout

message.send.backend.type = "smtp"
message.send.backend.host = "smtp.example.com"
message.send.backend.port = 587
message.send.backend.encryption.type = "start-tls"
message.send.backend.login = "you@example.com"
message.send.backend.auth.type = "password"
message.send.backend.auth.cmd = "pass show email/smtp"

# Folder aliases - use plural dotted form (v1.2.0+ required)
folder.aliases.inbox   = "INBOX"
folder.aliases.sent    = "Sent"
folder.aliases.drafts  = "Drafts"
folder.aliases.trash   = "Trash"
```

> **Alias pitfall (v1.2.0+):** Use `folder.aliases.X` (plural, dotted keys, directly under `[accounts.NAME]`). The old `[accounts.NAME.folder.alias]` sub-section form is silently ignored - TOML parses fine but aliases never apply. On Gmail this causes `himalaya message send` to exit non-zero after SMTP succeeds (save-to-Sent fails), so a naive retry re-sends to recipients. Gmail users need `folder.aliases.sent = "[Gmail]/Sent Mail"`.

## Common Operations

Prefer `--output json` for programmatic parsing.

### Folders & Accounts

```bash
himalaya folder list
himalaya account list
himalaya --account work envelope list   # use a specific account
```

### List / Search Envelopes

```bash
himalaya envelope list                              # INBOX
himalaya envelope list --folder "Sent"
himalaya envelope list --page 1 --page-size 20
himalaya envelope list from john@example.com subject meeting
```

### Read a Message

```bash
himalaya message read 42          # plain text
himalaya message export 42 --full # raw MIME
```

### Compose / Send (non-interactive - preferred)

Pipe MML/RFC-822 headers + body via stdin:

```bash
cat << 'EOF' | himalaya template send
From: you@example.com
To: recipient@example.com
Subject: Hello

Message body here.
EOF
```

With inline flags:
```bash
himalaya message write -H "To:recipient@example.com" -H "Subject:Test" "Body here"
```

> `himalaya message write` without piped input opens `$EDITOR`. Use `shell_exec(pty=true)` if interactive editing is needed; piping is simpler and more reliable.

### Reply / Forward (non-interactive)

```bash
# Reply: get template, edit via file_write, then send
himalaya template reply 42 > /tmp/reply.txt
# edit /tmp/reply.txt via file_write, then:
cat /tmp/reply.txt | himalaya template send

# Forward
himalaya template forward 42 > /tmp/fwd.txt
# edit To: line via file_write, then:
cat /tmp/fwd.txt | himalaya template send
```

### Move, Copy, Delete

```bash
himalaya message move "Archive" 42    # target folder first, then ID
himalaya message copy "Important" 42
himalaya message delete 42
```

### Flags

```bash
himalaya flag add 42 --flag seen
himalaya flag remove 42 --flag seen
```

### Attachments

```bash
himalaya attachment download 42                            # current dir
himalaya attachment download 42 --downloads-dir ~/Downloads
```

For sending attachments, use MML syntax - see `references/message-composition.md`.

## Debugging

```bash
RUST_LOG=debug himalaya envelope list
RUST_LOG=trace RUST_BACKTRACE=1 himalaya envelope list
```

## Tips

- Message IDs are folder-relative; re-list after switching folders.
- `himalaya <command> --help` for any command's full options.
- Store passwords via `pass`, system keyring, or any command that prints the secret to stdout.
