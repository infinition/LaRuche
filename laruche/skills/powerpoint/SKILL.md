---
type: skill
name: powerpoint
description: "Create, edit, extract, QA .pptx files via XML and bundled scripts."
version: "1.1.0"
license: Proprietary. LICENSE.txt has complete terms
platforms: [linux, macos, windows]
tools: [shell_exec, run_script, execute_code, file_read, file_write]
scripts:
  - scripts/add_slide.py
  - scripts/clean.py
  - scripts/office/pack.py
dependencies:
  python:
    - markitdown[pptx]
    - defusedxml
  system:
    - LibreOffice (soffice)       # PDF conversion for visual QA
    - Poppler (pdftoppm)          # PDF→images
  optional:
    - pptxgenjs (npm)             # programmatic creation alternative
---

# PowerPoint Skill

## When to Use

Any time a `.pptx` file is involved: creating decks, reading/extracting content, editing slides, adding/removing slides, working with templates, layouts, or notes.

---

## Reading Content

```bash
# Extract all text (markitdown[pptx] required)
python -m markitdown presentation.pptx

# Inspect raw XML (a .pptx is a zip)
unzip -o presentation.pptx -d unpacked/
```

---

## Editing Workflow

```bash
# 1. Unpack
unzip -o input.pptx -d unpacked/

# 2. Inspect layouts
ls unpacked/ppt/slideLayouts/

# 3. Add a slide
python scripts/add_slide.py unpacked/ slide2.xml          # duplicate existing
python scripts/add_slide.py unpacked/ slideLayout2.xml    # from layout
# Script prints the <p:sldId> element — add it to unpacked/ppt/presentation.xml <p:sldIdLst>

# 4. Edit slide XML directly
# Files: unpacked/ppt/slides/slideN.xml
# Namespaces: p: (presentationml), a: (drawingml), r: (relationships)

# 5. Clean orphaned files
python scripts/clean.py unpacked/

# 6. Repack
python scripts/office/pack.py unpacked/ output.pptx --original input.pptx
# Creating from scratch (no original):
python scripts/office/pack.py unpacked/ output.pptx --validate false
```

**Key XML patterns:**

```xml
<!-- Text shape -->
<p:sp>
  <p:nvSpPr>
    <p:cNvPr id="2" name="Title"/>
    <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
    <p:nvPr><p:ph type="title"/></p:nvPr>
  </p:nvSpPr>
  <p:txBody>
    <a:bodyPr/>
    <a:p><a:r><a:t>Your text here</a:t></a:r></a:p>
  </p:txBody>
</p:sp>

<!-- Position/size: EMU units. 1 inch = 914400 EMU. Slide = 9144000 x 5143500 EMU -->
<a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
```

---

## Creating from Scratch

**Option A — XML (recommended):** unzip any blank .pptx as a base, add slides with `add_slide.py` using `slideLayout` sources, then edit XML directly.

**Option B — pptxgenjs (optional, Node.js):**

```bash
npm install -g pptxgenjs
node -e "
const pptxgen = require('pptxgenjs');
const prs = new pptxgen();
let slide = prs.addSlide();
slide.addText('Hello World', { x: 1, y: 1, fontSize: 36 });
prs.writeFile({ fileName: 'output.pptx' });
"
```

---

## Design: Don't Make Boring Slides

### Palette Selection

Pick a bold, content-informed palette. One color dominates (60-70%), 1-2 supporting tones, one sharp accent. Vary light/dark across slides (dark title + light content = "sandwich"; or commit fully dark for premium look).

| Theme | Primary | Secondary | Accent |
|-------|---------|-----------|--------|
| Midnight Executive | `1E2761` (navy) | `CADCFC` (ice blue) | `FFFFFF` |
| Forest & Moss | `2C5F2D` (forest) | `97BC62` (moss) | `F5F5F5` |
| Coral Energy | `F96167` (coral) | `F9E795` (gold) | `2F3C7E` |
| Warm Terracotta | `B85042` (terracotta) | `E7E8D1` (sand) | `A7BEAE` |
| Charcoal Minimal | `36454F` (charcoal) | `F2F2F2` (off-white) | `212121` |
| Cherry Bold | `990011` (cherry) | `FCF6F5` (off-white) | `2F3C7E` |

### Layouts (vary across slides, never text-only)

- Two-column (text left, visual right)
- Icon + text rows (icon in colored circle, bold header, description below)
- 2x2 / 2x3 grid with image on one side
- Half-bleed image (full left/right) with content overlay
- Large stat callout (60-72pt number, small label below)
- Timeline / process flow (numbered steps, arrows)

### Typography

| Element | Size |
|---------|------|
| Slide title | 36-44pt bold (Georgia, Arial Black, Cambria, Trebuchet) |
| Section header | 20-24pt bold |
| Body text | 14-16pt (Calibri or Arial) |
| Captions | 10-12pt muted |

Margins: 0.5" min. Gaps between blocks: 0.3-0.5" consistent. Left-align body; center only titles.

### Avoid

- Repeating the same layout slide after slide
- Centering body paragraphs
- Defaulting to blue — match palette to the topic
- Text-only slides
- **Accent lines under titles** — hallmark of AI-generated slides; use whitespace or background color instead
- Low-contrast icons or text

---

## QA (Required)

Assume there are problems. Your job is to find them.

### Content QA

```bash
python -m markitdown output.pptx
# Check: missing content, wrong order, typos

# Check for placeholders
python -m markitdown output.pptx | grep -iE "xxxx|lorem|ipsum|this.*(page|slide).*layout"
# Any result → fix before declaring success
```

### Visual QA

```bash
# 1. Convert to PDF
soffice --headless --convert-to pdf output.pptx

# 2. Render to images
pdftoppm -jpeg -r 150 output.pdf slide
# Output: slide-01.jpg, slide-02.jpg, ...

# Re-render specific slides (N to N) after fixes
pdftoppm -jpeg -r 150 -f N -l N output.pdf slide-fixed
```

Inspect via subagent with fresh eyes — even on 2-3 slides. Prompt:

```
Visually inspect these slides. Assume there are issues — find them.

Look for:
- Overlapping elements (text through shapes, stacked elements)
- Text overflow or cut off at edges/box boundaries
- Decorative lines sized for single-line titles that wrapped to two lines
- Footers/citations colliding with content above
- Elements too close (< 0.3") or nearly touching
- Uneven gaps (large empty area vs cramped)
- Insufficient margins from edges (< 0.5")
- Misaligned columns or similar elements
- Low-contrast text or icons
- Text boxes too narrow causing excessive wrapping
- Leftover placeholder content

For each slide, list issues or concerns, even if minor.

Images:
1. /path/to/slide-01.jpg (Expected: [brief description])
2. /path/to/slide-02.jpg (Expected: [brief description])
```

### Verification Loop

1. Generate → convert to images → inspect
2. List issues (if none found, look again more critically)
3. Fix issues
4. Re-verify affected slides — one fix often creates another problem
5. Repeat until a full pass finds no new issues

Do not declare success until at least one fix-and-verify cycle is complete.

---

## Dependencies

```bash
pip install "markitdown[pptx]" defusedxml
npm install -g pptxgenjs          # optional
# System: install LibreOffice and Poppler for visual QA
```
