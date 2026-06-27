---
type: skill
name: architecture-diagram
description: "Generate dark-themed SVG/HTML architecture diagrams, no dependencies."
version: 1.0.0
author: Cocoon AI (hello@cocoon-ai.com)
license: MIT
dependencies: []
platforms: [linux, macos, windows]
tools: [file_read, file_write]
metadata:
  laruche:
    tags: [architecture, diagrams, SVG, HTML, visualization, infrastructure, cloud]
    homepage: https://github.com/Cocoon-AI/architecture-diagram-generator
---

# Architecture Diagram Skill

Generate professional, dark-themed technical architecture diagrams as standalone self-contained HTML files (inline SVG). No API keys, no rendering libraries, no JavaScript — works offline in any modern browser.

## Scope

**Suited for:** software system architecture, cloud infrastructure (VPC/subnets/services), microservice topology, database+API maps, deployment diagrams.

**Not suited for:** physics/chemistry/math diagrams, floor plans, narrative journeys, hand-drawn whiteboard sketches.

## Workflow

1. User describes their system (components, connections, technologies, layers).
2. Load the bundled template for working examples:
   ```
   file_read("skills/architecture-diagram/templates/template.html")
   ```
   If the template is unavailable, proceed from the design system below.
3. Generate the full HTML following the design system.
4. Write with `file_write` to `./[project-name]-architecture.html`.
5. Inform the user: open the file in any browser, works offline.

## Design System

### Color Palette

| Component Type | Fill (rgba)            | Stroke (Hex)             |
| :------------- | :--------------------- | :----------------------- |
| Frontend       | `rgba(8,51,68,0.4)`    | `#22d3ee` (cyan-400)     |
| Backend        | `rgba(6,78,59,0.4)`    | `#34d399` (emerald-400)  |
| Database       | `rgba(76,29,149,0.4)`  | `#a78bfa` (violet-400)   |
| AWS/Cloud      | `rgba(120,53,15,0.3)`  | `#fbbf24` (amber-400)    |
| Security       | `rgba(136,19,55,0.4)`  | `#fb7185` (rose-400)     |
| Message Bus    | `rgba(251,146,60,0.3)` | `#fb923c` (orange-400)   |
| External       | `rgba(30,41,59,0.5)`   | `#94a3b8` (slate-400)    |

### Typography & Background
- **Font:** JetBrains Mono via Google Fonts `<link>`
- **Sizes:** 12px (names) · 9px (sublabels) · 8px (annotations) · 7px (tiny)
- **Background:** `#020617` (slate-950) with 40px grid:

```svg
<pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse">
  <path d="M 40 0 L 0 0 0 40" fill="none" stroke="#1e293b" stroke-width="0.5"/>
</pattern>
```

## Technical Implementation

### Component Rendering
Rounded rectangles (`rx="6"`, 1.5px stroke). Use **double-rect masking** to prevent arrows showing through semi-transparent fills:
1. Draw an opaque background rect (`#0f172a`)
2. Draw the semi-transparent styled rect on top

### Connections & Z-Order
- Draw arrows **early** in the SVG (after the grid, before component boxes) so they render behind boxes.
- Arrowheads via SVG `<marker>` elements.
- Security flows: dashed lines, rose (`#fb7185`).
- Security group boundaries: dashed `4,4`, rose color.
- Region boundaries: dashed `8,4`, amber color, `rx="12"`.

### Layout Rules
- Component height: 60px standard; 80–120px for large nodes.
- Minimum 40px vertical gap between components.
- Message buses: place **in the gap** between services, never overlapping.
- **Legend placement (critical):** always outside all boundary boxes. Calculate the lowest Y of all boundaries; place legend ≥ 20px below it.

## Document Structure

```
1. Header   — title + pulsing dot indicator + subtitle
2. SVG area — diagram in a rounded-border card
3. Cards    — 3-column summary grid (title · dot · bullet list)
4. Footer   — minimal metadata
```

## Output Requirements
- Single self-contained `.html` file.
- All CSS and SVG inline (Google Fonts `<link>` is the only external call).
- No JavaScript — pure CSS for animations.
- Renders in any modern browser, fully offline.
