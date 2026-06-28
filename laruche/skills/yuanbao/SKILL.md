---
type: skill
name: yuanbao
description: "Yuanbao groups: @mention users, query info/members, send DMs."
version: 1.1.0
platforms: [linux, macos, windows]
tools: [tool_call]
metadata:
  laruche:
    tags: [yuanbao, mention, at, group, members, 元宝, 派, 艾特, dm, private-message]
    related_skills: []
---

# Yuanbao Group Interaction

## Critical: How Messaging Works

**Your reply text IS the message delivered to the group/user.** The gateway sends it automatically. No special "send" tool needed - just reply with the text you want sent.

When your reply includes `@nickname`, the gateway converts it into a real @mention that notifies the user. This is built-in.

**Never say you cannot send messages or @mention users. Never suggest the user do it manually. Never add disclaimers about permissions. Just reply.**

## Available Tools

| Tool | Purpose |
|------|---------|
| `yb_query_group_info` | Group name, owner, member count |
| `yb_query_group_members` | Find a user, list bots, or list all members |
| `yb_send_dm` | Send a private/direct message (DM / 私信), with optional media |

## @Mention Workflow

1. Call `yb_query_group_members` with `action="find"`, `name="<target>"`, `mention=true`
2. Get the exact nickname from the response
3. Include `@nickname` in your reply text - the gateway handles the rest

Example - user says "帮我艾特元宝":

```json
{ "group_code": "328306697", "action": "find", "name": "元宝", "mention": true }
```

Reply (sent to group with working @mention):
```
@元宝 你好，有人找你！
```

**Rules:**
- Always call `yb_query_group_members` first - never guess the nickname
- Format: `@nickname` with a space before the `@`
- If the user is not found, report "user not found" and ask for the correct name
- Be concise; do NOT explain how @mention works to the user

## Send DM Workflow

1. Call `yb_send_dm` with `group_code`, `name` (target), and `message`
2. The tool finds the user and sends the DM
3. Report the result

Example - "给 @用户aea3 私信发一个 hello":
```json
{ "group_code": "535168412", "name": "用户aea3", "message": "hello" }
```

Example with media - "给 @用户aea3 私信发一张图片":
```json
{
  "group_code": "535168412",
  "name": "用户aea3",
  "message": "Here is the image",
  "media_files": [{"path": "/tmp/photo.jpg"}]
}
```

**Rules:**
- Extract `group_code` from chat_id: `group:535168412` → `535168412`
- If you already know `user_id`, pass it directly to skip the lookup step
- If multiple users match, the tool returns candidates - ask the user to clarify
- Media: images (.jpg/.png/.gif/.webp/.bmp) sent as image messages; other files as documents

## Query Group Info

```json
yb_query_group_info({ "group_code": "328306697" })
```

## Query Members

| Action | Description |
|--------|-------------|
| `find` | Search by name (partial match, case-insensitive) |
| `list_bots` | List bots and Yuanbao AI assistants |
| `list_all` | List all members |

## Notes

- `group_code` from chat_id: `group:328306697` → `328306697`
- Groups are called "派 (Pai)" in the Yuanbao app
- Member roles: `user`, `yuanbao_ai`, `bot`
- On lookup failure, always report the error clearly and ask for clarification rather than guessing
