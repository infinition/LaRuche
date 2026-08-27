---
type: skill
name: browser-control
description: Drive a real web browser to navigate, read, find, click, fill, press keys, hover, scroll, wait, screenshot, and read its console and network.
tools: [browser]
---

# Browser control

Drive a real Chrome across several steps with the `browser` tool. The session stays open
between calls, so a whole flow on one site works: load a page, read it, act, read again.

Call `browser` **directly**. It needs approval, so it cannot be wrapped in `tool_call` or
`run_script`; doing so returns "requires approval: call it DIRECTLY". Approving once covers
the rest of the session.

## The actions, and nothing else

`navigate`, `read`, `find`, `overlays`, `click`, `right_click`, `double_click`,
`middle_click`, `drag`, `fill`, `upload`, `key`, `hover`, `scroll`, `wait`, `eval`,
`screenshot`, `console`, `network`, `cookies`, `download`, `back`, `forward`, `tabs`,
`open_tab`, `select`, `resize`, `dialog`, `close`. There is no other action. If a gesture is not in this list, do it with `eval`.
Never describe an action you did not actually call: if you did not `scroll`, do not say
you scrolled.

## The loop that needs no screenshot

Navigation is done blind, by element number, and that is the fast path:

1. `navigate` `{ "url": "..." }`
2. `read` returns a numbered map: `ref_1 <button> Send`, `ref_2 <input:search> ...`, plus
   the page text.
3. `click` `{ "ref": 1 }` or `fill` `{ "ref": 2, "text": "..." }`, using those numbers.
4. `scroll` `{ "direction": "down" }` (or `up`, `top`, `bottom`; optional `amount` in
   pixels; optional `ref` to bring one element into view).

On a page with two hundred controls, `find` `{ "text": "panier" }` returns only the
matching refs instead of the whole map. It numbers them exactly like `read`, so a `click`
straight after it works.

**Refs come only from the latest `read`.** Every `read` renumbers them, and any navigation
or page change throws them away. After navigating, after submitting, after anything that
reloads or replaces content, `read` again before you `click` or `fill`. Acting on a stale
ref returns "No element ref_N on this page. Run read again".

`read` walks into shadow roots and into same-origin iframes, so a page that is a shell
around an embedded application reads normally. An element found inside a frame is marked
`[in frame]`. A **cross-origin** iframe is still invisible: the browser forbids reading
across origins from a script, and that is the one blind spot left. A payment field, a
reCAPTCHA or an embedded checkout usually lives in one; when the page clearly has a form
that never appears in `read`, that is why.

A `<select>` lists its options in the read, with `*` on the selected one. Choose with
`fill` and the option's visible label: `fill { ref: 5, text: "France" }`. It matches the
label first, then the value, then a fragment, and if nothing matches it tells you what was
available rather than doing nothing.

A file input cannot be filled, by any script, in any browser: that restriction is what
stops a page from helping itself to your disk. `upload` `{ "ref": 4, "path": "C:/..." }`
goes through the debugger instead. The path must be absolute.

## When nothing happens: something is covering the page

This is the failure that does not look like one. `read` returns the whole page, including
what is behind a modal. You pick a real button, the click is sent, no error comes back,
and nothing moves, because a layer is on top or the body is frozen.

**`overlays` is the first thing to run when a click seems to do nothing.** It reports what
is covering the page, how much of the view it takes, whether the body can still scroll, and
the refs of the buttons inside it.

**A consent banner is the user's decision, not an obstacle.** Never accept one on their
behalf. Prefer the option that refuses everything optional: "Reject all", "Refuser tout",
"Continue without accepting", or the equivalent behind a "Manage" or "Customise" panel,
which is often where the real refusal is hidden. If the only way past is to accept, stop
and ask. This runs in the user's own browser, under their own session, and the answer is
recorded as theirs.

## Real mouse gestures

`click` calls the element directly, which is the reliable path and the right default. The
following send actual mouse events at the element's position instead, for what a direct
call cannot express:

- `right_click` opens a context menu. Note that a menu drawn by the browser itself is not
  part of the page and will not show up in `read`.
- `double_click` for a map, an editor, a rename-in-place.
- `middle_click` opens a link in a background tab.
- `drag` `{ "ref": 2, "to_ref": 7 }` presses on one element and drops it on another,
  moving in steps with a pause at each end. Sortable lists, kanban boards and HTML5 drop
  zones only arm on a real press-and-move, so a direct call can never do this.

## Checking a responsive layout

`resize` `{ "preset": "mobile" }` emulates a phone viewport, `tablet` and `desktop` do the
obvious, and `width`/`height` take an exact size. `mobile` and `tablet` also switch the
engine into touch rendering, which is what actually makes a well-built page change.

It is an override, not a real window: it survives navigation and stays until you resize
again. Reload before judging, because a page that decided its layout at load time will not
re-decide on its own, then `screenshot`.

## Dialogs, which freeze everything

`alert()`, `confirm()` and `prompt()` stop the page's JavaScript dead. Nothing else runs
until one is answered, so every later call would sit there until it times out and report
"CDP timeout", which explains nothing.

They are intercepted and **dismissed** by default, and the result of whatever action raised
one tells you what it said. Dismissing is the conservative answer: it leaves the page as it
was, where accepting makes it go ahead, and going ahead is precisely what a `confirm()` is
asking permission for. "Delete permanently?" and "Continue?" look identical from here.

To accept one deliberately, call `dialog` `{ "accept": true }` first, then do the gesture
that raises it. Add `text` to answer a `prompt()`. The policy is used **once** and then
forgotten, so it can never answer a question you did not read.

