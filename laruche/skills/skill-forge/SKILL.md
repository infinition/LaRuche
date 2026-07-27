---
type: skill
name: skill-forge
description: Write a skill, or forge a script tool, so a solved problem survives.
---

# Skill forge

A skill is a written procedure the agent can reopen later. When you solve something the
hard way, the fix is worthless unless it becomes a skill. This is how you write one from
inside a session.

Read `skills/AUTHORING.md` before creating one. It is the canon: what the frontmatter
does, why the description matters more than the body, and what a good body contains.

## Reading

- `skill_list` returns every installed skill with its description. Call it before
  creating anything, to check the procedure does not already exist.
- `skill_view` with `name` returns the full body. The catalog in your prompt only carries
  descriptions; the body is fetched on demand, so open the skill before following it.
  Do not act from the description alone.
- `skill_view` also reports missing prerequisites. If it says a command is absent,
  installing it is your job, not a question for the user.

## Creating

`skill_create` requires `name`, `description` and `body`. It also accepts `tools` and
`scripts`, two optional arrays written into the frontmatter.

`tools` grants nothing and forbids nothing. It earns its place elsewhere: the tools whose
signatures reach you on a given turn are picked from the user's message, before any skill
is opened, so a skill's own tools are frequently absent. When the skill is opened, every
listed tool that is registered but missing from the turn is named back to you, with the
instruction to reach it through `tool_call`. Fill it with real tool names, and only real
ones: anything the registry does not know is dropped without a word.

1. `skill_list` first. If something close exists, `skill_patch` it instead. Two
   overlapping skills make the model hesitate and pick the wrong one.
2. Choose `name`: lowercase, hyphens, no spaces. It becomes the folder name and the
   handle used by `skill_view`.
3. Write `description`: one sentence, **80 characters or fewer**, starting with a verb,
   naming the TRIGGER rather than the technology. The catalog drops everything after a
   ` - `, then everything after the first `. `, then cuts at 80 characters. Go over and
   the model reads an amputated sentence. This single line is injected into the system
   prompt on every turn, for every skill, and is the whole reason it will ever be opened.
4. Write `body` as markdown following AUTHORING.md: what it achieves, prerequisites with
   exact install commands, a numbered procedure where each step says how to verify it
   worked, traps, failure modes.
5. Call `skill_create`, then `skill_view` on the new name to confirm it round-tripped.

A skill you create is stored immediately and you can open it with `skill_view` by name
right away. It appears in the SKILL CATALOG of your prompt only from the next mission on,
because that catalog is assembled once at mission start to keep the cached prefix stable.
Its absence from the list is not a failure and must not trigger a second `skill_create`.

## Editing

`skill_patch` takes `name`, `old` and `new`, and replaces an exact string.

- `old` must match the file byte for byte, including indentation. Run `skill_view` first
  and copy from its output rather than from memory.
- If `old` appears more than once, the patch is ambiguous. Include surrounding lines
  until the fragment is unique.
- A failed patch changes nothing. Re-read and retry with a longer fragment; do not
  fall back to recreating the skill, that loses the rest of the file.

For a rewrite too large to express as a patch, `skill_file_write` with `skill` set to the
skill name and `path` set to `SKILL.md` replaces the whole file.

## Bundled files

A skill can carry scripts and data next to `SKILL.md`.

- `skill_file_list` with `skill` shows what it carries.
- `skill_file_read` with `skill` and `path` reads one.
- `skill_file_write` with `skill`, `path` and `content` creates or replaces one.
- `skill_file_delete` with `skill` and `path` removes one.

`path` is relative to the skill folder. Scripts belong under `scripts/`.

## Forging your own tools

You are not limited to the built-in tools. When no tool does what you need, WRITE ONE.
Python, Node, a shell script: anything the machine can run. This is normal work, not a
last resort, and it does not require asking the user.

Three levels, in increasing cost. Take the cheapest that works.

**1. A one-off script.** Write it to a temp path and run it with `shell_exec`. For
something you will do once, stop here. Do not bundle or register it.

