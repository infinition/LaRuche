---
type: skill
name: pretext
description: "DOM-free text layout demos: reflow, glyph physics, text-as-geometry."
version: 1.0.0
license: MIT
platforms: [linux, macos, windows]
tools: [file_read, file_write, shell_exec]
metadata:
  laruche:
    tags: [creative-coding, typography, pretext, ascii-art, canvas, generative, text-layout, kinetic-typography]
---

# Pretext Creative Demos

## What It Is

[`@chenglou/pretext`](https://github.com/chenglou/pretext) is a 15KB zero-dependency TypeScript library for **DOM-free multiline text measurement and layout**. Given `(text, font, width)`, it returns line breaks, per-line widths, per-grapheme positions, and total height - all via canvas measurement, no reflow. Fast enough for per-frame animation at 60fps.

## When to Use

- Text flowing around a moving shape (animated hero sections, editorial layouts)
- Games where obstacles are made of real words (Breakout-of-prose, Tetris-from-letters)
- Kinetic typography with per-glyph physics (shatter, scatter, wave)
- ASCII-art effects using real words, not monospace rasters
- Multiline shrink-wrap UI (smallest container that fits the text)

Do **not** use for: static CSS layouts, rich text editors, image→text (other skills), pure generative art with no text.

## Stack

Single self-contained HTML file per demo. No build step.

| Layer | Tool | Purpose |
|-------|------|---------|
| Core | `@chenglou/pretext` via `esm.sh` CDN | Text measurement + line layout |
| Render | HTML5 Canvas 2D | Glyph rendering, per-frame composition |
| Segmentation | `Intl.Segmenter` (built-in) | Grapheme splitting for emoji / CJK |
| Interaction | Raw DOM events | Mouse / touch / wheel |

```html
<script type="module">
import {
  prepare, layout,
  prepareWithSegments, layoutWithLines,
  layoutNextLineRange, materializeLineRange,
  measureLineStats, walkLineRanges,
} from "https://esm.sh/@chenglou/pretext@0.0.6";
</script>
```

Pin the version. Check [npm](https://www.npmjs.com/package/@chenglou/pretext) for updates if behavior seems off.

## The Two API Shapes

### Shape 1 - Measure height, let CSS render

```js
const prepared = prepare(text, "16px Inter");
const { height, lineCount } = layout(prepared, 320, 20);
```

Use for virtualized lists, masonry card heights, layout-shift prevention. Keep `ctx.font` in sync with CSS exactly - drift of 5–20% happens silently if the font 404s.

### Shape 2 - Measure and render yourself (canvas)

```js
const prepared = prepareWithSegments(text, FONT);
const { lines } = layoutWithLines(prepared, 320, 26);
for (let i = 0; i < lines.length; i++) {
  ctx.fillText(lines[i].text, 0, i * 26);
}
```

For **variable-width-per-line** flow (text around a shape):

```js
let cursor = { segmentIndex: 0, graphemeIndex: 0 };
let y = 0;
while (true) {
  const lineWidth = widthAtY(y);           // corridor width at this y
  const range = layoutNextLineRange(prepared, cursor, lineWidth);
  if (!range) break;
  const line = materializeLineRange(prepared, range);
  ctx.fillText(line.text, leftEdgeAtY(y), y);
  cursor = range.end;
  y += lineHeight;
}
```

### Useful helpers

- `measureLineStats(prepared, maxWidth)` → `{ lineCount, maxLineWidth }` - for shrink-wrap width.
- `walkLineRanges(prepared, maxWidth, callback)` - iterate lines without allocating strings (stats/per-grapheme physics).
- `@chenglou/pretext/rich-inline` - paragraphs mixing fonts / chips / mentions.

## Demo Patterns

| Pattern | Key API | Example |
|---|---|---|
| **Reflow around obstacle** | `layoutNextLineRange` + per-row width fn | Paragraph parting around a dragged cursor sprite |
| **Text-as-geometry game** | `layoutWithLines` + per-line collision rects | Breakout where each brick is a measured word |
| **Shatter / particles** | `walkLineRanges` → per-grapheme (x,y) → physics | Sentence explodes into letters on click |
| **ASCII obstacle typography** | `layoutNextLineRange` + measured per-row spans | Bitmap logo that text flows around |
| **Editorial multi-column** | `layoutNextLineRange` per column + shared cursor | Animated magazine spread |
| **Kinetic type** | `layoutWithLines` + per-line transform over time | Wave, bounce, glitch, Star Wars crawl |
| **Shrink-wrap card** | `measureLineStats` | Quote card that auto-sizes to tightest container |

Templates in `templates/`:
- `hello-orb-flow.html` - reflow around moving orb
- `donut-orbit.html` - advanced: ASCII obstacles, draggable wire objects, morphing shapes

## Aesthetic Requirements

Every demo must go beyond "hello world":

- **Dark background, considered palette.** Amber-on-black (CRT), cold-white-on-charcoal (editorial), or desaturated pastels (risograph). Pick one.
- **Proportional fonts.** Iowan Old Style, Inter, JetBrains Mono, Helvetica Neue, or a variable font. Never default sans.
- **Real corpus.** Short manifesto, poem, source code, found text - never lorem ipsum.
- **First-paint excellence.** No blank frames. Add vignette, idle auto-motion, or one interactive response (drag, hover, scroll, click).

## Workflow

1. Pick a pattern from the table above.
2. Read the appropriate template via `file_read` (`templates/hello-orb-flow.html` or `templates/donut-orbit.html`).
3. Swap the corpus for intentional prose matching the brief (10–100 sentences).
4. Tune aesthetic: font, palette, composition, interaction.
5. Write the file via `file_write` to the user's workspace.
6. Verify locally:
   ```sh
   python3 -m http.server 8765
   # open http://localhost:8765/<file>.html
   ```
   Run via `shell_exec`. Check the console - pretext throws on bad font strings.
7. Return the file path to the user.

## Performance Notes

- `prepare()` / `prepareWithSegments()` is expensive - call **once** per text+font pair. Cache in module scope.
- On resize, only rerun `layout()` / `layoutWithLines()` - never re-prepare.
- `layoutNextLineRange` in a tight loop is cheap enough for 60fps on normal paragraphs.
- For ASCII masks per frame: keep a typed array cell buffer, derive per-row obstacle spans, merge spans, feed into `layoutNextLineRange`.
- `ctx.font` is slow to set - do it once per frame, not per `fillText` call.

## Common Pitfalls

1. **Drifting font strings.** `ctx.font = "16px Inter"` measured, but CSS falls back to sans-serif when Inter 404s - measurements drift 5–20%. Preload fonts or use a web-safe family.
2. **Re-preparing inside the animation loop.** Only `layout*` is cheap. Re-calling `prepare` every frame tanks perf.
3. **Forgetting `Intl.Segmenter` for graphemes.** `"é".split("")` gives 2 chars - always use `new Intl.Segmenter(undefined, { granularity: "grapheme" })` for per-glyph work.
4. **`break: 'never'` chips without `extraWidth`.** In `rich-inline`, atomic chips without `extraWidth` overflow the container.
5. **Using `unpkg` instead of `esm.sh`.** `unpkg` will 404 or serve raw TS. Always use `esm.sh`.
6. **Tiny maxWidth instead of skipping rows.** When a corridor is too narrow, skip the row (`y += lineHeight; continue;`) - passing tiny maxWidth produces one-grapheme broken lines.
7. **Cold first-paint.** Add vignette, scanline, idle auto-motion, or one interactive response. Without it the demo looks tutorial-grade.

## Verification Checklist

- [ ] Single self-contained `.html` - opens via `python3 -m http.server` or double-click
- [ ] `@chenglou/pretext` imported from `esm.sh` with pinned version
- [ ] Corpus is real prose matching the concept, not lorem ipsum
- [ ] Font string passed to `prepare` matches CSS font exactly
- [ ] `prepare()` / `prepareWithSegments()` called once, not per frame
- [ ] Dark background + considered palette
- [ ] At least one interactive response or idle auto-motion
- [ ] No console errors; 60fps on a mid-tier laptop
