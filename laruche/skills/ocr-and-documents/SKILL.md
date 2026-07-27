---
type: skill
name: ocr-and-documents
description: Extract text from a PDF or a scanned document, with or without OCR.
---

# PDF & Document Extraction

Get the text out of a PDF, a scan, or a document you cannot read directly. This skill
covers PDFs and scanned documents. For DOCX, use `python-docx` through `execute_code`: it
parses the real document structure and beats OCR on every axis. For PPTX, `python-pptx`
does the same, slides and speaker notes included.

## Step 0: the file is already local

`read_extract` with `path` reads a PDF, a `.txt` or a `.md` from disk and returns its
text. One call, no install, no dependency. It is the first thing to try on a local file,
and the only extractor that is always available.

Its limits, which decide whether you need the rest of this skill:

- It takes `path`, never a URL. Passing a URL returns `File not found`.
- Use an ABSOLUTE path. A relative one resolves against the server's working directory.
- It does no OCR. On a scanned PDF, whose pages are images, it returns almost nothing.
  Near-empty output on a PDF that clearly has words in it means "this is a scan", not
  "this file is empty". Go to marker-pdf.
- Long documents come back as a head and a tail, with the middle dropped. For the whole
  text of a long PDF, use pymupdf below.

## Step 1: the document is behind a URL

`web_fetch` with `url` already extracts a PDF served over HTTP, with no local
dependencies. It takes ONE url, as a string.

```
web_fetch(url="https://arxiv.org/pdf/2402.03300")
web_fetch(url="https://example.com/report.pdf")
```

Fall back to local extraction when the file is local, when `web_fetch` fails or returns
boilerplate, or when you have a batch to process. To download first and read from disk,
see the `web-research` skill.

## Step 2: Choose Local Extractor

| Feature | pymupdf (~25MB) | marker-pdf (~3-5GB) |
|---------|-----------------|---------------------|
| Text-based PDF | ✅ | ✅ |
| Scanned PDF (OCR) | ❌ | ✅ (90+ languages) |
| Tables | ✅ (basic) | ✅ (high accuracy) |
| Equations / LaTeX | ❌ | ✅ |
| Code blocks | ❌ | ✅ |
| Forms | ❌ | ✅ |
| Headers/footers removal | ❌ | ✅ |
| Reading order detection | ❌ | ✅ |
| Images → text (OCR) | ❌ | ✅ |
| EPUB | ✅ | ✅ |
| Markdown output | ✅ (via pymupdf4llm) | ✅ (native, higher quality) |
| Speed | Instant | ~1-14s/page CPU, ~0.2s/page GPU |

**Decision**: Use pymupdf unless you need OCR, equations, forms, or complex layout analysis.

If marker is needed but disk is tight:
> "This document needs OCR/advanced extraction (marker-pdf), which requires ~5GB for PyTorch and models. Your system has [X]GB free. Options: free up space, provide a URL for `web_fetch`, or fall back to pymupdf (works for text-based PDFs only)."

---

## pymupdf (lightweight)

```bash
pip install pymupdf pymupdf4llm
```

**Via helper script** (`shell_exec`):
```bash
python scripts/extract_pymupdf.py document.pdf              # Plain text
python scripts/extract_pymupdf.py document.pdf --markdown    # Markdown
python scripts/extract_pymupdf.py document.pdf --tables      # Tables
python scripts/extract_pymupdf.py document.pdf --images out/ # Extract images
python scripts/extract_pymupdf.py document.pdf --metadata    # Title, author, pages
python scripts/extract_pymupdf.py document.pdf --pages 0-4   # Specific pages
```

**Inline** (`execute_code`):
```python
import pymupdf
doc = pymupdf.open("document.pdf")
for page in doc:
    print(page.get_text())
```

---

## marker-pdf (high-quality OCR)

```bash
# Check disk space first
python scripts/extract_marker.py --check

pip install marker-pdf
```

**Via helper script** (`shell_exec`):
```bash
python scripts/extract_marker.py document.pdf                # Markdown
python scripts/extract_marker.py document.pdf --json         # JSON with metadata
python scripts/extract_marker.py document.pdf --output_dir out/  # Save images
python scripts/extract_marker.py scanned.pdf                 # Scanned PDF (OCR)
python scripts/extract_marker.py document.pdf --use_llm      # LLM-boosted accuracy
```

**CLI** (installed with marker-pdf):
```bash
marker_single document.pdf --output_dir ./output
marker /path/to/folder --workers 4    # Batch processing
```

---

## Arxiv Papers

```
# Abstract only (fast)
web_fetch(url="https://arxiv.org/abs/2402.03300")

# Full paper
web_fetch(url="https://arxiv.org/pdf/2402.03300")

# Search
web_search(query="arxiv GRPO reinforcement learning 2026")
```

To search arXiv properly, by author, category or date, open the `arxiv` skill.

## Split, Merge & Search

Use `execute_code` with pymupdf - no extra dependencies:

```python
# Split: extract pages 1-5 to a new PDF
import pymupdf
doc = pymupdf.open("report.pdf")
new = pymupdf.open()
for i in range(5):
    new.insert_pdf(doc, from_page=i, to_page=i)
new.save("pages_1-5.pdf")
```

```python
# Merge multiple PDFs
import pymupdf
result = pymupdf.open()
for path in ["a.pdf", "b.pdf", "c.pdf"]:
    result.insert_pdf(pymupdf.open(path))
result.save("merged.pdf")
```

```python
# Search for text across all pages
import pymupdf
doc = pymupdf.open("report.pdf")
for i, page in enumerate(doc):
    hits = page.search_for("revenue")
    if hits:
        print(f"Page {i+1}: {len(hits)} match(es)")
        print(page.get_text("text"))
```

---

## Traps

- **A scanned PDF is not an empty PDF.** `read_extract` and pymupdf both return a few
  stray characters from a scan, because there is no text layer to read. Judge by the page
  count against the character count: 40 pages and 200 characters means OCR is required.
- **`read_extract` truncates the middle of a long document** and says so. If you need a
  passage from the middle, use pymupdf with `--pages`.
- **marker-pdf downloads about 2.5GB of models** to `~/.cache/huggingface/` on first use,
  on top of the PyTorch install. Run `python scripts/extract_marker.py --check` BEFORE
  suggesting it, and tell the user the cost before spending their disk.
- **`--use_llm` sends page content to a model provider.** Do not reach for it on a
  document the user has treated as confidential without asking first.
- **Page ranges are 0-based.** `--pages 0-4` is the first five pages, not pages one to
  four.

## Failure modes

**Extraction returns almost nothing from a PDF with visible text.** It is a scan, or the
text layer is an image. Confirm the page count with
`python scripts/extract_pymupdf.py file.pdf --metadata`, then use marker-pdf.

**`pip install marker-pdf` fails or fills the disk.** It pulls PyTorch. Check free space
first with `--check`. If the space is not there, say so and fall back to pymupdf, which
handles any PDF that has a real text layer.

**The text comes out in the wrong order, columns interleaved.** pymupdf reads in the
PDF's internal order, which for a two-column paper is not reading order. marker-pdf does
reading-order detection; use it for papers and magazines.

**Tables arrive as a wall of numbers.** pymupdf's `--tables` is basic. For tables that
carry the answer, use marker-pdf.

Both helper scripts accept `--help` for their full usage.
