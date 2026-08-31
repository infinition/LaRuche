# Computer and Browser

LaRuche can act on the desktop, the user's Chrome session and visual inputs. These are
separate capabilities with separate boundaries.

## Native computer control

The `computer` tool is implemented in Rust. It can:

- capture one monitor, every monitor, or a named window;
- inspect the Windows UI Automation accessibility tree and address elements by ref;
- move, resize, minimize, maximize, restore and close windows;
- click, double-click, right-click, drag, scroll and hold mouse buttons;
- type text, press shortcuts and read or write the text clipboard;
- find an element, wait for a state and release any held input.

A visible halo marks automated actions. LaRuche reports when Windows elevation blocks
input instead of pretending the action succeeded. `Ctrl+Alt+Shift+H` is checked during
input and stops control immediately. Held buttons and keys are also released
automatically after a timeout.

The tool is included in default desktop builds. Headless builds can omit GUI control
with `--no-default-features`.

## Chrome extension

The extension in `extension-chrome/` connects the node to Chrome over a local WebSocket.
This uses the browser and sessions the user already has open instead of launching a
temporary profile.

The `browser` tool can read the page, including same-origin frames and shadow DOM,
identify overlays, click with real mouse events, fill forms, upload and download files,
manage tabs, inspect cookie names and sizes, answer JavaScript dialogs and resize the
viewport with touch emulation.

Consent banners are identified but never accepted on the user's behalf. Cookie values
are not returned to the model.

The extension can also save the current page or a note into cognitive memory and record
a browser showcase when explicitly started by the user.

## Images and camera

Chat accepts images through the file picker, paste and drag-and-drop. Screenshots
returned by tools and frames captured by the `camera` tool follow the same multimodal
path to a vision-capable model.

PNG and JPEG inputs are resized before they exceed provider request limits. Recent
images remain available on later turns so a follow-up question can still refer to what
was shown.

Camera support is compiled by default and can be omitted with `--no-default-features`
for a smaller headless build.

## Approval and safety

Reading page or screen state and acting on it do not share one blanket approval. The
registry evaluates the actual tool and operation. The global tool-permission mode,
disabled-tool list and approval prompts apply to computer and browser actions exactly
as they do to shell or file writes.
