---
type: skill
name: extend-toolset
description: Add a plugin tool, connect an MCP server, or chain tools in one script.
---

# Extending the toolset

Three different things get confused here. Pick the right one before writing anything.

| You want | Use |
|---|---|
| A capability that does not exist, backed by a command or script | a **plugin** |
| Capabilities from an existing external server | an **MCP server** |
| Several existing tools run back to back in one turn | **run_script** |
| A documented sequence a human or model should follow | a **skill**, see skill-forge |

## Finding a tool that already exists

Only part of the registry is listed in your prompt on a given turn. Before concluding a
capability is missing:

1. `tool_search` with a keyword. It searches ALL registered tools and returns name,
   origin and description.
2. `tool_call` with `tool` and `args` runs any of them, listed this turn or not.

Sensitive tools that require approval cannot be run through `tool_call`. Call those
directly by name; going through `tool_call` will be refused, and that refusal is the
expected behaviour, not a bug to work around.

## Chaining tools in one turn

`run_script` executes a SEQUENCE without asking the model again between steps. It is the
cheapest thing in this file: it removes a full round trip per step.

- `steps` is a list of `{tool, args}` objects.
- The output of step N is injectable into a later argument as `{{N}}`, 1-based.

```json
{"steps":[
  {"tool":"web_search","args":{"query":"rust-analyzer release notes"}},
  {"tool":"file_write","args":{"path":"C:\\tmp\\notes.md","content":"{{1}}"}}
]}
```

Use it whenever the second call's arguments are fully determined by the first result. Do
NOT use it when you need to read the intermediate output and decide: a step whose
arguments depend on your judgement belongs in a normal turn.

## What you create now is callable now, but not listed until later

Read this before you conclude that something you just created does not exist.

The tool list in your prompt, and the native tool set sent to the provider, are both
frozen when the mission starts. Freezing them is what keeps the cached prefix stable and
the cost down. So a plugin you create mid-mission will NOT appear in your list, no matter
how many times you reload.

The registry behind `tool_search` and `tool_call` is live. A plugin registered by
`reload_plugins` is immediately reachable through `tool_call`, in the same mission, on
the next turn.

| | This mission | Next mission |
|---|---|---|
| A plugin, after `reload_plugins` | callable via `tool_call`, absent from your list | listed normally |
| A skill, after `skill_create` | openable via `skill_view` by name | listed in the catalog |

So: **do not verify a new plugin by looking for it in your tool list. Verify it by
calling it.** `tool_search` on its name, then `tool_call` with real arguments. Absence
from the list proves nothing and is expected.

## Creating a plugin

A plugin is a folder. `plugin_create` writes `plugins/<name>/plugin.json`, and the script
it runs sits beside it in the same folder:

```
plugins/
  meteo/
    plugin.json     the manifest: name, description, schema, command
    run.py          the body the command runs
```

Manifest and body travel together, so `plugin_delete` removes both at once. Never put a
plugin's script anywhere else.

The layout inside the folder is flat: a plugin is a manifest and the thing it runs, so
there is no `scripts/` level to create, unlike a skill which also carries references and
templates. A plugin that genuinely grows several files may organise them in subfolders of
its own, `{{plugin_dir}}/lib/parse.py` and so on, and nothing has to be declared for that
to work. Do not add the level for a single file.

`plugin_create` requires `name`, `description` and `command`.

- `command` is a shell template with `{{slots}}`, for example
  `python "{{plugin_dir}}/run.py" {{ville}}`.
- `{{plugin_dir}}` is filled in with the plugin's own folder. Use it for every path
  inside the plugin, so the command works whatever directory the node was started from.
- `schema` is a JSON Schema for the arguments. Every slot in the command must appear as a
  property. A slot with no matching property is never filled and the command runs
  malformed. `{{plugin_dir}}` is the exception: it is provided, never declared.
- `script_path` plus `script_content` writes the backing script at the same time.
  `script_path` is a bare file name such as `run.py`, not a path: it always lands in the
  plugin's folder.

Procedure:

1. `plugin_list` first, and `tool_search` on the same keyword. Do not create a duplicate
   of something already registered.
2. `plugin_create` with all four parts.
3. **`reload_plugins`.** This is not optional. Until it runs, the plugin exists on disk
   and is registered nowhere.
4. `tool_search` on the new name to confirm it entered the registry, then `tool_call` it
   once with a real argument to confirm it actually runs. Do NOT look for it in your tool
   list: it will not be there until the next mission, and that is normal.

`plugin_delete` with `name` removes one. Run `reload_plugins` after that too.

## Connecting an MCP server

1. `mcp_list` to see what is already connected.
2. `mcp_add` with `name` and `command`, plus `args` if the server needs them.
3. `mcp_list` again to confirm it is up. A server that fails to start still appears in
   configuration, so verify rather than assume.
4. `list_mcp_resources` shows what it exposes: URI, name, MIME type, description.
5. `read_mcp_resource` with `server_name` and `uri` reads one. The URI must come from
   `list_mcp_resources`; do not guess URIs.

`mcp_remove` with `name` disconnects one.

## Traps

- **Forgetting `reload_plugins`.** Created, invisible, and the failure message says the
  tool does not exist, which sends you off creating it again.
- **A `{{slot}}` with no schema property.** The command runs with the placeholder left in
  it and fails in a way that looks like a script bug.
- **Guessing an MCP URI.** They are opaque and server-specific. Always list first.
- **Reaching for a plugin when a skill was needed.** If the capability already exists and
  what is missing is knowing HOW to use it, write a skill. A plugin wrapping tools you
  already have adds a moving part and no capability.
- **Relative paths in `command`.** They resolve against the server's working directory,
  not the user's folder. Use `{{plugin_dir}}` for anything inside the plugin.
- **A JSON dropped loose at the root of `plugins/`.** It is not loaded. The node logs the
  folder it should have gone into, and the tool simply never exists.

## Failure modes

**The plugin was created but does not appear in my tool list.** Expected. The list is
frozen for the mission. Confirm with `tool_search` and use it through `tool_call`. Do not
create it a second time.

**The plugin was created but `tool_call` says unknown tool.** That one is real:
`reload_plugins` was not run, or it ran before the write finished. Run it again, then
`tool_search` the name.

**The plugin runs but receives empty arguments.** Slot names in `command` and property
names in `schema` disagree. They must match character for character.

**`run_script` step 2 receives the literal text `{{1}}`.** The placeholder is inside a
nested structure the substitution does not reach, or the index is wrong: it is 1-based,
so the first step is `{{1}}`, not `{{0}}`.

**An MCP server appears in `mcp_list` but exposes no resources.** It failed to start.
Check the command actually runs in a shell first with `shell_exec`, then re-add it.
