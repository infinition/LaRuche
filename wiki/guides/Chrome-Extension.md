# Chrome Extension

The LaRuche extension lets `browser` control the Chrome instance the user already has
open, including its tabs and signed-in sessions. Without it, `browser` remains available
in `launch` mode with a separate persistent profile.

Chrome 136 no longer honors a remote debugging port on the default profile. The
extension is the supported path to the user's normal browsing session.

## Install

Two sources, one procedure. From a release, download `laruche-extension-chrome.zip` and
unzip it. From a clone, use `extension-chrome/` directly.

1. Open `chrome://extensions`.
2. Enable developer mode.
3. Choose **Load unpacked** and select the folder.
4. Confirm the extension id is `ahgfjacmpohglimmcfnlbeccdghpkboo`.
5. Open the LaRuche popup, set the node port if it is not 8419, then enable browser
   control.

There is no `.crx` on purpose. Since Chrome 33, a packaged extension dragged into
`chrome://extensions` is refused on Windows and macOS: only the Web Store installs. A
`.crx` in a release would be a file nobody can open, and shipping it would mean
publishing the signing key, with which anyone could forge an update for the extension.

The id is fixed by the `key` field in the manifest, and that matters here: the node only
accepts a WebSocket whose origin is that exact id. Without the committed key, every
reinstall would produce a new id and break the link.

The badge turns amber when the local WebSocket is connected. If the manifest key was
changed, set `LARUCHE_EXTENSION_ID` to the resulting extension id or restore the
committed public key.

## What remains visible

- Agent tabs are grouped in a yellow LaRuche tab group.
- The controlled page has an amber frame and status panel.
- Clicked or filled elements flash briefly.
- Chrome keeps its own debugging banner visible.
- The popup switch disconnects control immediately.

The page panel keeps its position, size, narration, history and unsent draft across
navigations. After 40 seconds without a command, the extension detaches the debugger
and returns a borrowed tab to its previous group.

## Capture and showcase recording

The popup can record the active tab, a selected window or a screen. Recording continues
through navigation in an offscreen document and stops when the browser session closes,
after 40 seconds without a command, or when the user clicks stop.

Current Chrome versions save MP4 when the encoder is available and fall back to WebM.
Tab capture starts only after a click in the popup. Window and screen capture always use
Chrome's own source picker.

## Memory capture

The popup can save the current page or a short note into LaRuche's cognitive memory.
This is an explicit user action. The extension does not scrape tabs in the background.

## Transport

The extension connects only to `127.0.0.1` on the configured node port and uses
`/ws/navigateur`. The node checks the extension id before accepting the bridge.

The narrow extension protocol exposes `navigate`, `eval`, `screenshot`, `glow`, `tap`,
`cdp`, `tab`, `tabs`, `open_tab`, `select`, `close` and `ping`. Higher-level operations
such as `read`, `find`, refs, overlays and form handling live in the node and are shared
with launch and attach modes.

## Security boundary

The extension requests `debugger`, tab, capture and download permissions. This is a
powerful local bridge. No analytics or third-party service is used, and cookie values
are not copied out of Chrome.

The remaining risk is local port replacement. The extension trusts the process
listening on its configured port. If LaRuche is stopped and another local process takes
that port, that process could receive the bridge. Disable browser control when the node
is not running.

The implementation is documented in `extension-chrome/README.md`. Node-side transport
lives in `laruche-essaim/src/pont_navigateur.rs` and
`laruche-node/src/ws_navigateur.rs`.
