---
type: skill
name: claude-design
description: Build self-contained HTML artifacts: landings, decks, prototypes, component labs.
version: 1.0.0
author: BadTechBandit
license: MIT
platforms: [linux, macos, windows]
tools: [file_write, file_read, file_list, shell_exec, browser_navigate, browser_screenshot, web_fetch, read_extract]
metadata:
  laruche:
    tags: [design, html, prototype, ux, ui, creative, artifact, deck, motion, design-system]
    related_skills: [design-md, popular-web-designs, excalidraw, architecture-diagram]
---

# Claude Design — LaRuche Mode

Use this skill for one-off designed artifacts: landing pages, prototypes, decks, component labs, motion studies.

**Skill routing:**

| Need | Skill |
|---|---|
| From-scratch designed artifact | **claude-design** (this one) |
| Match a known brand (Stripe, Linear, Vercel…) | `popular-web-designs` — load alongside this one |
| Author a persistent token spec file | `design-md` |

## Runtime Tools

No hosted UI, no preview panes. Use these LaRuche tools:

- `file_write` — write HTML artifacts to disk
- `file_read` / `file_list` — inspect repo/theme files before designing
- `shell_exec` — run syntax checks
- `browser_navigate` — open the artifact in a real browser
- `browser_screenshot` — capture and verify the rendered result
- `web_fetch` / `read_extract` — fetch brand docs, GitHub sources

Default deliverable: a complete self-contained HTML file. Report exact on-disk path. Verify before saying done.

## Workflow

1. **Understand the brief** — artifact type, audience, locked constraints.
2. **Gather context** — `file_list` + `file_read` on theme/token/component files. Do not invent UI when source exists.
3. **Ask focused questions** for ambiguous or high-fidelity work: output format, audience, fidelity, source materials, brand/design system, variants, conservative vs. divergent. Skip when direction is clear.
4. **Define the mini design system** — colors, type, spacing, radii, shadows, motion posture.
5. **Choose format:**
   - Static comparison → single HTML canvas, options side by side
   - Flow/interaction → clickable prototype
   - Presentation → fixed-size HTML deck with slide navigation
   - Component lab → variants in one page
   - Motion study → timeline or state-based animation
6. **Build the artifact** — single self-contained HTML (CSS in `<style>`, JS in `<script>`). Save prior versions as `Name v2.html` before major revisions.
7. **Verify** — `file_read` to confirm file exists, `browser_navigate` to open it, `browser_screenshot` to capture the primary viewport. Never claim visual verification unless `browser_screenshot` was actually called.
8. **Report** — exact path, what it contains, what was verified, next suggested action.

## HTML Standards

- Descriptive filenames: `Landing Page.html`, `Command Palette Prototype.html`
- CSS variables for tokens; CSS Grid for layout; container queries when helpful
- `text-wrap: pretty`; real focus + hover states; `prefers-reduced-motion` for non-trivial animation
- Responsive unless format is intentionally fixed-size
- Mobile hit targets ≥ 44px; deck text ≥ 24px; print text ≥ 12pt
- Avoid remote CDN dependencies unless stable and necessary

**React in standalone HTML** (only when state/variants/interaction warrants it):
- Pin exact CDN versions; no `type="module"` unless needed
- No multiple globals named `styles` — use specific names (`deckStyles`, `paletteStyles`)
- Attach shared components to `window` when splitting Babel scripts

## Deck Rules

Fixed canvas 1920×1080 (16:9), scaled to viewport. Required: keyboard navigation, visible slide count, `localStorage` persistence. Keep slides sparse — solve empty space with layout/rhythm/scale, not filler. No speaker notes unless asked. Max 2 background colors unless brand requires more.

## Prototype Rules

Make the primary path clickable. Include key states: default, hover/focus, loading, empty, error, success. Persist important state in `localStorage` when refresh continuity matters. Design the flow, not just the first screen.

## Variation Rules

Default to three options: **Conservative** (lowest risk), **Strong-fit** (best interpretation), **Divergent** (novel, tests taste boundaries). Variations explore layout, hierarchy, type scale, density, color posture, interaction model — not mere color swaps. When the user picks a direction, consolidate.

## Tweaks Panel

Add unobtrusive in-page controls when useful: theme mode, layout variant, density, accent color, type scale, motion on/off. Design must look final when tweaks are hidden. Persist with `localStorage`.

## Design Principles

**Start from context, not vibes.** Read existing source before inventing.

**Anti-slop** — avoid: aggressive gradients, glassmorphism by default, emoji without brand reason, generic SaaS icon-card grids, left-border accent callouts, fake dashboards with arbitrary numbers, stock-photo heroes, oversized rounded rectangles as hierarchy substitute, rainbow palettes, vague labels ("Insights", "Scale", "Optimize").

**Content discipline** — every element earns its place. No fake metrics, decorative stats, placeholder testimonials, or AI fluff sections. Mark copy as draft when not final.

**Typography** — use the existing type system if present. Otherwise: editorial → serif headline + restrained sans; software/productivity → precise sans with strong numeric treatment; deck → large, clear, high contrast. Type sets hierarchy before boxes or color.

**Color** — brand/system colors first. If inventing: small system (neutrals, surface, ink, muted, border, accent, danger/success), one primary accent, prefer `oklch` for harmonious custom palettes, check contrast.

**Layout** — rhythm via scale, whitespace, density, alignment, repetition, contrast, interruption. Not every section is a card grid.

**Motion** — clarifies state, reduces loading anxiety, shows continuity. Never loops without purpose, delays the user, or hides poor hierarchy. Always `prefers-reduced-motion`.

**Copyright** — extract principles (density, command-first, monochrome+accent, editorial hierarchy), do not clone proprietary layouts or reproduce branded screens.

## Pitfalls

- Do not over-ask when the user already gave enough direction.
- Do not under-ask for high-fidelity work with no brand context.
- Do not produce generic SaaS layouts and call them designed.
- Never claim browser verification unless `browser_screenshot` was actually called.
- Never say "done" if the file was not written.
