# Writing a LaRuche skill

A skill is a PROCEDURE: how to accomplish a kind of task, step by step, with the exact
commands and the traps. It orchestrates tools; it is not a tool itself.

The reader is not you. Assume a small local model, 8B or so, that will not infer, will
not improvise, and will take any ambiguity as permission to guess. Every sentence here
exists because a model got it wrong.

---

## Frontmatter

Only four fields change what the agent does. Set them, and do not invent others.

```yaml
---
type: skill               # REQUIRED. Without it the disk sync ignores the file entirely.
name: watcher-design      # REQUIRED. Must equal the folder name, exactly.
description: Turn a monitoring wish into a deterministic watcher rule tree.
prerequisites:            # OPTIONAL. Checked against PATH when the skill is opened.
  commands: [openhue, jq] # A missing one is reported, with an order to install it.
---
```

`enabled: false` also works: the skill stays on disk and disappears from the catalog.

**Write the description on ONE line, never as a block scalar.** `description: >-` with the
text on the following line is legal YAML that one of the two parsers reading these files
rejects outright, taking the whole skill with it. One line, plain, as shown above. It is
also exactly what `skill_create` writes.

`name` is in English, lowercase, hyphenated, and describes the ACTION: `cognitive-memory`,
`local-git`, `watcher-design`. It is a handle the model matches against an intent, so a
French or abbreviated name costs recall for no benefit. Only the brand vocabulary stays
French, and only where it names a LaRuche concept: `lareine-charter`.

`skill_create` also accepts `tools` and `scripts`, two arrays, written into the
frontmatter as `tools: [a, b]` and `scripts: [scripts/x.py]`.

`tools:` is a recommendation, never a permission. A tool listed there is not granted, one
omitted is not denied, and the skill stays usable with neither. What it does buy you:

- In a mission or a cron task, the list is surfaced above the skill body as the tools to
  prefer.
- In chat, opening the skill with `skill_view` compares the list against the tools whose
  signatures were injected this turn. Any that are registered but absent are named in the
  volatile tier, so the model knows to reach them with `tool_call` instead of concluding
  they do not exist.

That second point is the reason to fill it: the turn's tool selection is computed from
the user's message, before any skill is opened, so a skill's own tools are often missing
from it. Name only tools that really exist. An unknown name is dropped in silence.

Nothing else is read at runtime. `version`, `author`, `license`, `platforms`, `tags`,
`allowed-tools`, `when_to_use`, `arguments` exist in some older files and in the Rust
struct; no code consumes them today. Do not fill fields that do nothing: it teaches
whoever reads the file that metadata here is decorative, and the next real field gets
ignored too.

Every skill here is written for LaRuche, from LaRuche's own tools and paths. Do not
import one from another agent's library: its tool names, its runtime paths and its
voice all resolve to nothing here, and the failure is unreadable. If another project
solved the same problem, read it, learn the facts, then write the procedure yourself
against the tools in this registry.

**Renaming a skill folder orphans its memory node.** The disk watcher syncs
`skills/<folder>/SKILL.md` into `capacities.skills.<folder>`, keyed on the folder name,
and it never deletes the node the old name wrote. Rename the folder and the old node
survives, still listed by `skill_list`, still injected in the catalog, forever pointing at
a file nobody maintains. Pick the name once. If a rename is genuinely required, delete the
old node with `memory_delete_node` on `capacities.skills.<old-folder>` in the same change,
then confirm with `skill_list`. This is why four shipped skills still carry underscores
(`cron_manager`, `weather_forecast`, `youtube_transcribe`, `deep_research_synthesis`):
correcting them costs a ghost, so they stay until a migration handles both halves.

---

## The description is the only thing most models will ever read

Every skill's description sits in the system prompt, all of them, on every single turn.
The body is fetched on demand with `skill_view(name)`. So the description does one job:
**make the model reach for this skill at the right moment, and not at the wrong one.**

### The catalog TRUNCATES it, and does not tell you

`resumer_description` in `laruche-essaim/src/contexte.rs` shortens every description
before it reaches the prompt. Three cuts, in this order:

