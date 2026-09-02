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
channel for reporting, and its own profile and model. Useful both as the agent's own
work queue and as a shared surface where you drop tasks for the hive.

**Only the Ready column runs.** A task you write, or one the agent creates, lands in
Triage or Todo and stays there until someone promotes it. Nothing you type is executed
on the spot.

This is the single most surprising thing about the board, and it is deliberate. Without
that gate the dispatcher would take everything: the agent files five follow-ups while
working, and five agentic runs start a few seconds later, none of which anybody asked
for. Promotion to Ready is the moment a human says yes.

Once a task is in Ready, the dispatcher picks it up within the poll interval, five
seconds by default, adjustable in the Kanban tab. One task at a time: a board is a
queue, not a fan-out.

## Watchers

Event-driven rather than time-driven. See [Watchers](Watchers) for the rules, the four
observable types, and what a watcher can do when it fires.

### Cron or watcher?

This is the one choice worth getting right, because the costs are not comparable.

A cron wakes a full agentic run at every tick, whether anything changed or not. A
watcher evaluates a compiled rule tree, which costs nothing, and only involves a model
if the tree explicitly asks for one or if its action is `agent`.

So: **if you are asking "has X happened yet", it is a watcher, not a cron**, even when
what you observe needs a command to answer. Watching a lamp, a service or a disk
through a `command` watcher costs nothing per check; the same thing scheduled as a cron
costs a model turn every single time.

Reach for a cron when the work itself is the point and its timing is arbitrary: a
morning digest, a weekly report, a nightly cleanup. Reach for a watcher when a
condition decides.

## How they compose

The four primitives combine naturally:

- A **watcher** notices the release file appeared, and hands it to the agent.
- The agent run files a summary in **memory** and adds a follow-up task to the
  **kanban**.
- A **cron** picks up the board every morning and sends the digest to Telegram.

None of this requires you to write configuration. Describe the loop you want; the agent
assembles it from its own primitives and shows you what it built.
