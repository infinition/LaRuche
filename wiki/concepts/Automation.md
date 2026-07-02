# Automation

The automation hub gathers everything the hive does on its own: crons, missions, the
kanban, and [watchers](Watchers). The agent creates all of them through its own tools,
from a plain-language request, with your approval.

## Crons

Scheduled prompts. "Every morning at 9, check these three pages and summarize what
changed" becomes a cron whose payload is an agentic run. Each cron can target a
specific channel, so the morning digest lands on Telegram while a weekly report stays
in the web feed.

Telegram users can inspect and prune from the phone: `/crons`, `/delcron <name|all>`.

## Missions

Long-running goals that survive across days: "research topic X in depth, iterate daily,
keep your notes in memory." A mission holds its own state and channel, gets scheduled
work, accumulates findings in the cognitive map, and reports on its channel.

A finished mission stops occupying the scheduler; its cron disappears with it.

## Kanban

The board behind the missions: tasks move through columns, each task can carry its own
channel for reporting. Useful both as the agent's own work queue and as a shared
surface where you drop tasks for the hive.

## Watchers

Event-driven rather than time-driven; see [Watchers](Watchers) for the compiled rules
DSL.

## How they compose

The four primitives combine naturally:

- A **watcher** notices the release file appeared, and hands it to the agent.
- The agent run files a summary in **memory** and adds a follow-up task to the
  **kanban**.
- A **cron** picks up the board every morning and sends the digest to Telegram.

None of this requires you to write configuration. Describe the loop you want; the agent
assembles it from its own primitives and shows you what it built.