**2. A script bundled in a skill.** Use `skill_file_write` with `path` under `scripts/`
when the script IS the procedure: too long to inline, and only meaningful inside this
skill. It lands in `skills/<name>/scripts/`, and the skill body then says which script to
run and with which arguments. It stays inert until the skill is loaded, so it costs
nothing on turns where the skill is not in play.

**3. A registered tool.** When the capability is useful OUTSIDE this skill and you want
to call it by name like any built-in, register it as a plugin with `plugin_create`, then
`reload_plugins`. It lands in its own folder, `plugins/<name>/`, with the manifest and
the script side by side, and it is callable from the moment it exists, with no skill
loaded. Full procedure in the extend-toolset skill. This is how a script you wrote today
becomes a tool available on every future turn.

The test between 2 and 3: must it work without anyone knowing the skill exists? Then it
is a plugin. Does it only make sense inside the procedure you are writing? Then it is a
bundled script.

Rules for anything you write, at every level:

- **Exit non-zero on failure.** A script that prints an error and exits 0 reports success
  to the caller, and the agent then builds on a result that does not exist. This is the
  single most damaging mistake in a self-written tool.
- **Print what you did on success**, on stdout, in a form the next step can read. Errors
  go to stderr.
- **Check dependencies before assuming them.** `python --version`, `node --version`, and
  the import you need. If a package is missing, install it: that is your job, not a
  reason to stop. Record the install command in the skill's Prerequisites section so the
  next run does not rediscover it.
- **Take inputs as arguments, never hardcode a path.** A script with `C:\Users\me\...`
  baked in works exactly once, on one machine.
- **Absolute paths inside the script.** Relative ones resolve against the server's
  working directory, not the user's folder.
- **Run it before you rely on it.** Write, run once with real input, read the output.
  Only then register it or reference it from a skill body. Registering an untested script
  hides the failure behind a tool name.
- **Stay inside the skill folder and the user's home.** A tool that writes elsewhere is a
  surprise nobody consented to.

When a script has proved itself, write the skill around it: the install command, the
invocation, what its output looks like, and what to do when it fails. That is the loop.
Solve it once, script it, register it, document it, and the next occurrence is one call.

## Deleting

`skill_delete` with `name`. Use it for a skill that is wrong or fully superseded, not for
one that is merely unused. Confirm with `skill_list` afterwards: the entry must be gone
from the catalog, not just from disk.

## Traps

- **The description is not a category label.** `Git: commits, branches, tags` matches
  nothing. `Commit staged work with a message that explains why` matches an intent.
- **Do not fill decorative frontmatter.** Only `type`, `name`, `description`,
  `prerequisites.commands` and `enabled` are read. `version`, `author`, `license`,
  `platforms`, `tags` are ignored by the runtime; adding them teaches the next reader
  that metadata here is fiction.
- **`name` must equal the folder name exactly.** A mismatch makes the skill invisible to
  `skill_view` while still appearing in `skill_list`.
- **Never reference another agent's runtime** in a body. Paths from other agents resolve
  to directories that do not exist here, and the failure is unreadable.
- **A skill is a procedure, not a tool.** If what you want is a new capability rather
  than a documented sequence, you want a plugin. See the `extend-toolset` skill.
- **Never write `description: >-`.** The folded block scalar is legal YAML and the
  dashboard's parser used to reject the whole file over it. Put the description on one
  line, which is also what `skill_create` writes.

## Failure modes

**`skill_view` returns nothing for a skill that `skill_list` shows.** The `name` in the
frontmatter disagrees with the folder. Fix the frontmatter with `skill_file_write`.

**A newly created skill never gets picked.** The description does not describe a trigger,
or it overlaps another skill. Rewrite the description; the body is not the problem.

**`skill_patch` keeps failing on an exact-looking string.** Invisible difference: line
endings, a non-breaking space, or trailing whitespace. Copy the fragment from
`skill_view` output verbatim, or replace the whole file with `skill_file_write`.
