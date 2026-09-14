# The App manifest

`app.json` sits at the root of an [App](Apps) package and describes everything the host
needs to know before running any of its code. It is validated on load, and a package
whose manifest does not pass is rejected with a reason rather than half-loaded.

Unknown fields are refused, not ignored. A typo in a field name is an error at install
time instead of a setting that silently does nothing.

The manifest is read from `app.json`, falling back to `addon.json` for packages written
before the format was renamed. It may not exceed 256 KiB.

## Top level

| Field | Required | Contents |
|---|---|---|
| `apiVersion` | yes | Must be `1` |
| `id` | yes | Lowercase reverse-DNS identifier, 3 to 128 characters |
| `name` | yes | Display name, 1 to 80 characters |
| `version` | yes | Semantic version |
| `description` | yes | One sentence, 1 to 500 characters |
| `publisher` | yes | Who publishes it, see below |
| `icon` | no | A path inside the package |
| `homepage` | no | A URL |
| `license` | no | A licence identifier |
| `compatibility` | no | Which LaRuche and which platforms |
| `ui` | see below | The views |
| `backend` | see below | A supervised process, not yet active |
| `permissions` | no | What the App asks for |
| `network` | no | The hosts its sandbox may reach |
| `contributes` | no | Reserved declarations |
| `guide` | no | Documentation for agents, up to 24 KiB |
| `actions` | no | What an agent may call, up to 64 |

An App needs at least a `ui` or a `backend`. One that declares neither is refused,
because it would have nothing to run.

`id` may hold only lowercase letters, digits, dots and hyphens, must contain at least one
dot, cannot start or end with a dot or hyphen, and cannot carry two separators in a row.
`dev.yourname.my-app` is the shape to follow.

`id` and `version` must also match the two folder names the package is installed under.
A manifest that disagrees with its folder is rejected, which is what keeps a version from
claiming to be another.

## `publisher`

| Field | Required | Contents |
|---|---|---|
| `name` | yes | 1 to 100 characters |
| `url` | no | A URL |
| `keyId` | no | A signing key identifier |

## `compatibility`

| Field | Required | Contents |
|---|---|---|
| `laruche` | no | A version requirement for the host |
| `platforms` | no | The platforms the App supports |

## `ui.views`

One to twenty views. Each is a page of the App.

| Field | Required | Contents |
|---|---|---|
| `id` | yes | Unique within the App, up to 80 characters |
| `title` | yes | What the tab shows |
| `entry` | yes | The HTML file, which must live under `ui/` |
| `icon` | no | A path inside the package |
| `navigation` | no | Whether it appears in navigation. Defaults to `true` |
| `detachable` | no | Whether it may be detached into its own window. Defaults to `true` |
| `multiInstance` | no | Whether several instances may run at once. Defaults to `false` |
| `waitForReady` | no | Whether the host waits for the App to report ready. Defaults to `false` |
| `minSize` | no | `width` and `height`, each between 240 and 4096 |

`entry` is required to live under `ui/` so that everything the browser loads sits in one
directory, which is the directory the Content-Security-Policy names.

Set `waitForReady` on any view with a runtime to load. The bridge handshake proves the
transport is connected, not that Pyodide, a dataset or a game engine is usable. Report
progress with `ui.setStatus` from [the SDK](App-SDK) while it loads.

## `permissions`

```json
"permissions": { "required": ["storage.private"], "optional": ["laruche.memory"] }
```

Names come from the catalogue in [Apps](Apps), and the node checks three things when the
user enables the App. Every required permission must exist on this node. Every permission
being granted must have been declared here, so nothing can be granted that the consent
screen never described. And every required permission must actually be granted, so
`required` means what it says: refusing one means the App does not run.

Optional permissions may be refused, and the App is expected to keep working without
them. Read `grantedCapabilities` from the SDK context rather than assuming.

Ask for the least that works. A permission listed as required is one the user cannot
decline while still using the App.

## `network`

```json
"network": { "hosts": ["cdn.jsdelivr.net"] }
```

Widens the sandbox to reach those hosts over HTTPS only, and only once `network.fetch`
has actually been granted. Each host is written in full: no wildcards, no schemes, no
paths, no ports. A malformed entry is dropped rather than widening the policy, and the
exception applies to this App's frame alone.

Vendoring whatever the App needs inside the package is the safer answer whenever it fits.

## `actions`

What an agent may call. At most 64.

| Field | Required | Contents |
|---|---|---|
| `name` | yes | Letters, digits, dot, underscore or hyphen, up to 80 characters |
| `description` | yes | What it does, 1 to 1000 characters |
| `viewId` | yes | The view that handles it, which must exist |
| `inputSchema` | yes | A JSON Schema for the arguments |
| `readOnly` | no | Whether it only reads. Defaults to `false` |

`open` and `discover` are reserved: they name the two permissions that gate the App
itself, so an action cannot be called either.

Names must be unique, and the schema is validated when the manifest loads rather than at
the first call. Every declared action needs a handler registered through
`LaRucheApp.actions.register`, or calling it fails.

`readOnly` is a hint shown to the agent alongside the action. The permission that
actually decides is the one granted for that action name.

## `guide`

Free text an agent reads through `app_guide` before using the App. Up to 24 KiB.

It is documentation from an untrusted source. It is never treated as a system
instruction, and it cannot grant a permission. Use it to say what the App is for, what
its actions expect, and what order they go in.

## `backend` and `contributes`

Both parse and are validated. Neither is active yet.

`backend` describes a supervised process: `type` as `mcp-stdio` or `wasi`, a `command`,
up to 100 `args` of 1000 characters each, `healthTimeoutMs` between 1000 and 120000,
`shutdownTimeoutMs` between 100 and 30000, and `restart` as `never` or `on-failure`.

`contributes` reserves `tools`, `events` and `jobs`.

A manifest may declare them for forward compatibility, but no App supervises a backend
process today. What an App exposes to an agent goes through `actions`.
