---
type: skill
name: design-md
description: Author, lint, diff, and export Google DESIGN.md token spec files.
version: 1.0.0
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec, file_write, file_read, file_edit]
metadata:
  laruche:
    tags: [design, design-system, tokens, ui, accessibility, wcag, tailwind, dtcg, google]
    related_skills: [excalidraw, architecture-diagram]
    homepage: https://github.com/google-labs-code/design.md
---

# DESIGN.md Skill

DESIGN.md is Google's open spec (Apache-2.0, `google-labs-code/design.md`) for
describing a visual identity to coding agents. One file combines:

- **YAML front matter** — machine-readable design tokens (normative values)
- **Markdown body** — human-readable rationale, organized into canonical sections

Tokens give exact values. Prose tells agents *why* those values exist and how to
apply them. The CLI (`npx @google/design.md`) lints structure + WCAG contrast,
diffs versions for regressions, and exports to Tailwind or W3C DTCG JSON.

## Triggers

- User asks for a DESIGN.md file, design tokens, or a design system spec
- User wants consistent UI/brand across multiple projects or tools
- User pastes an existing DESIGN.md and asks to lint, diff, export, or extend it
- User asks to port a style guide into a format agents can consume
- User wants contrast / WCAG accessibility validation on their color palette

## File anatomy

```md
---
version: alpha
name: Heritage
description: Architectural minimalism meets journalistic gravitas.
colors:
  primary: "#1A1C1E"
  secondary: "#6C7278"
  tertiary: "#B8422E"
  neutral: "#F7F5F2"
typography:
  h1:
    fontFamily: Public Sans
    fontSize: 3rem
    fontWeight: 700
    lineHeight: 1.1
    letterSpacing: "-0.02em"
  body-md:
    fontFamily: Public Sans
    fontSize: 1rem
rounded:
  sm: 4px
  md: 8px
  lg: 16px
spacing:
  sm: 8px
  md: 16px
  lg: 24px
components:
  button-primary:
    backgroundColor: "{colors.tertiary}"
    textColor: "#FFFFFF"
    rounded: "{rounded.sm}"
    padding: 12px
  button-primary-hover:
    backgroundColor: "{colors.primary}"
---

## Overview

Architectural Minimalism meets Journalistic Gravitas...

## Colors

- **Primary (#1A1C1E):** Deep ink for headlines and core text.
- **Tertiary (#B8422E):** "Boston Clay" — the sole driver for interaction.

## Typography

Public Sans for everything except small all-caps labels...

## Components

`button-primary` is the only high-emphasis action on a page...
```

## Token types

| Type | Format | Example |
|------|--------|---------|
| Color | `#` + hex (sRGB) | `"#1A1C1E"` |
| Dimension | number + unit (`px`, `em`, `rem`) | `48px`, `-0.02em` |
| Token reference | `{path.to.token}` | `{colors.primary}` |
| Typography | object with `fontFamily`, `fontSize`, `fontWeight`, `lineHeight`, `letterSpacing`, `fontFeature`, `fontVariation` | see above |

Component property whitelist: `backgroundColor`, `textColor`, `typography`,
`rounded`, `padding`, `size`, `height`, `width`. Variants (hover, active,
pressed) are **separate component entries** (`button-primary-hover`), not nested.

## Canonical section order

Sections are optional, but present ones MUST appear in this order. Duplicate
headings reject the file.

1. Overview (alias: Brand & Style)
2. Colors
3. Typography
4. Layout (alias: Layout & Spacing)
5. Elevation & Depth (alias: Elevation)
6. Shapes
7. Components
8. Do's and Don'ts

Unknown sections are preserved, not errored. Unknown token names are accepted
if the value type is valid. Unknown component properties produce a warning.

## Workflow: authoring a new DESIGN.md

1. **Ask or infer** the brand tone, accent color, and typography direction. If the
   user provided a site, image, or vibe, translate it to the token shape above.
2. **Write `DESIGN.md`** in the project root via `file_write`. Always include
   `name:` and `colors:`; other sections optional but encouraged.
3. **Use token references** (`{colors.primary}`) in `components:` instead of
   re-typing hex values — single-source palette.
4. **Lint it** (see below). Fix any broken references or WCAG failures before
   returning.
5. **If the project already exists**, also write Tailwind or DTCG exports next
   to the file (`tailwind.theme.json`, `tokens.json`).

## Workflow: lint / diff / export

Run via `shell_exec`. The CLI is `@google/design.md` (Node); use `npx -y` — no global install needed.

```bash
# Validate structure + token references + WCAG contrast
npx -y @google/design.md lint DESIGN.md

# Compare two versions (exit 1 = regression)
npx -y @google/design.md diff DESIGN.md DESIGN-v2.md

# Export to Tailwind theme JSON
npx -y @google/design.md export --format tailwind DESIGN.md > tailwind.theme.json

# Export to W3C DTCG JSON
npx -y @google/design.md export --format dtcg DESIGN.md > tokens.json

# Print the spec rules as JSON (useful for injecting into prompts)
npx -y @google/design.md spec --rules-only --format json
```

All commands accept `-` for stdin. `lint` exits 1 on errors. Pass `--format json`
and parse output to report findings structurally.

### Lint rules (what they catch)

- `broken-ref` (error) — `{colors.missing}` points at a non-existent token
- `duplicate-section` (error) — same `## Heading` appears twice
- `invalid-color`, `invalid-dimension`, `invalid-typography` (error)
- `wcag-contrast` (warning/info) — `textColor` vs `backgroundColor` ratio vs WCAG AA (4.5:1) and AAA (7:1)
- `unknown-component-property` (warning) — outside the whitelist

When accessibility matters, surface WCAG findings explicitly — they are the most
load-bearing reason to run the CLI.

## Pitfalls

- **Don't nest component variants.** `button-primary.hover` is wrong;
  `button-primary-hover` as a sibling key is right.
- **Hex colors must be quoted strings.** Unquoted `#` breaks YAML or truncates values.
- **Negative dimensions need quotes.** `letterSpacing: -0.02em` is a YAML flow scalar — write `"-0.02em"`.
- **Section order is enforced.** Reorder user-supplied prose to match the canonical list before saving.
- **`version: alpha` is the current spec version** (as of 2026). The spec is marked alpha — watch for breaking changes.
- **Token references resolve by dotted path.** `{colors.primary}` works; `{primary}` does not.

## Spec source

- Repo: https://github.com/google-labs-code/design.md (Apache-2.0)
- CLI: `@google/design.md` on npm
