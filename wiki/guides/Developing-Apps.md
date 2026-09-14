# Developing Apps

Apps can be developed and installed without rebuilding LaRuche. The current host supports
HTML, CSS, JavaScript and browser WebAssembly. Rust is needed only when changing the host
itself, not when adding or updating an App.

This page is the workflow: build, install, update, test. Two reference pages carry the
detail it does not repeat. [The App manifest](App-Manifest) covers every field of
`app.json` and what validation refuses. [The App SDK](App-SDK) covers every method a page
can call, with its errors and limits. [Apps](Apps) explains the sandbox, the permission
model and how a version is chosen.

## Try the reference Apps

From the repository root:

```powershell
node apps-library/2048/test/game.test.js
python apps-library/2048/build.py
python apps-library/checkers/build.py
python apps-library/wasm-demo/build.py
python apps-library/ds-studio/build.py
```

In LaRuche, open **Apps**, click **Installer**, select an archive from the example's `dist/`
directory and enable it. Open its view from the left rail.

| App | What to look at |
|---|---|
| **2048** | Keyboard and touch controls, private saved games, and an agent that can take one turn |
| **Checkers** | Turn ownership enforced by the engine, so an agent playing out of turn is refused |
| **WASM Lab** | `19 + 23 = 42` computed inside a real WebAssembly module |
| **DS Studio** | A Python notebook driven through more than twenty declared actions |

Start from 2048 to read the whole surface at its smallest, and from DS Studio to see what
a serious action set looks like.

**Panneau** opens the App in LaRuche's resizable side panel, which remains visible when you
navigate to Chat. On mobile it uses the same bottom sheet as detached Settings. **Fenêtre**
opens an independent browser popup. The panel's arrow returns the App to its full page.
One side panel is shared with Settings: opening Appearance there replaces the App panel.
Panels are not automatically restored after a browser reload.

## Create your own App

Start with `apps-library/2048/package/` or the smaller `apps-library/wasm-demo/package/`.
Keep your source in a separate directory with this layout:

```text
my-app/
  app.json
  ui/
    index.html
    app.js
    styles.css
    engine.wasm   (optional, supply your compiled module)
```

Change the manifest's `id`, `name`, `version`, view declarations and permissions. Use a unique
reverse-DNS identifier such as `dev.yourname.my-app`. The 2048 manifest is a complete working
example. All view entry paths start with `ui/`.

Include the host SDK before your own JavaScript:

```html
<script src="/apps-runtime/v1.js" defer></script>
<script src="./app.js" defer></script>
```

Use its ready promise before calling host capabilities:

```javascript
const context = await LaRucheApp.ready();
const saved = await LaRucheApp.storage.get('state.v1');
await LaRucheApp.storage.set('state.v1', { score: 42 });
await LaRucheApp.ui.setTitle('My App');
```

Storage needs the `storage.private` permission in the manifest. The optional context fields
`locale` and `theme` need `ui.locale.read` and `ui.theme.read`. The SDK also provides
`storage.delete`, `storage.list`, `ui.setDirty`, `ui.requestDetach` and `ui.close`.
`ui.requestDetach` requests a popup and may be blocked by browser popup rules; prefer the
native **Fenêtre** button for a direct user gesture.

Package your source without compiling the host:

```powershell
python apps-library/package_app.py C:/Dev/my-app --output C:/Dev/my-app-dist
```

Install the resulting `.laruche-app` through the UI. For an update, increment the manifest's
version, rebuild and install again. Existing versions cannot be overwritten through the
installer. Re-enable the version and choose its permissions. Storage uses the App identifier,
so give your saved data its own versioned keys and migrate it when its shape changes.

## Do not write your own Content-Security-Policy

The host sends one with every page, built from absolute URLs. Adding a meta tag of your
own cannot loosen it and will almost certainly break the App: an App frame is sandboxed
without `allow-same-origin`, so its origin is opaque, and `'self'` matches nothing in an
opaque origin. `default-src 'self'` therefore blocks every script and stylesheet the page
loads, the SDK included.

The symptom is recognisable. The page shows its static HTML, unstyled, and never gets
further; `app_open` fails with `App did not connect` because no script ever ran to answer
the bridge. If an App looks frozen on its own loading screen, check for a meta CSP first.

## WASM and isolation

Place browser-targeted `.wasm` files inside `ui/` and load them using
`WebAssembly.instantiateStreaming(fetch('./engine.wasm', {credentials:'omit'}), imports)`.
The server sends `application/wasm` and static-asset CORS headers. CSP permits WASM compilation
and fetches only from this package's versioned asset directory. It does not permit JavaScript
`eval`, calls to LaRuche's HTTP APIs or arbitrary external services.

