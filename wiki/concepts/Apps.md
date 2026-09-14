# Apps

An App is an installable package that brings its own interface into LaRuche, and can
declare actions an agent is allowed to call. It is the larger of the two extension
formats; the smaller one is a [Forged Tool](Forged-Tools), which adds a single callable
command and no interface at all.

Apps are installed and updated without rebuilding LaRuche. The host supports HTML, CSS,
JavaScript and browser WebAssembly. See [Developing Apps](Developing-Apps) for the
workflow, [the App SDK](App-SDK) for what a page can call, and
[the App manifest](App-Manifest) for every field of `app.json`.

## The sandbox

An App page runs in an iframe with an opaque origin. It receives no cookies, no ambient
HTTP client, and no access to the host page. After the frame loads, the host transfers
one private `MessagePort` to that exact instance, and every capability call travels over
that channel. Closing the view closes the port, which is what makes a grant revocable
while the App is running rather than only at the next start.

The Content-Security-Policy the host sends with each page is the second half of the
isolation. It starts at `default-src 'none'` and names only the App's own versioned asset
directory plus the SDK file. `object-src`, `base-uri` and `form-action` are `'none'`, and
`frame-ancestors` is the LaRuche origin, so the page cannot be framed elsewhere. Scripts
get `'wasm-unsafe-eval'`, which is what makes browser WebAssembly work inside the sandbox.

An App therefore reaches nothing by default, LaRuche's own API included. Its only route
out is the bridge, and everything on that bridge is a declared permission.

**Do not add a `Content-Security-Policy` meta tag to your page.** The host already sends
one, written with absolute URLs. A page-authored policy can only narrow what the host
allows, and the usual way to write one narrows it to nothing: the frame is sandboxed
without `allow-same-origin`, so its origin is opaque, and in an opaque origin `'self'`
matches no URL at all. The two policies are enforced together, so `script-src 'self'`
blocks every script the App loads, including the SDK. What is left on screen is the
static HTML, with no styles and no bridge, and the agent is told the App did not connect.

## Permissions

Permissions are declared in `app.json`, as `required` or `optional`, and granted by the
user when the App is enabled. The consent screen greys out any name the node does not
actually enforce: a checkbox that grants nothing is worse than no checkbox, because the
user believes they decided something.

| Permission | Risk | What it allows |
|---|---|---|
| `storage.private` | low | Persistent key-value storage, isolated by App and by user |
| `ui.locale.read` | low | Read the current interface language |
| `ui.theme.read` | low | Read the current interface theme |
| `agents.invoke` | high | Ask an authorized agent to work, which spends model tokens |
| `network.fetch` | high | Reach only the hosts the package declares in `network.hosts` |
| `laruche.memory` | high | Read cognitive memory and propose facts to it |
| `laruche.files` | high | Read and write files inside the App's own folder |

Disabling an App clears its granted permissions rather than remembering them, so
re-enabling asks again.

`network.fetch` deserves its own note. The default sandbox reaches nothing, and that is
the point of it. The exception exists for one shape of App that the default makes
impossible: a notebook running real Python needs an interpreter and wheels, and carrying
them inside the archive hits the package size cap long before the scientific stack is
aboard. Hosts are pinned by exact name, with no wildcards, schemes, paths or ports, they
only enter the policy once the permission is actually granted, and they widen that App's
frame alone.

## What an App gives the agent

A manifest can declare **actions**: a name, a description, a JSON Schema for the input, a
view they belong to, and a read-only hint. Those actions are how an agent operates the
App, through the five `app_*` tools listed in [Tools](Tools). The sequence is discover,
read the guide, open a view, wait for it to be ready, then call one action.

Three things are separate permissions, and all three are checked on the server rather
than in the page:

- **discover**, whether the App appears at all,
- **open**, whether its views may be opened,
- **each individual action**, by name.

They are granted per App and per agent. An agent allowed to read a notebook is not
thereby allowed to write to it. When nothing has been set for a given agent, the rule for
the default principal applies, and failing that only discovery is allowed.

Text an App writes, its guide included, is documentation from an untrusted source. It is
never treated as an instruction and it cannot grant a permission.

## Views, panels and instances

An App with an interface declares one to twenty views. Each has an id, a title and an
entry file, and can say whether it appears in navigation, whether it may be detached,
whether several instances may run at once, and whether the host should wait for it to
report ready before treating it as usable.

A view opens in LaRuche's resizable side panel, which stays visible while the user
navigates elsewhere, or in an independent browser window. One side panel is shared with
Settings, so opening a settings tab there replaces the App panel.

`waitForReady` matters for anything with a runtime. The SDK handshake only proves the
transport is connected; it does not mean Pyodide, a dataset or a game engine has
finished loading. A browser host reports its instances once a second, and the node
considers a host present for ninety seconds after its last report, so a sleeping or
heavily throttled tab eventually disappears from the list an agent can see.

## Where an App lives, and how a version is chosen

Packages sit on disk under `packages/<id>/<version>/`, and the manifest's `id` and
`version` must match those two folder names. A package whose manifest disagrees with its
folder is rejected with a diagnostic rather than loaded quietly.

The registry file next to them records, per App, which version is active, whether it is
enabled, and which permissions were granted. At startup the node reads every version it
finds, sorts them by semantic version, and keeps the recorded active one if it is still
present, or the highest one otherwise. An App recorded as installed whose package has
disappeared stays listed, disabled, carrying the reason.

Installing a version makes it the active one and leaves it **disabled** with no
permissions. An update is therefore never silent: the user re-enables it and chooses its
permissions again, having seen what the new version asks for.

## Limits

| Limit | Value |
|---|---|
| Package archive | 32 MiB |
| Files in an archive | 4 096 |
| Manifest | 256 KiB |
| Views per App | 1 to 20 |
| Bridge message | 64 KiB |
| Action result | 60 KiB |
| Concurrent bridge requests | 8 |
| Bridge requests per minute | 120 |
| Private storage | 1 MiB per App and user, 256 keys, 64 KiB per value |
| App folder | 32 MiB, 2 000 entries, 2 MiB per file |

## The reference Apps

All four live under `apps-library/` and build into an installable `.laruche-app` archive
without compiling LaRuche.

| App | What it demonstrates |
|---|---|
| **2048** | The SDK bridge, private saved games, keyboard and touch input, detachable views, and an agent that can take one turn or play on its own |
| **Checkers** | Turn ownership enforced by the engine, so an agent playing out of turn is refused rather than trusted |
| **WASM Lab** | A browser WebAssembly module running inside the sandbox |
| **DS Studio** | A Python notebook an agent drives through more than twenty declared actions, with datasets, charts, kept results and a notebook library |

Read 2048 first to see the whole surface at its smallest, and DS Studio to see what a
serious action set looks like: an App whose state is worth more than its view, where the
agent adds a cell, runs it, reads the result, and what it produced outlives the
conversation.

## Compatibility

Packages made during the earlier prototype remain readable. The node accepts
`addon.json`, the `.laruche-addon` extension, the old `/api/addons` routes and the
`LaRucheAddon` SDK name as aliases of their current equivalents.
