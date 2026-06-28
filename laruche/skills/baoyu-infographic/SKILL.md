---
type: skill
name: baoyu-infographic
description: "Generate infographics: 21 layouts × 21 styles, combinable."
version: 1.56.1
author: 宝玉 (JimLiu)
license: MIT
platforms: [linux, macos, windows]
tools: [file_read, file_write, shell_exec, media_present]
metadata:
  laruche:
    tags: [infographic, visual-summary, creative, image-generation]
    homepage: https://github.com/JimLiu/baoyu-skills#baoyu-infographic
---

# Infographic Generator

Adapted from [baoyu-infographic](https://github.com/JimLiu/baoyu-skills) for LaRuche.

Two dimensions: **layout** (information structure) × **style** (visual aesthetics). Any layout can be combined with any style.

Trigger keywords: "infographic", "visual summary", "信息图", "可视化", "高密度信息大图".

## Options

| Option | Values |
|--------|--------|
| Layout | 21 options (see Layout Gallery), default: `bento-grid` |
| Style | 21 options (see Style Gallery), default: `craft-handmade` |
| Aspect | `landscape` (16:9), `portrait` (9:16), `square` (1:1), or custom W:H (e.g. `3:4`) |
| Language | en, zh, ja, etc. |

## Layout Gallery

| Layout | Best For |
|--------|----------|
| `linear-progression` | Timelines, processes, tutorials |
| `binary-comparison` | A vs B, before-after, pros-cons |
| `comparison-matrix` | Multi-factor comparisons |
| `hierarchical-layers` | Pyramids, priority levels |
| `tree-branching` | Categories, taxonomies |
| `hub-spoke` | Central concept with related items |
| `structural-breakdown` | Exploded views, cross-sections |
| `bento-grid` | Multiple topics, overview (default) |
| `iceberg` | Surface vs hidden aspects |
| `bridge` | Problem-solution |
| `funnel` | Conversion, filtering |
| `isometric-map` | Spatial relationships |
| `dashboard` | Metrics, KPIs |
| `periodic-table` | Categorized collections |
| `comic-strip` | Narratives, sequences |
| `story-mountain` | Plot structure, tension arcs |
| `jigsaw` | Interconnected parts |
| `venn-diagram` | Overlapping concepts |
| `winding-roadmap` | Journey, milestones |
| `circular-flow` | Cycles, recurring processes |
| `dense-modules` | High-density modules, data-rich guides |

Full definitions: `references/layouts/<layout>.md`

## Style Gallery

| Style | Description |
|-------|-------------|
| `craft-handmade` | Hand-drawn, paper craft (default) |
| `claymation` | 3D clay figures, stop-motion |
| `kawaii` | Japanese cute, pastels |
| `storybook-watercolor` | Soft painted, whimsical |
| `chalkboard` | Chalk on black board |
| `cyberpunk-neon` | Neon glow, futuristic |
| `bold-graphic` | Comic style, halftone |
| `aged-academia` | Vintage science, sepia |
| `corporate-memphis` | Flat vector, vibrant |
| `technical-schematic` | Blueprint, engineering |
| `origami` | Folded paper, geometric |
| `pixel-art` | Retro 8-bit |
| `ui-wireframe` | Grayscale interface mockup |
| `subway-map` | Transit diagram |
| `ikea-manual` | Minimal line art |
| `knolling` | Organized flat-lay |
| `lego-brick` | Toy brick construction |
| `pop-laboratory` | Blueprint grid, coordinate markers, lab precision |
| `morandi-journal` | Hand-drawn doodle, warm Morandi tones |
| `retro-pop-grid` | 1970s retro pop art, Swiss grid, thick outlines |
| `hand-drawn-edu` | Macaron pastels, hand-drawn wobble, stick figures |

Full definitions: `references/styles/<style>.md`

## Recommended Combinations

| Content Type | Layout + Style |
|--------------|----------------|
| Timeline/History | `linear-progression` + `craft-handmade` |
| Step-by-step | `linear-progression` + `ikea-manual` |
| A vs B | `binary-comparison` + `corporate-memphis` |
| Hierarchy | `hierarchical-layers` + `craft-handmade` |
| Overlap | `venn-diagram` + `craft-handmade` |
| Conversion | `funnel` + `corporate-memphis` |
| Cycles | `circular-flow` + `craft-handmade` |
| Technical | `structural-breakdown` + `technical-schematic` |
| Metrics | `dashboard` + `corporate-memphis` |
| Educational | `bento-grid` + `chalkboard` |
| Journey | `winding-roadmap` + `storybook-watercolor` |
| Categories | `periodic-table` + `bold-graphic` |
| Product Guide | `dense-modules` + `morandi-journal` |
| Technical Guide | `dense-modules` + `pop-laboratory` |
| Trendy Guide | `dense-modules` + `retro-pop-grid` |
| Educational Diagram | `hub-spoke` + `hand-drawn-edu` |
| Process Tutorial | `linear-progression` + `hand-drawn-edu` |

Default: `bento-grid` + `craft-handmade`

## Keyword Shortcuts

When user input matches these keywords, **auto-select** the layout and offer the listed styles as top recommendations in Step 3. Skip content-based layout inference.

If a shortcut has **Prompt Notes**, append them verbatim to the generated prompt (Step 5).

| User Keyword | Layout | Recommended Styles | Default Aspect | Prompt Notes |
|--------------|--------|--------------------|----------------|--------------|
| 高密度信息大图 / high-density-info | `dense-modules` | `morandi-journal`, `pop-laboratory`, `retro-pop-grid` | portrait | - |
| 信息图 / infographic | `bento-grid` | `craft-handmade` | landscape | Minimalist: clean canvas, ample whitespace, no complex background textures. Simple cartoon elements and icons only. |

## Output Structure

```
infographic/{topic-slug}/
├── source-{slug}.{ext}
├── analysis.md
├── structured-content.md
├── prompts/infographic.md
└── infographic.png
```

Slug: 2-4 words kebab-case from topic. On conflict: append `-YYYYMMDD-HHMMSS`.

## Workflow

### Step 1: Analyze Content

**Load references**: Read `references/analysis-framework.md` via `file_read`.

1. Save source content to `source.md` via `file_write`. If `source.md` already exists, rename it first: `shell_exec mv source.md source-backup-YYYYMMDD-HHMMSS.md`
2. Analyze: topic, data type, complexity, tone, audience.
3. Detect source language and user language.
4. Extract design instructions from user input.
5. Save analysis to `analysis.md` via `file_write` (rename existing first).

See `references/analysis-framework.md` for output format.

### Step 2: Generate Structured Content → `structured-content.md`

Transform content into infographic structure:
1. Title and learning objectives
2. Sections with: key concept, content (verbatim), visual element, text labels
3. Data points (all statistics/quotes copied exactly)
4. User design instructions

Save via `file_write`. **Rules**: Markdown only. No new information. Preserve data faithfully. Strip any credentials, API keys, or tokens from output.

See `references/structured-content-template.md` for format.

### Step 3: Recommend Combinations

**3.1 Keyword shortcuts first**: If user input matches a keyword from the Keyword Shortcuts table, auto-select the associated layout and prioritize the listed styles. Skip content-based inference.

**3.2 Otherwise**, recommend 3-5 layout×style combinations based on data structure, content tone, audience, and user design instructions.

### Step 4: Confirm Options

Ask the user directly (LaRuche has no `clarify` tool):

- **Q1 - Combination**: Present 3+ layout×style combos with rationale. Ask user to pick.
- **Q2 - Aspect**: Ask for aspect ratio (landscape/portrait/square or custom W:H).
- **Q3 - Language** (only if source ≠ user language): Which language for text content?

### Step 5: Generate Prompt → `prompts/infographic.md`

Rename existing `prompts/infographic.md` first if it exists (`shell_exec`).

Load via `file_read`:
- `references/layouts/<layout>.md` (selected layout)
- `references/styles/<style>.md` (selected style)
- `references/base-prompt.md`

Assemble final prompt:
1. Layout definition
2. Style definition
3. Base template
4. Structured content from Step 2
5. All text in confirmed language
6. Keyword Shortcut Prompt Notes if applicable

**Aspect ratio substitution for `{{ASPECT_RATIO}}`**:
- `landscape` → `16:9` | `portrait` → `9:16` | `square` → `1:1`
- Custom W:H → use as-is (e.g. `3:4`, `2.35:1`)

Save to `prompts/infographic.md` via `file_write`.

### Step 6: Generate Image

Use `media_present` (or the image-generation tool active in your deployment) to render the prompt from Step 5.

- Map aspect to supported format: `16:9` → landscape, `9:16` → portrait, `1:1` → square
- Custom ratios: map to nearest named aspect (e.g. `3:4` → portrait)
- On failure: auto-retry once with the same prompt; if it fails again, report the error with the full prompt path so the user can retry manually
- Save resulting image URL/path into the output directory

### Step 7: Output Summary

Report: topic, layout, style, aspect, language, output path, and list of files created.

## References

- `references/analysis-framework.md` - Analysis methodology
- `references/structured-content-template.md` - Content format
- `references/base-prompt.md` - Prompt template
- `references/layouts/<layout>.md` - 21 layout definitions
- `references/styles/<style>.md` - 21 style definitions

## Pitfalls

1. **Data integrity** - never summarize or paraphrase source statistics. "73% increase" stays "73% increase".
2. **Strip secrets** - scan source content for API keys, tokens, or credentials before writing any output file.
3. **One concept per section** - overloading sections reduces readability.
4. **Style consistency** - apply the selected style uniformly; do not mix styles mid-infographic.
5. **Aspect ratio mapping** - the image tool only supports `landscape`, `portrait`, `square`. Map custom ratios to the nearest named option.
6. **Tool names** - use `file_write` / `file_read` (not `write_file` / `read_file`). Use `shell_exec` for renames.
