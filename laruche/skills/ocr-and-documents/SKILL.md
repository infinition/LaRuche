---
type: skill
name: ocr-and-documents
description: >-
  Extract text from PDFs/scans: pymupdf (fast) or marker-pdf (OCR).
---

# PDF & Document Extraction

For DOCX: use `python-docx` (parses actual document structure, far better than OCR).
For PPTX: see the `powerpoint` skill (`python-pptx` with full slide/notes support).
This skill covers **PDFs and scanned documents**.

## Step 1: Remote URL Available?

If the document has a URL, **always try `web_fetch` first** - no local dependencies:

```
web_fetch(urls=["https://arxiv.org/pdf/2402.03300"])
web_fetch(urls=["https://example.com/report.pdf"])
```

Fall back to local extraction only when: the file is local, `web_fetch` fails, or batch processing is needed.

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
web_fetch(urls=["https://arxiv.org/abs/2402.03300"])

# Full paper
web_fetch(urls=["https://arxiv.org/pdf/2402.03300"])

# Search
web_search(query="arxiv GRPO reinforcement learning 2026")
```

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

## Notes

- marker-pdf downloads ~2.5GB of models to `~/.cache/huggingface/` on first use
- Both helper scripts accept `--help` for full usage
