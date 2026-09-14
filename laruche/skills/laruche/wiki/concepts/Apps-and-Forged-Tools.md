# Apps and Forged Tools

LaRuche has two extension formats with different jobs. Apps provide complete interface
experiences. Forged Tools add small callable capabilities to the agent. Keeping these names
separate prevents a command manifest from being mistaken for a full application.

## Apps

An App is an installable package with one or more views. It appears in the Apps section and
each view opens as a tab that can be detached when the manifest allows it. App pages run in an
isolated iframe and communicate with LaRuche through a narrow message bridge. They do not
receive the user's cookies or unrestricted access to the host page.

Views support both a LaRuche side panel (**Panneau**) and an independent popup (**Fenêtre**).
Browser WASM modules can be bundled with an App. See [Developing Apps](Developing-Apps) for
the build/install workflow, SDK examples, supported permissions and remaining limitations.

The canonical package extension is `.laruche-app`. It is a ZIP archive whose root contains
`app.json` and the files referenced by the manifest. An App identifier uses reverse-DNS form,
for example `dev.example.supervision`, and its version follows semantic versioning.

```text
supervision.laruche-app
  app.json
  icon.svg
  ui/
    index.html
    app.js
```

The current host supports these permissions:

| Permission | Risk | Purpose |
|---|---|---|
| `storage.private` | low | Persistent storage isolated by App and user |
| `ui.locale.read` | low | Read the current interface language |
| `ui.theme.read` | low | Read the current interface theme |
| `agents.invoke` | high | Ask an authorized agent to work, which spends model tokens |
| `network.fetch` | high | Reach only the hosts the package declares |
| `laruche.memory` | high | Read cognitive memory and propose facts to it |
| `laruche.files` | high | Read and write files inside the App's own folder |

Permissions must be declared in `app.json`, and the consent screen greys out any name the
node does not actually enforce. A checkbox that grants nothing is worse than no checkbox,
because the user believes they decided something.

The App SDK exposes private storage and controlled UI operations such as changing a tab
title, marking it dirty, requesting detachment and closing the view.

## What an App gives the agent

An App declares **actions** in its manifest, each with a name, a description, a JSON Schema
and a read-only hint. Those actions are how an agent operates the App, through the five
`app_*` tools described in [Tools](Tools): discover, read the guide, open a view, wait for
it to be ready, then call one action.

Three things are separate permissions, granted per App and per agent and checked on the
server rather than in the page: discovering the App, opening its views, and each individual
action. An agent that may read a notebook is not thereby allowed to write to it.

The manifest also reserves backend declarations for MCP stdio and WASI. Those remain
inactive: a manifest may describe them for forward compatibility, but no App supervises a
backend process today.

## The reference Apps

All four live under `examples/apps/` and build into an installable `.laruche-app` archive
without compiling LaRuche. Their READMEs carry the exact commands.

| App | What it demonstrates |
|---|---|
| **2048** | The whole surface at its smallest: isolated SDK bridge, private saved games, keyboard and touch input, detachable views, and an agent that can take one turn, play on `Auto`, or pause |
| **Checkers** | An 8x8 game with strict turn ownership, so an agent that plays out of turn is refused by the engine rather than trusted |
| **WASM Lab** | A browser WebAssembly module inside the App sandbox, computing `19 + 23 = 42` in a real runtime |
| **DS Studio** | The largest one: a Python notebook an agent drives through more than twenty declared actions, with datasets, charts, kept results and a notebook library |

DS Studio is the useful one to read when designing your own actions. It shows an App whose
state is worth more than its view: the agent adds a cell, runs it, reads the result, and
what it produced survives the conversation.

## Forged Tools

A Forged Tool is an atomic command that the agent can call like a built-in tool. It is useful
for a local script, a repeatable shell command or a small adapter that does not need its own
interface lifecycle.

```text
forged_tools/
  weather/
    tool.json
    run.py
```

`tool.json` declares the name, description, argument schema, danger level, timeout and shell
command. The `{{forged_tool_dir}}` placeholder resolves to the tool folder, so scripts do not
depend on the node's working directory. The tools `forged_tool_create`, `forged_tool_list`,
`forged_tool_delete` and `reload_forged_tools` manage this format.

## Choosing the right extension

| Need | Use |
|---|---|
| A dashboard, game, data lab or supervision screen | App |
| A detachable tab with private persistent state | App |
| One command callable by the model | Forged Tool |
| Tools or resources from an existing service | MCP server |
| A documented sequence using existing capabilities | Skill |

An App can eventually contribute Forged Tool-like capabilities through its supervised backend.
That convergence should happen behind the Apps permission and lifecycle model, not by deleting
the current tool loader before the backend host exists.

## Compatibility

Packages made during the earlier prototype remain readable. LaRuche accepts `addon.json`, the
`.laruche-addon` extension, old `/api/addons` routes and the `LaRucheAddon` SDK name as aliases.
New packages should use `app.json`, `.laruche-app`, `/api/apps` and `LaRucheApp`.

Likewise, existing `plugins/<name>/plugin.json` folders are moved to
`forged_tools/<name>/tool.json` at startup when no canonical folder conflicts. A conflicting
legacy folder stays in place and remains readable, and the old `{{plugin_dir}}` placeholder
still resolves. New tools are written only to the canonical layout. This is migration and read
compatibility, not two competing product names.
