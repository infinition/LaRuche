---
type: skill
name: sketch
description: "Throwaway HTML mockups: 2-3 design variants to compare."
version: 1.1.0
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec, file_write, file_read, browser_navigate, browser_screenshot]
metadata:
  laruche:
    tags: [sketch, mockup, design, ui, prototype, html, variants, exploration, wireframe, comparison]
    related_skills: [spike]
---

# Sketch

Use when the user wants to **explore a design direction before committing** — disposable HTML mockups to compare side-by-side. Goal: 2-3 interactive variants, not shippable code.

Trigger phrases: "sketch this screen", "show me what X could look like", "compare layout A vs B", "give me 2-3 takes on this UI", "mockup this before I build".

## When NOT to use

- User wants a production component → build it properly
- Design is already locked → just build it
- User wants a diagram → use a diagramming approach instead

## Core flow

```
intake → variants → visual check → head-to-head → pick winner (or iterate)
```

---

## 1. Intake (skip if user already gave you enough)

Ask one question at a time — three things needed:

1. **Feel.** "What should this feel like? Adjectives, emotions, a vibe." (*"calm, editorial, like Linear"* beats *"minimal"*)
2. **References.** "What apps, sites, or products capture that feel?"
3. **Core action.** "What's the single most important thing a user does on this screen?" (variants must all serve this well)

Reflect each answer briefly before the next question. If user gave all three upfront, skip straight to variants.

---

## 2. Variants (2-3, never 1, rarely 4+)

Produce 2-3 variants in one go — each a **complete standalone HTML file**. Don't describe variants; build them.

Each variant must take a **different design stance**, not just different colors. Pick one axis:

- **Density:** compact vs. airy vs. ultra-dense
- **Emphasis:** content-first vs. action-first vs. tool-first
- **Aesthetic:** editorial vs. utilitarian vs. playful
- **Layout:** single-column vs. sidebar vs. split-pane

**Variant naming:** describe the stance, not the number.

```
sketches/
├── 001-calm-editorial/
│   ├── index.html
│   └── README.md
├── 001-utilitarian-dense/
│   ├── index.html
│   └── README.md
└── 001-playful-split/
    ├── index.html
    └── README.md
```

---

## 3. Build real HTML

Each variant is a **single self-contained HTML file**:

- Inline `<style>` — no build step, no external CSS
- System fonts or one Google Font via `<link>`
- Tailwind via CDN is fine: `<script src="https://cdn.tailwindcss.com"></script>`
- Realistic fake content (actual sentences and names, not Lorem ipsum)
- **Interactive**: hovers real, at least one state transition (open/close, filter, toggle)

**Default CSS reset + system font stack:**

```html
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
                 "Helvetica Neue", Arial, sans-serif;
    -webkit-font-smoothing: antialiased;
    color: #1a1a1a;
    background: #fafafa;
    line-height: 1.5;
  }
</style>
```

**Tool sequence per variant:**

```
shell_exec("mkdir -p sketches/001-calm-editorial")
file_write("sketches/001-calm-editorial/index.html", "<!doctype html>...")
file_write("sketches/001-calm-editorial/README.md", "## Variant: Calm editorial\n...")
browser_navigate("file:///absolute/path/to/sketches/001-calm-editorial/index.html")
browser_screenshot()
```

Inspect the screenshot. Fix layout bugs (collapsed flex containers, failed font imports, overlapping text) before moving on. Repeat for each variant.

**Opening locally (fallback if no browser tool):**
- macOS: `shell_exec("open sketches/001-calm-editorial/index.html")`
- Linux: `shell_exec("xdg-open sketches/001-calm-editorial/index.html")`
- Windows: `shell_exec("start sketches/001-calm-editorial/index.html")`

---

## 4. Variant README

Each `README.md` answers:

```markdown
## Variant: {stance name}

### Design stance
One sentence on the principle driving this variant.

### Key choices
- Layout: ...
- Typography: ...
- Color: ...
- Interaction: ...

### Trade-offs
- Strong at: ...
- Weak at: ...

### Best for
- The kind of user or use case this variant actually serves
```

---

## 5. Head-to-head

After all variants are built, present a comparison table — **opinionate**, don't just list:

```markdown
## Three takes on the home screen

| Dimension | Calm editorial | Utilitarian dense | Playful split |
|-----------|----------------|-------------------|---------------|
| Density   | Low            | High              | Medium        |
| Primary action visibility | Low | High | Medium |
| Scan-ability | High | Medium | Low |
| Feel | Calm, trusted | Sharp, tool-like | Inviting, energetic |

**My take:** Utilitarian dense for power users, calm editorial for content-forward audiences.
Playful split is weakest — tries to do both and commits to neither.
```

Let the user pick a winner, ask for a hybrid, or request another round.

---

## Interactivity threshold

A sketch is interactive enough when the user can:

1. Click a primary action → something visible happens (state change, modal, toast, nav)
2. See one meaningful state transition (filter a list, toggle a mode, open/close a panel)
3. Hover recognizable affordances (buttons, rows, tabs)

More than that is over-engineering a throwaway. Less than that is a screenshot.

---

## Theming (when the project has a visual identity)

Put shared tokens in `sketches/themes/tokens.css` and `@import` in each variant:

```css
:root {
  --color-bg: #fafafa;
  --color-fg: #1a1a1a;
  --color-accent: #0066ff;
  --color-muted: #666;
  --radius: 8px;
  --font-display: "Inter", sans-serif;
  --font-body: -apple-system, BlinkMacSystemFont, sans-serif;
}
```

Three colors and one font is enough for a throwaway sketch.

---

## What to sketch next (frontier mode)

If sketches already exist and the user asks "what should I sketch next?":

- **Consistency gaps** — winning variants made independent choices not yet composed
- **Unsketched screens** — referenced but never explored
- **State coverage** — happy path done, but not empty / loading / error / overflow
- **Responsive gaps** — validated at one viewport; does it hold at mobile / ultrawide?
- **Interaction patterns** — static layouts exist; transitions, drag, scroll behavior don't

Propose 2-4 named candidates. Let the user pick.

---

## Output summary

- Create `sketches/` in the repo root (or `.planning/sketches/` if project uses that convention)
- One subdir per variant: `NNN-stance-name/index.html` + `README.md`
- Keep variants disposable — a sketch worth preserving should be promoted to real project code
