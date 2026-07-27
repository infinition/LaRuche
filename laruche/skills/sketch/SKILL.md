---
type: skill
name: sketch
description: Build throwaway HTML mockups so the user can compare design variants.
---

# Sketch

Design arguments are unwinnable in prose. "Cleaner", "more modern", "less busy" mean
different things to everybody in the room. Build two or three real screens instead, put
them side by side, and the argument settles itself in ten seconds.

The output is disposable HTML the user can click, not a component you would ship. If a
sketch is good enough to keep, that is the signal to rebuild it properly, not to promote
it.

Reach for this on "show me what this could look like", "sketch this screen", "a couple of
takes on the layout", "before I build it". Do not use it when the design is already
decided, when the user asked for the production component, or when what they want is a
diagram.

## 1. Learn three things first

Ask one question at a time, and reflect the answer back before the next. Skip whatever
the user already gave you.

1. **The feel.** Adjectives, a mood, a product they admire. "Calm, editorial, like a
   reading app" tells you something; "minimal" tells you nothing, because everyone means
   a different thing by it.
2. **References.** Which products already feel that way to them.
3. **The one action.** What is the single most important thing someone does on this
   screen. Every variant has to serve that well, or the comparison is meaningless.

## 2. Take genuinely different positions

Two or three variants. Never one, because one is a proposal rather than a choice, and the
user ends up critiquing instead of choosing. Rarely four: past three, nobody can hold the
differences in mind.

Each variant commits to a different STANCE, not a different accent colour. Pick one axis
and go to the ends of it:

| Axis | Ends |
|---|---|
| Density | compact, airy, ultra-dense |
| Emphasis | content first, action first, tool first |
| Aesthetic | editorial, utilitarian, playful |
| Layout | one column, sidebar, split pane |

Three variants that differ only in shade are one variant with a colour picker. If you
cannot say in a sentence what each one BELIEVES, they are not different enough.

Name them for the stance, and number the round:

```
sketches/
  001-calm-editorial/index.html + README.md
  001-utilitarian-dense/index.html + README.md
  001-playful-split/index.html + README.md
```

## 3. Build them, do not describe them

One self-contained HTML file per variant. Inline `<style>`, no build step, no bundler, no
framework unless it arrives from a CDN in one line.

- **Real content.** Actual names, actual sentences, plausible numbers. Lorem ipsum hides
  exactly the problems a sketch exists to reveal: the heading that wraps, the label that
  overflows, the empty state nobody designed.
- **Interactive enough to judge**, which means: the primary action visibly does something,
  one real state transition (open, filter, toggle), and hover states on things that are
  clickable. More than that is engineering a thing you are about to throw away. Less than
  that is a picture.

A reasonable starting point:

```html
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    -webkit-font-smoothing: antialiased;
    color: #1a1a1a; background: #fafafa; line-height: 1.5;
  }
</style>
```

Write them with `file_write`, using ABSOLUTE paths:

```
shell_exec(command="mkdir -p sketches/001-calm-editorial")
file_write(path="C:/dev/project/sketches/001-calm-editorial/index.html", content="<!doctype html>...")
file_write(path="C:/dev/project/sketches/001-calm-editorial/README.md", content="## Variant: calm editorial\n...")
```

## 4. Look at it before you show it

**This is the step everyone skips, and it is the one that matters.** You cannot tell
whether a layout works by reading the HTML you just wrote.

```
browser_screenshot(url="file:///C:/dev/project/sketches/001-calm-editorial/index.html",
                   output_path="C:/dev/project/sketches/001-calm-editorial/shot.png")
```

Then read the image back with `file_read` and actually look at it.

**`browser_screenshot` accepts `file://`. `browser_navigate` does not.** The two tools
differ: `browser_navigate` validates the scheme and refuses anything that is not `http://`
or `https://`, so a `file://` URL comes back as `URL must start with http:// or https://`.
Only `browser_screenshot` passes the URL straight through to the browser.

If you need `browser_navigate`, serve the folder first:

```
shell_exec(command="cd sketches && python -m http.server 8765 &")
browser_navigate(url="http://localhost:8765/001-calm-editorial/index.html")
```

Fix what the screenshot shows before moving to the next variant: collapsed flex
containers, text over text, a font that never loaded, a card that escaped its grid. Repeat
per variant.

## 5. One README per variant

```markdown
## Variant: calm editorial

### The stance
One sentence on the principle this variant commits to.

### Choices
- Layout, typography, colour, interaction

### Trade-offs
- Strong at:
- Weak at:

### Best for
- The user or situation this genuinely serves
```

The trade-off section is the point. A variant with no weaknesses has not committed to
anything.

## 6. Head to head, with an opinion

```markdown
| | Calm editorial | Utilitarian dense | Playful split |
|---|---|---|---|
| Density | low | high | medium |
| Primary action visible | low | high | medium |
| Scannability | high | medium | low |
| Feel | calm, trusted | sharp, tool-like | inviting |

**Recommendation:** utilitarian dense, if these are daily power users. Calm editorial if
most visits are first visits. Playful split is the weakest of the three: it reaches for
both and commits to neither.
```

**Say which one you would pick and why.** A neutral table hands the decision back with
more to read and no help. The user can overrule a recommendation; they cannot overrule an
absence of one.

Then let them choose, ask for a hybrid, or send you round again.

## Shared tokens, when the project has an identity

```css
/* sketches/themes/tokens.css */
:root {
  --bg: #fafafa;
  --fg: #1a1a1a;
  --accent: #0066ff;
  --muted: #666;
  --radius: 8px;
}
```

Three colours and one font are enough for something disposable. A full design system in a
throwaway sketch is time spent on the wrong artefact.

## What to sketch next

When sketches exist and the user asks what is missing:

- **States nobody drew.** Empty, loading, error, one item, two hundred items. The happy
  path is the easy fifth of the work.
- **Screens referenced but never explored.**
- **Viewports.** It held at one width. Mobile? Ultrawide?
- **Consistency.** Two winning variants from different rounds made independent choices
  that have never been put in the same screen.

Propose two to four, named, and let the user pick.

## Traps

- **Lorem ipsum.** It makes every layout look fine. Use real sentences.
- **Three variants of one idea.** See section 2.
- **Showing without looking.** A broken screenshot shown to the user costs their trust in
  the whole round.
- **Polishing.** A sketch is done when the decision can be made, not when it is pretty.
- **Committing `sketches/`** without asking. Most repositories want it ignored.
- **Relative paths in `file_write`.** They land in the server's working directory, not the
  project.

## Failure modes

**The screenshot is blank or white.** The page needs JavaScript that has not run, or the
path is wrong. Confirm the file exists with `file_read` first; a typo in an absolute path
renders as an empty page rather than an error.

**`browser_navigate` refuses the URL.** It was a `file://` path. See section 4: use
`browser_screenshot`, or serve the directory over HTTP.

**The CDN did not load.** The machine is offline or filtered. Fall back to plain CSS;
every variant here should survive without a network anyway.

**The user likes parts of two variants.** That is the sketch working. Build the hybrid as
a new numbered round rather than editing either original: the comparison is the record of
how the decision was reached.

**The user cannot choose.** The variants are too close, or none serves the one action from
step 1. Go back to that answer and build something that takes a real position against it.