The iframe has an opaque origin and cannot read LaRuche's DOM, cookies or localStorage.
WASM runs within that browser sandbox; it is not a replacement for the iframe or permissions.
All App-to-host operations still pass through the SDK. Bundle JS, CSS, fonts and libraries
locally. Framework builds must use relative asset paths and avoid eval-based development
bundles. Shared-memory WASM threads and native WASI processes are not supported yet.

## Connect an agent to an App

The authenticated web-chat agent receives five tools: `app_list`, `app_guide`, `app_open`,
`app_wait` and `app_call`. It should discover installed Apps before substituting an external
website, read the guide, open the view, wait for readiness and only then call an action.
An open LaRuche browser is required: actions execute in its sandboxed App iframe, not in Rust.
If several instances are open, select `instanceId` from discovery. Never automatically retry
a timed-out mutation: it might already have executed before its reply was lost.

Declare actions and their purpose in `app.json`:

```json
{
  "guide": "Read game.state, then choose a legal move using its current revision.",
  "actions": [
    {
      "name": "game.move",
      "description": "Play one legal move against the current board revision.",
      "viewId": "main",
      "readOnly": false,
      "inputSchema": {
        "type": "object",
        "properties": {
          "direction": {"type": "string", "enum": ["left", "right", "up", "down"]},
          "revision": {"type": "integer", "minimum": 0}
        },
        "required": ["direction", "revision"],
        "additionalProperties": false
      }
    }
  ]
}
```

This is a fragment to merge into a full manifest. Register the corresponding handler after
the SDK handshake:

```javascript
await LaRucheApp.ready();
LaRucheApp.actions.register('game.move', async ({direction, revision}) => {
  // Implement these functions in your App. The engine, not the LLM, owns the rules.
  validateRevisionAndMove(revision, direction);
  const next = applyMove(direction);
  await LaRucheApp.storage.set('game.v1', next);
  return next;
});
```

The host validates the declared schema and the user's current permission before dispatch.
`readOnly` is an author-supplied hint, not a guarantee or a permission bypass. Guides and
action output are untrusted task data and cannot grant access to anything.

The accepted JSON Schema subset is explicit: `type` (object, array, string, number, integer,
boolean, null), `properties`, `required`, boolean `additionalProperties`, `items`, `enum`,
`minimum`, `maximum`, `minLength`, `maxLength`, `minItems`, `maxItems`, `title`, `description`.
Schemas may nest eight levels. Unsupported keywords, including `$ref`, are rejected at
installation, not silently ignored. There are at most 64 actions; guides are at most 16 KiB.

## Permissions and consent

Installation and updates select the package **disabled** and display its requested rights.
The native **Permissions** button on each App opens three separate sections:

1. **App to LaRuche capabilities**: required and optional manifest capabilities. An administrator
   approves the package for the hive. Required capabilities cannot be removed individually;
   disable the App to revoke the whole package.
2. **Agent to App**: discovery, opening, and each declared action. Pick default LaRuche or a
   named agent. Each rule is Inherit, Allow or Deny. Initially discovery is allowed and all
   opens/actions are denied. An agent override takes precedence over the LaRuche default.
3. **App to agents**: select exactly which agents this App may invoke for your user account.
   This also requires the package capability `agents.invoke`. Both gates must allow it.

Per-user rules save immediately. Each new command checks them on the server; queued revoked
commands are cancelled, and model generation checks revocation every 500 ms. A completed
effect cannot be undone by revoking its permission. The audit log records operations and
agent/provider identity, not prompts, datasets or API keys.

An App cannot edit these rules, register users, access provider keys or choose another user.
The bridge binds its App and instance identity to the native host. Chat identity comes from
the authenticated cookie, not a user id proposed by a model or websocket query parameter.
App tool calls without an authenticated web-user context, including current external MCP
and unattended paths, fail closed. They do not impersonate a browser user.

## Reusable agents and separate contexts

Open **Apps > Agents** to create a named profile with an emoji/text avatar, personality,
instructions, provider/model, token limit and temperature. Provider choices come from the
configured profile catalogue. There is no silent fallback if a provider/model disappears.
Provider-specific parameter limitations still apply. This library is separate from existing
round-table specialists; no specialist configuration is migrated or overwritten.

An App declares `agents.invoke` as an optional or required capability, and uses:

```javascript
const agents = await LaRucheApp.agents.list(); // Only agents granted to this App/user.
const white = await LaRucheApp.agents.run(agents[0].id, 'match-42-white', 'Suggest a move.');
const black = await LaRucheApp.agents.run(agents[0].id, 'match-42-black', 'Suggest a move.');
// Different seats have different contexts, even when sharing one agent definition/model.
await LaRucheApp.agents.reset(agents[0].id, 'match-42-white');
```

