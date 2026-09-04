# LaRuche 2048 App

This dependency-free reference App verifies the frontend extension path without rebuilding
LaRuche. It covers:

- installation from a `.laruche-app` ZIP archive;
- the sandboxed `LaRucheApp` SDK handshake;
- per-user private persistence;
- locale and theme context;
- keyboard, pointer and touch input;
- a detachable view;
- package and view icons.

## Build and test

From the repository root:

```powershell
node examples/apps/2048/test/game.test.js
python examples/apps/2048/build.py
```

The archive is written to `examples/apps/2048/dist/`. Open LaRuche, select **Apps**, choose
**Installer**, then enable 2048. The game state and best score survive navigation and node
restarts through `storage.private`.

The `package/` directory itself is the complete source package. It deliberately uses no CDN,
framework, network request or browser storage, so it remains compatible with the App sandbox
and its `connect-src 'none'` policy.
