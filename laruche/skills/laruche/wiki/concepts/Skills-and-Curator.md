# Skills and the Curator

## Skills

A skill is a markdown file: instructions, examples, and conventions for a category of
task. Skills live on disk, are editable from the UI or any editor, and are selected
dynamically: the working-set assembly matches the user's intent against the skill
catalog and injects only the relevant ones, so the context stays lean.

Built-in skills cover things like watcher creation (`watcher-architecte`) and mission
planning; your own skills teach the agent your conventions: how you like reports
formatted, how your homelab is laid out, what "deploy" means in your world.

## The curator

The curator (curateur) is the background component that grows the skill library from
experience. After a conversation ends, it can review the transcript and ask: did the
agent figure out something reusable here? If yes, it drafts a skill, verifies it, and
proposes it.

### Verified, not vibed

A curator skill proposal is not a summary of the chat. The draft is checked for
concrete, reusable instructions, and proposals can be routed through the
[LaReine](LaReine) proposal queue so you review the full content before anything joins
the library. The agent proposes, you dispose.

### Polite by construction

The curator shares your local model with your live conversations, so it is engineered
to never get in the way:

- **Single-flight**: at most one curator pass at a time.
- **Cooldown**: a minimum delay between passes (default 10 minutes,
  `LARUCHE_CURATEUR_COOLDOWN_SECS`).
- **Yields to you**: if any chat or agent run is active, the curator waits. Your
  conversation never lags because the hive is studying its notes.

## Editing and versioning

Skills are plain markdown, so they diff, they version, and they travel. The
[Miel mesh](Architecture#background-jobs) can federate skills between your nodes with
provenance tags, opt-in.