1. everything from the first `" - "` (space, hyphen, space) is **dropped**;
2. otherwise everything from the first `". "` is dropped, so only sentence one survives;
3. whatever is left is cut at **80 characters**, on a word boundary, with an ellipsis.

So `Answer a factual question from the web, with sources, when a single search is not
enough.` reached the model as `Answer a factual question from the web, with sources, when
a single search is…`, which reads as a condition with no consequence. Nine of the shipped
descriptions were being amputated this way, one of them losing an entire capability.

**Write to a hard budget of 80 characters, in one sentence, and never put `" - "` in a
description.** Count them. If it does not fit, the description is describing the body
instead of the trigger.

### Rules, in order of how often they are broken

1. **Describe the TRIGGER, not the technology.** The model is matching an intent, not
   shopping for a library.
   - No: `ASCII art: banners, cowsay, boxes, image-to-ASCII, QR, weather.`
   - Yes: `Render text or an image as ASCII art for terminal-friendly output.`
2. **One sentence, 80 characters or fewer.** See above: past that it is cut mid-word.
3. **Start with a verb.** "Turn", "Read", "Compile", "Deploy". A noun phrase reads like
   a category label and matches nothing.
4. **No two descriptions may overlap.** If a reader cannot tell two skills apart from
   their descriptions alone, merge them. Four near-identical research skills made the
   model hesitate, then pick wrong.
5. **Name the artefact produced**, when there is one: a report, a diagram, a patch.

Check the whole set at once, from the repository root:

```bash
python scripts/check_skills.py
```

---

## The body

Structure, in this order:

```markdown
# <Title>

One paragraph: what this achieves and when to use it. No history, no marketing.

## Prerequisites

Exact install commands, per OS if they differ. Then the verification command and what
its success looks like. A skill that assumes its tool is installed will loop.

## Procedure

Numbered steps. Each step: one command, and how to check it worked.

## Traps

The things that cost someone an hour. Wrong flag, silent failure, misleading success.

## Failure modes

Symptom, cause, fix. Written so the model can act on it without asking the user.
```

Rules for the body:

- **Exact commands, ready to run.** Not "install the CLI" but the line to paste.
- **Verify after every step that changes something.** Created a file, check it exists.
  Installed a binary, run its version command. The agent must never assume.
- **Say what success looks like.** "Prints the bridge IP" beats "should work".
- **A failed command is a state to handle**, not a dead end. Say what to do next.
- **Never reference another agent's runtime.** No foreign home directory, no foreign
  environment variable, no foreign user-agent string. Those paths resolve to nothing on a
  LaRuche install, so the command fails for a reason no one reading the output can work
  out. LaRuche's own: `skills/<name>/` for bundled files, `plugins/<name>/` for a plugin
  and the scripts it runs, `LARUCHE_HOME` (defaulting to `~/.laruche`) for runtime state.
- **Do not name another agent, anywhere.** Not in the body, not in a comment, not in a
  template that ends up in a pull request, not in a user-agent header. `check_skills.py`
  fails the build on it.
- **Use LaRuche tool names**: `shell_exec`, `file_read`, `file_write`, `file_edit`,
  `web_fetch`, `web_deep_search`, `memory_write`.
- **English throughout**, including comments in bundled scripts. The brand vocabulary
  (butinage, essaim, abeille, éclaireuse, escale, carnet, vigie, cap, LaReine, miel)
  stays French, it is the identity.
- **No em dash anywhere.** Use a comma, a colon, or two sentences.

---

## Bundled scripts

`skills/<name>/scripts/` holds anything too long to inline. A script must exit non-zero
on failure, print what it did on success, and never depend on a path outside the skill
folder and the user's home.

Writing a script is expected, not exceptional. When no built-in tool does the job, the
agent writes one, tests it, and either bundles it here or registers it as a plugin so it
becomes callable by name. The `skill-forge` skill documents that loop; keep this file and
that one in agreement.

---

## Before you ship one

- The description alone tells a stranger when to use it.
- No other skill answers the same need.
- Every command has been run, on this machine.
- Every declared prerequisite is really required, and really named as on PATH.
- A model that follows the steps literally, without thinking, succeeds.
