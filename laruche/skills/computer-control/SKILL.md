---
type: skill
name: computer-control
description: Drive the machine itself, mouse, keyboard and screen, for desktop applications and native dialogs. Works without vision through the accessibility tree.
tools: [computer]
---

# Computer control

The `computer` tool drives the machine outside the browser: desktop applications, installers,
native dialogs, anything that is not a web page.

**For a web page, use `browser` instead.** It reads the DOM and clicks by element number,
so it is faster, cheaper in tokens and it does not miss.

## The fast path needs no screenshot

This is the one that matters, and the only one that works if you cannot see images.

1. `windows` lists every open window, with its size, position and application. The focused
   one is marked `*`.
2. `focus_window` `{ "window": "Notepad" }` brings one to the front, matching a piece of its
   title.
3. `read` returns a numbered map of that window's controls, plus the static text on it:

   ```
   ref_1 <document> Text editor = the current contents
   ref_7 <tab> notes.txt
   ref_13 <menuitem> File
   ref_21 <button> Minimise
   ```

4. `click`, `double_click`, `right_click` and `fill` take a `ref` instead of coordinates.
   `focus` gives a control focus without acting on it.
5. `read` again to see what changed.

Acting by ref goes through the OS accessibility API. It calls the control directly instead
of simulating someone aiming at it, so it is deterministic, it cannot land a pixel off, and
it works even when the window is not in front. The result says which pattern was used
(`invoke`, `toggle`, `select`, `setvalue`) or tells you it had to fall back to a real click.

**Refs come only from the latest `read`.** Every read renumbers them, and changing window
throws them away. Read again after acting, exactly as with the browser tool.

`read` uses UI Automation, so it is wired for Windows today. Elsewhere, use the pixel path
below; `windows` still works everywhere.

## The pixel path, for what the tree cannot see

Hand-drawn interfaces expose nothing: games, canvases, poorly tagged Electron apps. When
`read` comes back empty, fall back to looking.

1. `screens` lists the monitors: rank, id, size, position, scale, name.
2. `screenshot` captures one (`screen` picks it by id, rank or name; default is the primary)
   and returns it as an image, 1280px wide by default. **That capture sets the coordinate
   system.**
3. `mouse_move`, `left_click`, `left_click_drag`, `scroll` at `x`,`y` read off that image.

**Coordinates are always pixels of the last screenshot.** Never desktop pixels. The tool
converts, including on a display scaled to 150% and on a mixed setup where a 4K sits next
to a 1080p. Pointing before any screenshot is refused on purpose: without a capture there
is no shared coordinate system, and treating the numbers as desktop pixels would work on a
simple setup and fail silently everywhere else.

`max_width` raises the resolution when you genuinely cannot read what you need. It costs
tokens in proportion.

## Keys

`type` types into whatever has focus. `key` presses one key or a chord.

Named keys: `Enter`, `Tab`, `Escape`, `Backspace`, `Delete`, `Space`, `Home`, `End`,
`PageUp`, `PageDown`, `Up`, `Down`, `Left`, `Right`, `F1` to `F12`, or a single character.
Chord with `Control+c`, `Shift+Tab`, `Alt+F4`, `Meta+r`. `repeat` presses several times,
`hold_ms` holds each press.

`key_down` and `key_up` exist for what `key` cannot do: hold a key while other gestures
happen, which is what a game needs and what a drag with a modifier needs. If you press a key
down, release it. A key left down poisons every later interaction, the user's own included.

## What the human sees

The screen being driven gets an amber frame, a floating panel that names each action as it
happens, and a ring that follows the cursor and pulses on every click. The cursor glides to
its target instead of teleporting, text appears character by character, and a screenshot
triggers a brief flash once the capture is taken. It all fades a few seconds after LaRuche
stops acting. The panel can be dragged anywhere and stays where the user puts it.

`glow: false` removes the decoration, `animate: false` acts instantly on a long sequence,
`speed` above 1 slows the motion down for a demonstration. Acting by ref shows the panel
line without moving the mouse at all, because nothing needs to be aimed at.

## What it refuses, and why

Two refusals are by design, not bugs to work around:

- **LaRuche's own windows.** A click there would let you answer your own approval prompts,
  which would make every safeguard in the node decorative. The check is by process id, not
  by window title.
- **The human took the mouse back.** If the pointer moved well away from where the tool left
  it, the next gesture is refused and nothing is done. Say so and wait. Repeating the same
  call takes control back deliberately, so it is not a deadlock, but do not repeat it
  reflexively: someone is using the machine.

Approval comes in two classes. Approving one observing action (`screens`, `screenshot`,
`cursor_position`) does **not** approve a click, and the first acting call is asked
separately.

## When not to use it

- A web page: `browser`.
- Moving, renaming, converting or reading files: `shell_exec` or `file_*`. Driving a file
  manager by clicking is slower and fails in more ways.
- Anything a command line can do: run the command. Clicking through a settings dialog to
  read a value that `systeminfo` prints is wasted work and wasted tokens.

The right use is the case with no other door: a native application with no CLI, an
installer, a dialog the OS put on top of everything.