In extension mode the dialog appears in the user's own Chrome and they answer it. If the
page seems frozen there, ask them to look at their browser.

## Downloads

Call `download` first: it allows downloading, picks the folder, and turns on the reporting
that gives you the filename. Then click the link. The name arrives attached to the next
action's result, with the full folder path, and you open it with `read_extract` or
`file_read`.

Without that first call, a download is a dead end: the browser either refuses silently or
drops the file somewhere you were never told, under a name you never saw.

**Tell the user what you downloaded and where it came from.** A file arriving on their disk
is not an implementation detail.

## Cookies, and what is not returned

`cookies` lists the names, sizes and domains of the current page's cookies. It never returns
their values, deliberately: a session cookie *is* the session, and printing one would put it
in the transcript, in memory, and possibly through a model provider. To check whether a
login worked, read the page.

## More than one tab

`tabs` lists them, `open_tab` `{ "url": "..." }` opens one, `select` `{ "tab_id": ... }`
moves you to it. Opening a tab does not move you into it. Refs belong to a page, so read
again after every switch.

`open_tab` is not available through the extension: putting a window in front of someone
without warning is not the tool's call to make.

## Keys, hover, and waiting

- `key` `{ "key": "Enter" }` presses a real key. Add `ref` to focus a field first, which is
  how you submit a search box without hunting for its button. Named keys: `Enter`, `Tab`,
  `Escape`, `Backspace`, `Delete`, `Space`, `Home`, `End`, `PageUp`, `PageDown`,
  `ArrowUp/Down/Left/Right`, `F1` to `F12`, or any single character. Chord with
  `Control+a`, `Shift+Tab`, `Alt+F`, `Meta+K`. `repeat` presses several times, `hold_ms`
  holds the key down before releasing it. These are real events from the browser itself, so
  sites that ignore synthetic ones react to them normally.
- `hover` `{ "ref": 3 }` moves the mouse onto an element without clicking. Dropdown menus
  that open on hover do not open on `click`, so when a menu is described in the page but its
  entries are missing from `read`, hover the parent then `read` again.
- `wait` `{ "text": "..." }` or `{ "selector": "..." }` blocks until it appears, up to
  `timeout` seconds (15 by default). Use it after an action that loads something instead of
  reading in a loop. With neither, `wait` `{ "amount": 800 }` just pauses.

## When a page misbehaves

- `console` returns what the page logged, newest last, with `level` to keep only `error`.
- `network` returns what it requested, with status and duration; `text` filters by URL.

Both only see what happened after LaRuche started driving the tab, and both are reset by a
navigation. Read them right after reproducing the problem, not later.

`back` and `forward` move through history. They throw the refs away like any navigation.

## When to look, and when to compute

- `screenshot` returns the rendering to you as an image, and it is shown in the chat. Use
  it only when you must judge layout, colour or a visual state. The text loop above does
  not need it.
- `eval` `{ "script": "..." }` runs JavaScript in an async wrapper: use `await`, and
  `return` the value you want. This reads data out of a page, checks a condition, or does
  anything without a dedicated action.

## A tab the user already has open

When the request points at an existing tab, "take my Dealabs tab", "read the page
I'm on", "the tab I left open", do **not** `navigate`. Navigating loads the site
fresh in your own driven tab and leaves theirs untouched, which is not what they
asked. Instead:

1. `tabs` lists every open tab across every Chrome window, with its tabId. Windows
   are tagged `[win N]`, the focused one with `*`, so two tabs of the same site in
   two windows are distinguishable.
2. `select` `{ "tab_id": <the id from tabs> }` adopts that exact tab. In extension
   mode it joins a yellow LaRuche tab group for the duration and Chrome raises its
   debug banner on it, so the takeover is visible; it leaves the group and the
   banner clears once you stop acting on it (or on `close`).
3. `read` it, then act.

`navigate` is only for opening a NEW page.

## Which browser it drives

`mode` chooses, `auto` by default:

- `auto`: the user's own Chrome via the LaRuche extension if it is connected, otherwise a
  browser LaRuche starts itself.
- `extension`: require the user's Chrome, with its open tabs and logged-in sessions. Since
  Chrome 136 this is the only way into an already-open session.
- `launch`: a browser LaRuche starts on its own persistent profile. Sign in once and the
  profile keeps the cookies for later runs. No extension needed.
- `attach`: an existing Chrome started with `--remote-debugging-port`.

The page under control shows an amber frame, a "LaRuche" badge and a floating panel the user
can drag anywhere or fold. The panel has three parts: the actions as they happen, what you
are saying right now, and a box where the user can answer you without leaving the page.
What they type there arrives as steering, mid-run, exactly as if they had typed it in the
chat window. You will see it as a user interruption; take it seriously, it usually means
you are doing the wrong thing. An amber cursor glides
to each target, typing appears character by character, and a screenshot triggers a brief
flash once the capture is taken. It all fades a few seconds after LaRuche stops acting on
the tab. Set `glow: false` for a clean screenshot, `animate: false` to act instantly on a
long sequence, `speed` above 1 to slow the motion down for a demonstration.

`close` ends the session and removes the indicator; the browser process is left running.

## A worked example

Search Marmiton and open the first result, honestly:

1. `navigate` to `https://www.marmiton.org`
2. `read`, find the search input's ref
3. `fill` that ref with `tarte aux pommes`, then `key` `{ "key": "Enter", "ref": <that
   ref> }` rather than looking for a search button
4. `read` again (navigation reset the refs), find the first recipe link's ref
5. `click` it, `read` again to get the ingredients and steps from the page text
6. `screenshot` only if the user wants to see the page
