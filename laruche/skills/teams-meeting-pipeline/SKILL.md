---
type: skill
name: teams-meeting-pipeline
description: "Teams meeting summaries via Microsoft Graph: status, replay, subscriptions."
version: 1.2.0
author: Teknium
license: MIT
prerequisites:
  env_vars: [MSGRAPH_TENANT_ID, MSGRAPH_CLIENT_ID, MSGRAPH_CLIENT_SECRET]
tools: [shell_exec, cron_create]
metadata:
  laruche:
    tags: [Teams, Microsoft Graph, Meetings, Productivity, Operations]
    related_docs:
      - /docs/guides/microsoft-graph-app-registration
      - /docs/user-guide/messaging/teams-meetings
      - /docs/guides/operate-teams-meeting-pipeline
---

# Teams Meeting Pipeline

Use when the user asks about Microsoft Teams meeting summaries, transcripts, recordings, action items, Graph subscriptions, or pipeline operations.

All commands run via `shell_exec`. Secrets are injected from the LaRuche vault as `${MSGRAPH_TENANT_ID}`, `${MSGRAPH_CLIENT_ID}`, `${MSGRAPH_CLIENT_SECRET}` — never read from a file.

## Prerequisites

Azure AD app registration with admin-consented Graph **application** permissions. If missing, direct user to `/docs/guides/microsoft-graph-app-registration`.

## Command reference

### Status and inspection (start here)

```bash
teams-pipeline validate                          # config snapshot — run first after any change
teams-pipeline token-health                      # Graph token status
teams-pipeline token-health --force-refresh      # force fresh token acquisition
teams-pipeline list                              # recent meeting jobs
teams-pipeline list --status failed              # failed jobs only
teams-pipeline show <job-id>                     # full detail of one job
teams-pipeline subscriptions                     # current Graph webhook subscriptions
```

### Re-running / debugging

```bash
teams-pipeline run <job-id>                      # replay a stored job
teams-pipeline fetch --meeting-id <id>           # dry-run: resolve meeting + transcript, no persist
teams-pipeline fetch --join-web-url "<url>"      # dry-run by join URL
```

### Subscription management

```bash
teams-pipeline subscribe \
  --resource communications/onlineMeetings/getAllTranscripts \
  --notification-url https://<your-public-host>/msgraph/webhook \
  --client-state "${MSGRAPH_WEBHOOK_CLIENT_STATE}"

teams-pipeline renew-subscription <sub-id> --expiration <iso-8601>
teams-pipeline delete-subscription <sub-id>
teams-pipeline maintain-subscriptions            # renew near-expiry subscriptions
teams-pipeline maintain-subscriptions --dry-run  # preview what would be renewed
```

## Decision tree

- **"Why didn't I get a summary?"** → `list --status failed`, then `show <job-id>`. If job missing, check `subscriptions` — webhook likely expired.
- **"Is setup working?"** → `validate` → `token-health` → `subscriptions`. All three pass → request a test meeting → check `list` for a fresh row.
- **"Re-run summary for meeting X"** → `list` to find job ID → `run <job-id>`. Still fails → `show <job-id>` → `fetch --meeting-id` for dry-run.
- **"Add a past meeting"** → Pipeline is subscription-driven. For a specific past meeting use `fetch` to pull the transcript, then `run` once a job is created.

## Critical pitfall: Graph subscriptions expire in 72 hours

Microsoft Graph caps webhook subscriptions at 72 hours and **will not auto-renew**. Without renewal, meeting notifications silently stop after 3 days.

When user reports "worked yesterday, nothing arriving today":
1. `teams-pipeline subscriptions` — empty list or all `expirationDateTime` in the past confirms it.
2. Recreate with `subscribe` as above.
3. **Schedule automated renewal immediately** — 12-hour interval gives 6× headroom. Use `cron_create` in LaRuche or a system crontab: `0 */12 * * * teams-pipeline maintain-subscriptions`.

## Other pitfalls

- **Transcript not ready.** Teams takes 2–5 min after a meeting ends to generate transcripts. `fetch --meeting-id` on a just-ended meeting may return empty — wait and retry.
- **Delivery mode mismatch.** `list` shows success but nothing lands: check `platforms.teams.extra.delivery_mode` and matching target config (`incoming_webhook_url` / `chat_id` / `team_id`+`channel_id`) in config.yaml or `TEAMS_*` env vars.
- **Admin consent not granted.** `token-health` passes but Graph calls return 401/403 — permissions were added without re-granting consent. User: Azure portal → app registration → "Grant admin consent".
