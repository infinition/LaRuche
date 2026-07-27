---
type: skill
name: ocr-and-documents
description: Extract text from a PDF or a scanned document, with or without OCR.
---

# Getting text out of documents

A PDF is not a text file. It is a set of drawing instructions that usually, but not
always, include the characters. When they do, extraction is instant and free. When they do
not, because the page is a photograph of paper, no amount of retrying helps and you need
optical recognition, which costs gigabytes.

So the first question is never "which tool", it is **does this file contain text at all**.

## The ladder, cheapest rung first

| Rung | Tool | Cost | Handles |
|---|---|---|---|
| 0 | `read_extract` | nothing | a local PDF, `.txt` or `.md` with a text layer |
| 1 | `web_fetch` | nothing | a document behind a URL, PDF included |
| 2 | pymupdf | 25 MB | full text, page ranges, basic tables, images, metadata |
| 3 | marker-pdf | 3 to 5 GB | scans, equations, columns, real tables |

Never start at rung 3. Most documents are answered at rung 0.

## Rung 0: the file is on disk

`read_extract` with `path`. One call, no install, always available.

```
read_extract(path="C:/dev/docs/report.pdf")
```

Its limits are exactly what send you up the ladder:

- `path` only. Given a URL it answers `File not found`; it has no network at all.
- Use an ABSOLUTE path. A relative one resolves against the server's working directory.
- No OCR. On a scan it returns almost nothing.
- A long document comes back as a head and a tail, with the middle dropped. For a passage
  in the middle, go to pymupdf with `--pages`.

## Rung 1: the document is behind a URL

`web_fetch` with `url` extracts a PDF served over HTTP by itself, with nothing installed.
It takes ONE url, as a string.

```
web_fetch(url="https://example.com/annual-report.pdf")
```

If the server refuses it, download it and drop back to rung 0:

```bash
curl -sL "https://example.com/annual-report.pdf" -o "C:\tmp\report.pdf"
```

## Is it a scan? Decide before installing anything

This is the whole diagnosis, and it takes one command:

```bash
python skills/ocr-and-documents/scripts/extract_pymupdf.py document.pdf --metadata
```

Compare the page count against the characters you got at rung 0. **Forty pages and two
hundred characters means the pages are images.** It does not mean the document is empty,
and it does not mean the extraction failed. Nothing below rung 3 will ever read it.

A document with a PARTIAL text layer, common in scanned paperwork carrying a searchable
overlay, is worse: it gives you text that is present and wrong. Plausible words in an
implausible order is the signature. Check one page against the original before trusting
any of it.

## Rung 2: pymupdf

```bash
pip install pymupdf pymupdf4llm
```

```bash
python skills/ocr-and-documents/scripts/extract_pymupdf.py doc.pdf              # plain text
python skills/ocr-and-documents/scripts/extract_pymupdf.py doc.pdf --markdown   # structure kept
python skills/ocr-and-documents/scripts/extract_pymupdf.py doc.pdf --pages 0-4  # first five pages
python skills/ocr-and-documents/scripts/extract_pymupdf.py doc.pdf --tables
python skills/ocr-and-documents/scripts/extract_pymupdf.py doc.pdf --images out/
python skills/ocr-and-documents/scripts/extract_pymupdf.py doc.pdf --metadata
```

`--pages` is **0-based**: `0-4` is the first five pages, not pages one to four.

Reading order is the PDF's internal order, which for a two-column paper is not the order a
human reads it in. Text arriving interleaved between columns is that limitation, and it is
a reason to climb to rung 3 rather than a bug to work around.

For anything programmatic, `execute_code` with pymupdf directly beats the script:
splitting, merging, searching across pages.

```python
import pymupdf
doc = pymupdf.open("C:/dev/docs/report.pdf")
for number, page in enumerate(doc, 1):
    hits = page.search_for("revenue")
    if hits:
        print(number, len(hits))
```

## Rung 3: marker-pdf

Real OCR, layout analysis, reading order, equations, and tables that survive.

**Check the cost before proposing it**, and give the user the number:

```bash
python skills/ocr-and-documents/scripts/extract_marker.py --check
```

It reports free disk. marker-pdf pulls PyTorch, then downloads roughly 2.5 GB of models
into `~/.cache/huggingface/` on first use. That is a real cost on someone's laptop, and
spending it is their decision.

```bash
pip install marker-pdf

python skills/ocr-and-documents/scripts/extract_marker.py scanned.pdf
python skills/ocr-and-documents/scripts/extract_marker.py doc.pdf --json
python skills/ocr-and-documents/scripts/extract_marker.py doc.pdf --output_dir out/
python skills/ocr-and-documents/scripts/extract_marker.py doc.pdf --use_llm
```

It also reads DOCX, PPTX, XLSX, HTML, EPUB and plain images.

**`--use_llm` sends page content to a model provider.** Do not reach for it on a document
the user has treated as confidential without asking first.

When the disk is not there, say so and give the alternatives plainly: free some space,
provide a URL for rung 1, or accept pymupdf's output knowing it cannot read a scan.

## Formats that are not PDFs

OCR is the wrong tool for a format that already carries its structure. Use the parser:

- **DOCX**: `python-docx` through `execute_code`. Paragraphs, styles and tables as
  authored.
- **PPTX**: `python-pptx`. Slides and speaker notes.
- **XLSX**: `openpyxl`, or `pandas.read_excel`.

Running OCR over a DOCX throws the structure away and then guesses it back.

## Traps

- **An almost-empty result is a diagnosis, not a failure.** See the scan check above.
- **`--pages` counts from zero.**
- **A table extracted as prose is not a table.** pymupdf's `--tables` is basic. If the
  answer lives in a table, climb to marker-pdf instead of reconstructing rows by eye.
- **Ligatures and hyphens.** Extracted text contains real ligature characters, and a word
  broken across two lines keeps its hyphen. Normalise before matching on a string, or the
  search silently finds nothing and you conclude the term is absent.
- **Headers, footers and page numbers land in the text**, mid-sentence, at every page
  break. Strip them before summarising or they get quoted as content.
- **Never install marker-pdf silently.** Several gigabytes without being asked is not a
  detail.

## Failure modes

**Almost no text from a PDF with visible words.** It is a scan, or the text layer is an
image. Confirm with `--metadata`, then rung 3.

**`pip install marker-pdf` fails or fills the disk.** It pulls PyTorch. Run `--check`
first. Without the space, say so and fall back to pymupdf for any document that does have
a text layer.

**Columns interleaved, sentences from two places mixed together.** pymupdf reading order.
marker-pdf does reading-order detection; use it for papers and magazines.

**The text is there but full of question marks or boxes.** An encoding or embedded-font
problem. Try `--markdown`, then marker-pdf, which re-recognises the glyphs rather than
trusting the font's character map.

**The document is encrypted.** pymupdf raises on open. If the user has the password,
`pymupdf.open(path)` then `doc.authenticate(password)` through `execute_code`. If they do
not, stop: that is not something to work around.

**It worked on page one and produced nothing after.** Either a `--pages` range is still
set, or the document changes format partway, which is normal in a text-layer report
carrying scanned appendices. Extract the sections separately.
