# Developing Apps

Apps can be developed and installed without rebuilding LaRuche. The current host supports
HTML, CSS, JavaScript and browser WebAssembly. Rust is needed only when changing the host
itself, not when adding or updating an App.

## Try the reference Apps

From the repository root:

```powershell
node examples/apps/2048/test/game.test.js
python examples/apps/2048/build.py
python examples/apps/wasm-demo/build.py
```

In LaRuche, open **Apps**, click **Installer**, select an archive from the example's `dist/`
directory and enable it. Open its view from the left rail. **2048** supports keyboard and
touch controls and private saved games. **WASM Lab** computes `19 + 23 = 42` inside a real
WebAssembly module.

**Panneau** opens the App in LaRuche's resizable side panel, which remains visible when you
navigate to Chat. On mobile it uses the same bottom sheet as detached Settings. **Fenêtre**
opens an independent browser popup. The panel's arrow returns the App to its full page.
One side panel is shared with Settings: opening Appearance there replaces the App panel.
Panels are not automatically restored after a browser reload.

## Create your own App

Start with `examples/apps/2048/package/` or the smaller `examples/apps/wasm-demo/package/`.
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
python examples/apps/package_app.py C:/Dev/my-app --output C:/Dev/my-app-dist
```

Install the resulting `.laruche-app` through the UI. For an update, increment the manifest's
version, rebuild and install again. Existing versions cannot be overwritten through the
installer. Re-enable the version and choose its permissions. Storage uses the App identifier,
so give your saved data its own versioned keys and migrate it when its shape changes.

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

## Current limits

- Private storage: 1 MiB per App/user; bridge messages: 64 KiB; 8 concurrent requests and
  120 requests per minute. Package upload: 32 MiB.
- Wait for pending saves before closing or moving a view. Opening a new view reloads it;
  in-memory state is not transferred. Independent windows have no live state synchronization.
- Permissions are selected when enabling an App; changing them currently requires disabling
  and re-enabling it. A dedicated permissions editor remains to be built.
- Agent tool contributions, LLM calls, event subscriptions, supervised native/WASI backends
  and GPU access through such backends are not implemented. Games work for human players;
  autonomous play by a LaRuche model is a separate next step.

The next host milestone is an authenticated, permission-checked action bridge so an App can
expose operations such as `game.state` and `game.move` to the agent. The game engine must
validate every proposed move regardless of the model's response.
