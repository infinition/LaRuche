# LaRuche 2048 App

This dependency-free reference App verifies the frontend extension path without rebuilding
LaRuche. It covers:

- installation from a `.laruche-app` ZIP archive;
- the sandboxed `LaRucheApp` SDK handshake;
- per-user private persistence;
- locale and theme context;
- keyboard, pointer and touch input;
- a side panel and an independent popup window;
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
framework, network request or browser storage, so it remains compatible with the App sandbox.
The host restricts fetches to the App's own package assets.

Choose **Panneau** to keep playing alongside Chat or Settings. Choose **Fenêtre** for a
separate browser popup. The panel shares the same resize handle and mobile bottom-sheet layout
as detached Settings sections. The panel header arrow returns the App to its page.

Wait for **Synchronisé** before moving a game between views: a newly opened view restores the
last saved state. Independent windows do not synchronize live; avoid playing the same saved
game in two windows simultaneously. Closing the main LaRuche window closes its App popups.

See `wiki/guides/Developing-Apps.md` for authoring your own App and the current SDK limits.
