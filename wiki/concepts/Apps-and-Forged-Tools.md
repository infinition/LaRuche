# Apps and Forged Tools

LaRuche has two extension formats with different jobs. Apps provide complete interface
experiences. Forged Tools add small callable capabilities to the agent. Keeping these names
separate prevents a command manifest from being mistaken for a full application.

## Apps

An App is an installable package with one or more views. It appears in the Apps section and
each view opens as a tab that can be detached when the manifest allows it. App pages run in an
isolated iframe and communicate with LaRuche through a narrow message bridge. They do not
receive the user's cookies or unrestricted access to the host page.

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

| Permission | Purpose |
|---|---|
| `storage.private` | Persistent storage isolated by App and user |
| `ui.locale.read` | Read the current interface language |
| `ui.theme.read` | Read the current interface theme |

Permissions must be declared in `app.json` and are granted when the App is enabled. The App
SDK exposes private storage and controlled UI operations such as changing a tab title,
marking it dirty, requesting detachment and closing the view.

The manifest already reserves backend and contribution declarations for MCP stdio, WASI,
tools, events and jobs. Backend supervision and agent-facing contributions are not active in
the current implementation. A manifest may describe them for forward compatibility, but an
App cannot yet expose a new agent tool through that path.

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
