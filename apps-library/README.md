# Apps library

The Apps that ship with LaRuche, as source. Each one builds into an installable
`.laruche-app` archive without compiling LaRuche.

They are reference implementations rather than throwaway samples: they are versioned,
they are what the integration tests drive, and they are meant to be installed and used.
Read them when writing your own.

| App | Version | What it shows |
|---|---|---|
| [2048](2048) | 1.3.0 | The whole surface at its smallest: the SDK bridge, private saved games, keyboard and touch input, detachable views, and an agent that can take one turn or play on its own |
| [Checkers](checkers) | 1.2.1 | Turn ownership enforced by the engine, so an agent playing out of turn is refused rather than trusted |
| [WASM Lab](wasm-demo) | 1.0.0 | A browser WebAssembly module running inside the sandbox |
| [DS Studio](ds-studio) | 1.7.7 | A Python notebook an agent drives through twenty-seven declared actions, with datasets, charts, kept results and a notebook library |

## Building them

From the repository root:

```bash
python apps-library/2048/build.py
python apps-library/checkers/build.py
python apps-library/wasm-demo/build.py
python apps-library/ds-studio/build.py
```

Each writes its archive into its own `dist/`, which is not tracked by git. In LaRuche,
open **Apps**, click **Installer**, pick the archive, then enable it and choose its
permissions. An App always arrives disabled: an update is never silent.

An archive is a snapshot, and an edit under `package/` does not reach one already built.
`check_dist.py` compares them, so a correction cannot be handed over without being in
what gets installed:

```bash
python apps-library/check_dist.py
```

Bump the manifest version before rebuilding when the previous one is already installed:
the node refuses to replace a version it already holds.

`package_app.py` does the same for a directory of your own:

```bash
python apps-library/package_app.py path/to/my-app --output path/to/dist
```

## Testing them

The engine tests need nothing but Node:

```bash
node apps-library/2048/test/game.test.js
node apps-library/checkers/test/game.test.js
node apps-library/ds-studio/test/engine.test.js
```

The tests under [test](test) drive a real node with a real browser, so they also need
Playwright, Chromium and a built LaRuche node. Their README says how.

## Where to read next

[Apps](../wiki/concepts/Apps.md) covers the sandbox, the permissions and how a version is
chosen. [The App manifest](../wiki/reference/App-Manifest.md) covers every field of
`app.json`. [The App SDK](../wiki/reference/App-SDK.md) covers every method a page can
call. [Developing Apps](../wiki/guides/Developing-Apps.md) is the workflow.
