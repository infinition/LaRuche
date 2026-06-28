---
type: skill
name: excalidraw
description: "Generate Excalidraw JSON diagrams: arch, flow, seq."
version: 1.0.0
license: MIT
dependencies: []
platforms: [linux, macos, windows]
tools: [file_write, shell_exec]
scripts: [scripts/upload.py]
metadata:
  laruche:
    tags: [Excalidraw, Diagrams, Flowcharts, Architecture, Visualization, JSON]
    related_skills: []

---

# Excalidraw Diagram Skill

Create diagrams by writing standard Excalidraw element JSON and saving as `.excalidraw` files. Drag-and-drop onto [excalidraw.com](https://excalidraw.com) for viewing and editing. No accounts, no API keys, no rendering libraries - just JSON.

## When to Use

Generate `.excalidraw` files for architecture diagrams, flowcharts, sequence diagrams, concept maps. Files can be opened at excalidraw.com or uploaded for shareable links.

## Workflow

1. **Write the elements JSON** - an array of Excalidraw element objects
2. **Save the file** using `file_write` to create a `.excalidraw` file
3. **Optionally upload** for a shareable link using `scripts/upload.py` via `shell_exec`

### Saving a Diagram

Wrap elements in the standard envelope and save with `file_write`:

```json
{
  "type": "excalidraw",
  "version": 2,
  "elements": [ ...your elements array here... ],
  "appState": {
    "viewBackgroundColor": "#ffffff"
  }
}
```

Save to any path, e.g. `~/diagrams/my_diagram.excalidraw`.

### Uploading for a Shareable Link

```bash
python skills/excalidraw/scripts/upload.py ~/diagrams/my_diagram.excalidraw
```

Uploads to excalidraw.com (no account needed), prints a shareable URL. Requires: `pip install cryptography`.

---

## Element Format Reference

### Required Fields (all elements)
`type`, `id` (unique string), `x`, `y`, `width`, `height`

### Defaults (skip - applied automatically)
- `strokeColor`: `"#1e1e1e"`
- `backgroundColor`: `"transparent"`
- `fillStyle`: `"solid"`
- `strokeWidth`: `2`
- `roughness`: `1` (hand-drawn look)
- `opacity`: `100`

### Element Types

**Rectangle**:
```json
{ "type": "rectangle", "id": "r1", "x": 100, "y": 100, "width": 200, "height": 100 }
```
- `roundness: { "type": 3 }` for rounded corners
- `backgroundColor: "#a5d8ff"`, `fillStyle: "solid"` for filled

**Ellipse**:
```json
{ "type": "ellipse", "id": "e1", "x": 100, "y": 100, "width": 150, "height": 150 }
```

**Diamond**:
```json
{ "type": "diamond", "id": "d1", "x": 100, "y": 100, "width": 150, "height": 150 }
```

**Labeled shape (container binding)**

> **WARNING:** Do NOT use `"label": { "text": "..." }` on shapes - it is silently ignored, producing blank shapes. Use container binding below.

Shape needs `boundElements` listing the text; text needs `containerId` pointing back:
```json
{ "type": "rectangle", "id": "r1", "x": 100, "y": 100, "width": 200, "height": 80,
  "roundness": { "type": 3 }, "backgroundColor": "#a5d8ff", "fillStyle": "solid",
  "boundElements": [{ "id": "t_r1", "type": "text" }] },
{ "type": "text", "id": "t_r1", "x": 105, "y": 110, "width": 190, "height": 25,
  "text": "Hello", "fontSize": 20, "fontFamily": 1, "strokeColor": "#1e1e1e",
  "textAlign": "center", "verticalAlign": "middle",
  "containerId": "r1", "originalText": "Hello", "autoResize": true }
```
- Works on rectangle, ellipse, diamond
- Text is auto-centered when `containerId` is set; `x`/`y`/`width`/`height` are approximate
- `originalText` must match `text`; always include `fontFamily: 1` (Virgil/hand-drawn font)

**Labeled arrow**:
```json
{ "type": "arrow", "id": "a1", "x": 300, "y": 150, "width": 200, "height": 0,
  "points": [[0,0],[200,0]], "endArrowhead": "arrow",
  "boundElements": [{ "id": "t_a1", "type": "text" }] },
{ "type": "text", "id": "t_a1", "x": 370, "y": 130, "width": 60, "height": 20,
  "text": "connects", "fontSize": 16, "fontFamily": 1, "strokeColor": "#1e1e1e",
  "textAlign": "center", "verticalAlign": "middle",
  "containerId": "a1", "originalText": "connects", "autoResize": true }
```

**Standalone text** (titles and annotations - no container):
```json
{ "type": "text", "id": "t1", "x": 150, "y": 138, "text": "Hello", "fontSize": 20,
  "fontFamily": 1, "strokeColor": "#1e1e1e", "originalText": "Hello", "autoResize": true }
```
- `x` is the LEFT edge. To center at `cx`: `x = cx - (text.length * fontSize * 0.5) / 2`
- Do NOT rely on `textAlign` or `width` for positioning

**Arrow**:
```json
{ "type": "arrow", "id": "a1", "x": 300, "y": 150, "width": 200, "height": 0,
  "points": [[0,0],[200,0]], "endArrowhead": "arrow" }
```
- `points`: `[dx, dy]` offsets from element `x`, `y`
- `endArrowhead`: `null` | `"arrow"` | `"bar"` | `"dot"` | `"triangle"`
- `strokeStyle`: `"solid"` (default) | `"dashed"` | `"dotted"`

### Arrow Bindings (connect arrows to shapes)

```json
{
  "type": "arrow", "id": "a1", "x": 300, "y": 150, "width": 150, "height": 0,
  "points": [[0,0],[150,0]], "endArrowhead": "arrow",
  "startBinding": { "elementId": "r1", "fixedPoint": [1, 0.5] },
  "endBinding": { "elementId": "r2", "fixedPoint": [0, 0.5] }
}
```

`fixedPoint` coordinates: `top=[0.5,0]`, `bottom=[0.5,1]`, `left=[0,0.5]`, `right=[1,0.5]`

### Drawing Order (z-order)
- Array order = z-order (first = back, last = front)
- GOOD: `bg_zone → shape1 → text_for_shape1 → arrow1 → arrow_label → shape2 → text_for_shape2`
- BAD: all rectangles, then all texts, then all arrows
- Always place the bound text element immediately after its container shape

### Sizing Guidelines

**Font sizes:**
- Body text / labels: `fontSize` ≥ 16
- Titles / headings: `fontSize` ≥ 20
- Secondary annotations: `fontSize` ≥ 14 (use sparingly)
- Never use `fontSize` below 14

**Element sizes:**
- Minimum shape: 120×60 for labeled rectangles/ellipses
- Leave 20–30px gaps between elements minimum

### Color Palette

| Use | Fill Color | Hex |
|-----|-----------|-----|
| Primary / Input | Light Blue | `#a5d8ff` |
| Success / Output | Light Green | `#b2f2bb` |
| Warning / External | Light Orange | `#ffd8a8` |
| Processing / Special | Light Purple | `#d0bfff` |
| Error / Critical | Light Red | `#ffc9c9` |
| Notes / Decisions | Light Yellow | `#fff3bf` |
| Storage / Data | Light Teal | `#c3fae8` |

### Tips
- Use the color palette consistently
- **Text contrast is CRITICAL** - never light gray on white. Minimum text color on white: `#757575`
- Do NOT use emoji in text - they don't render in Excalidraw's font
- For dark mode diagrams: see `references/dark-mode.md`
- For larger examples: see `references/examples.md`
