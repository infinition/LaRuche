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
   one is marked `*`. Minimised windows are not listed.
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
(`invoke`, `toggle`, `select`, `expand`, `setvalue`, `rangevalue`, `default action`) or
tells you it had to fall back to a real click.

**Refs come only from the latest `read` or `find`.** Every read renumbers them, and changing
window throws them away. Read again after acting, exactly as with the browser tool.

`read` uses UI Automation, so it is wired for Windows today. Elsewhere, use the pixel path
below; `windows` still works everywhere.

### When the window is too big to read

`read` stops at 300 controls and 120 lines of static text, and says so when it truncates. On
a large Electron app or an office suite you will hit that, and you will pay for it every
time.

`find` `{ "text": "Save" }` runs the same read and returns only the controls whose label
matches. Use it whenever you already know what you are looking for. It renumbers refs like
any other read, and the numbers it hands back are usable immediately.

### What a ref means depends on the action

Every one of these acts on the element, not on a guess:

| Action with a `ref` | What happens |
|---|---|
| `left_click` | its automation pattern, or a real click if it exposes none |
| `right_click`, `double_click`, `triple_click`, `middle_click` | a real click at its centre |
| `mouse_move` (or `hover`) | the pointer moves onto it and **nothing is clicked** |
| `scroll` | it is scrolled into view through its own pattern |
| `left_click_drag` | it is the grab point, `to_x` and `to_y` are the drop |
| `fill` | text into a field, or a **number** into a slider or spinner |
| `focus` | focus only, no action |

**A slider or a spinner is set, not clicked.** `click` on one is refused, and the refusal
names its minimum, maximum and current value. Use `fill` with the number you want. A click
would have landed at the centre of the control's rectangle, which is halfway along its
travel, and reported success.

**`scroll` on a ref is the only way to reach an off-screen control without a screenshot.**
`read` skips anything scrolled out of view, so a control below the fold does not exist as
far as the tree is concerned. Scroll a nearby ref into view, then read again.

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

## Dragging

`left_click_drag` does the whole gesture in one call: press, move in steps, release. It
holds briefly at each end, because HTML5, Electron and most file managers treat a press
that moves immediately as a click and drop nothing. `hold_ms` changes that pause,
`button` makes it a right or middle drag, which is how CAD and 3D tools orbit and pan.

For anything the single call cannot express, `mouse_down` and `mouse_up` hold a button
across several calls, exactly as `key_down` and `key_up` do for the keyboard:

- drawing a path on a canvas, or a lasso selection
- reading a slider's value part way through its travel
- dropping onto a target that has to open under the pointer first
- a drag with a modifier: `key_down` Shift, `mouse_down`, `mouse_move`, `mouse_up`, `key_up`

## Keys

`type` types into whatever has focus, up to 5000 characters. For more than that, or for
anything with awkward characters, put it on the clipboard with `write_clipboard` and press
`Control+v`: there is no length limit that way, and no per-character timing to go wrong.

`key` presses one key or a chord.

**Both tell you which window they went to.** Read that line. A keystroke goes to the front
window, not to the one you have in mind, and on a multi-screen setup the window you were
thinking of is often on the other screen with something else in front. If the reported focus
is not what you expected, nothing you typed went where you thought: `focus_window` first,
then type again. Typing into LaRuche's own window is refused outright.

Named keys: `Enter`, `Tab`, `Escape`, `Backspace`, `Delete`, `Space`, `Home`, `End`,
`PageUp`, `PageDown`, `Up`, `Down`, `Left`, `Right`, `F1` to `F12`, or a single character.
Chord with `Control+c`, `Shift+Tab`, `Alt+F4`, `Meta+r`. `repeat` presses several times,
`hold_ms` holds each press.

`key_down` and `key_up` hold a key while other gestures happen, which is what a game needs
and what a drag with a modifier needs.

**Anything left held is released for you after a minute**, and `release_all` releases
everything on demand. Release deliberately anyway: a key still down changes what every
later click means, including the user's own, and the automatic release is a safety net for
an abandoned mission, not a substitute for finishing the gesture.

## The clipboard

`read_clipboard` and `write_clipboard` move text in and out. This is the shortest route
into and out of a native application, and it is the only way to read what a `Control+c`
actually copied.

## Waiting

`wait` `{ "ms": 1500 }` pauses while an installer advances, a menu animates open, or a
dialog appears. Use it instead of taking a screenshot you do not need: a capture costs
tokens and a wait costs nothing.

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
- **The human took the mouse back.** If the pointer has moved more than 30 pixels from
  where the tool left it, and less than 45 seconds have passed, the next gesture is refused
  and nothing is done. Say so and wait. Repeating the same call takes control back
  deliberately, so it is not a deadlock, but do not repeat it reflexively: someone is using
  the machine. Note that this guard only covers gestures that carry coordinates. `type` and
  `key` go to whatever has focus and are not checked.

Approval comes in two classes. Approving one observing action (`screens`, `screenshot`,
`cursor_position`) does **not** approve a click, and the first acting call is asked
separately.

The whole tool can be switched off on a node with `LARUCHE_COMPUTER=0`. If every call comes
back saying GUI control is disabled, that is why, and it is the operator's decision: say so
rather than looking for another way in.

## When not to use it

- A web page: `browser`.
- Moving, renaming, converting or reading files: `shell_exec` or `file_*`. Driving a file
  manager by clicking is slower and fails in more ways.
- Anything a command line can do: run the command. Clicking through a settings dialog to
  read a value that `systeminfo` prints is wasted work and wasted tokens.

The right use is the case with no other door: a native application with no CLI, an
installer, a dialog the OS put on top of everything.