`run` returns `{text, agentId, sessionId, model}` and exposes no host tools to the model.
For one validated action turn, use `agents.act(agentId, sessionId, stateAction, prompt)`.
It reads the declared state action, asks the model for one JSON action using the current
guide/schema/state, then invokes that action through the same permission-checked broker.
Malformed model output or an illegal/stale move fails visibly with no automatic retry.
The state action needs agent-to-App permission too. Author the App's own handlers defensively.

Contexts are keyed by user, App, agent and session id. They are bounded and held in memory,
not persisted across server restarts. Agent definitions and permissions are disk-persistent.
The reference **2048** has an agent selector, **Un tour**, **Auto** and **Pause**. Pause
lets the current turn finish but cancels the next scheduled turn. Grant opening, `game.state`
and `game.move`, approve `agents.invoke`, and select an allowed agent before using it.

## Readiness and long-running runtimes

Set `waitForReady: true` on a view that needs initialization. The SDK handshake only means
the transport is connected; it does not mean Pyodide, a dataset or a game engine has loaded.

```javascript
await LaRucheApp.ready();
await LaRucheApp.ui.setStatus('loading', 'Loading runtime', 10);
try {
  await initializeMyRuntime(); // Your App's initialization, not a built-in SDK method.
  registerMyActions();
  await LaRucheApp.ui.setStatus('ready', 'Ready', 100);
} catch (error) {
  await LaRucheApp.ui.setStatus('error', String(error.message));
}
```

`app_open` returns the instance and its loading state. `app_wait` waits up to 20 seconds and
returns readiness/progress; repeat it when still loading. Calls are rejected while not ready.
An action handler currently has a 15-second host deadline. Long cell calculations will need
an asynchronous job/status/cancel API rather than keeping an action open indefinitely.

## Persistence and a future Data Science App

`storage.private` is **not browser localStorage**. Values live in
`<LARUCHE_DATA_DIR>/apps/data/<user UUID>/<app ID>/storage.json`, with interrupted-replacement
recovery. They survive browser cache clearing, server restarts and package version upgrades,
as long as the data home, account and App id remain the same. Back up the data home. A new
account has a different isolated store. Concurrent windows currently use last-write-wins.

This is suitable for preferences and game saves, not large datasets: the present limits are
1 MiB per App/user, 64 KiB per value and 256 keys. Do not encode images into JSON to evade
these limits. Apps deliberately have an opaque iframe origin, so direct browser storage is
not the supported persistence contract.

For the proposed Obsidian Data Science Studio port, Pyodide would run Python in browser WASM
without a system Python installation. It is still Python, not a JavaScript rewrite. The
notebook can display cell edits and results while calculations run in a Worker. The current
host does not yet provide that notebook or the complete Pyodide runtime integration.

Required next work: a permissioned persistent binary file/artifact store with quotas and
usage indicators, export/import and backup; mediated dataset downloads with source/size
checks; a bounded Worker policy and offline runtime packaging; cell jobs with progress,
cancellation and durable notebook revisions. Persist notebooks, data and outputs, but rebuild
the Python heap on startup and report Ready only after the runtime and libraries are loaded.
Do not claim arbitrary Python package, GPU or native-extension support from browser WASM.

## SDK versus MCP

The current implementation is a local SDK/action protocol with schema-described operations
and an authored guide. It is **not** an implementation of the MCP Apps extension. A future
authenticated MCP adapter can expose the same operations without changing App logic, but
must preserve identity, consent, permission enforcement and instance selection. Documentation
is not an authorization mechanism, in either protocol.

## Current limits and tests

- Private storage: 1 MiB per App/user; bridge messages: 64 KiB; 8 concurrent requests and
  120 requests per minute. Package upload: 32 MiB.
- Wait for pending saves before closing or moving a view. Opening a new view reloads it;
  in-memory state is not transferred. Independent windows have no live state synchronization.
- Browser hosts report their instances once a second, and the node considers a host
  present for ninety seconds after its last report. A sleeping or heavily throttled tab
  eventually disappears from what an agent can see. An App action has fifteen seconds to
  answer before the host gives up on it. Background or server-only App execution is not
  implemented.
- Agent calls: four simultaneous model requests per node, 30 requests/minute per user,
  120-second model timeout, 256 contexts, 16 history messages/48 KiB per context. Maximum
  model response: 48 KiB. No budget accounting or automatic LLM repair loop is provided yet.
- Event subscriptions and supervised native/WASI backends are not implemented. Browser WASM
  remains distinct from Wasmtime/WASI.

Run `cargo test -p laruche-node apps::` from the Rust workspace and
`node apps-library/test/agent-bridge.test.cjs` from the repository root (requires Playwright,
a Chromium installation and a built debug node). The browser test starts an isolated data
home and controlled local provider. It tests the real SDK, consent UI, agent library, action
dispatch, separate contexts/users and live revocation, without using a real model/account.
`CHROME_PATH` and `LARUCHE_TEST_BINARY` may override the browser and node executable.
