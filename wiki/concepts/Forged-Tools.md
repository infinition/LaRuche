# Forged Tools

A Forged Tool is an atomic command the agent can call like a built-in tool. It is a JSON
manifest and a command line, with no interface and no lifecycle of its own. It suits a
local script, a repeatable shell command, or a small adapter to a service that has no MCP
server.

It is the smaller of the two extension formats. The larger one is an [App](Apps), which
brings its own interface and can declare a set of actions. Keeping the names apart
prevents a command manifest from being mistaken for a full application.

## The folder

One folder per tool, under `forged_tools/`:

```text
forged_tools/
  weather/
    tool.json     the manifest: name, description, schema, command
    run.py        the body, if the command needs one
    lib/          anything else the body reads
```

The manifest and the body travel together, so deleting the tool removes both at once. A
JSON file dropped loose at the root of `forged_tools/` is **not** loaded; the node logs
it and names the folder it expected instead.

## The manifest

| Field | Required | Meaning |
|---|---|---|
| `name` | yes | The tool name the agent calls. It joins the registry under this name |
| `description` | yes | What it does, and when to use it. This is what the model reads to choose it |
| `parameters` | yes | A JSON Schema for the arguments, validated before the command runs |
| `command` | yes | The shell command to execute |
| `danger` | no | `safe`, `needs_approval` or `dangerous`. Defaults to `safe` |
| `timeout_secs` | no | Seconds before the command is killed. Defaults to 30 |

`danger` decides whether the call goes through the approval policy described in
[Security](Security). A tool that writes, sends or deletes should not be left at `safe`.

## Placeholders

Each declared argument is available in `command` as `{{argument}}`. Two rules keep it
predictable.

`{{forged_tool_dir}}` is filled in with the tool's own folder. Use it for every path
inside the tool, so it works regardless of where the node was started:
`python "{{forged_tool_dir}}/run.py" {{city}}`. It is provided, never declared: it does
not belong in `parameters`.

A long text argument is passed on standard input rather than through the shell, when its
name is one of `message`, `text`, `content`, `code` or `body` and the command does not
already mention it as a placeholder. That is what keeps a multi-line payload from being
mangled by quoting.

## Creating one

1. `forged_tool_list` first, and `tool_search` on the same keyword. Do not create a
   duplicate of something already registered.
2. `forged_tool_create` with `name`, `description` and `command`, plus `schema` for the
   arguments. It can write the body in the same call, through `script_path` and
   `script_content`. It loads the tool into the live registry itself, so there is no
   reload step afterwards.

   The argument is called `schema` on the tool and lands in `tool.json` as `parameters`.
   Both names are correct, in their own place.
3. `tool_search` on the new name to confirm it entered the registry, then `tool_call` it
   once with a real argument to confirm it actually runs.

Do not look for the new tool in your own tool list. It will not be there until the next
mission, and that is normal: the registry behind `tool_search` and `tool_call` is live,
while the list injected into a running conversation is not rebuilt mid-run.

`forged_tool_delete` removes one and reloads what remains by itself.

The layout this format had before it was renamed is still read: `plugins/<name>/plugin.json`,
in its own directory, loaded before the canonical one so that a name present in both
resolves to `forged_tools/`. The `{{plugin_dir}}` placeholder also still works.

`reload_forged_tools` rescans the whole directory. It is not needed after
`forged_tool_create` or `forged_tool_delete`, which reload on their own. It **is** needed
whenever a manifest reached the disk another way: edited by hand, copied in, or generated
by a script. Until it runs, the folder and the registry disagree, and the registry is
what gets called.

## Choosing between the two formats

Write a Forged Tool when the answer is a command and the result is its output. Write an
[App](Apps) when there is state to keep, a view to show, or several related operations
that only make sense together. An App can also declare actions an agent calls, so the
choice is not interface against agent access; it is one command against a surface.

MCP servers cover a third case: a capability that already exists as a server, which
LaRuche connects to rather than reimplements. See [MCP](MCP).
