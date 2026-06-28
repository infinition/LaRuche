---
type: skill
name: nano-pdf
description: "Edit PDF text/typos/titles via natural-language instructions (nano-pdf CLI)."
version: 1.0.0
author: community
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec]
metadata:
  laruche:
    tags: [PDF, Documents, Editing, NLP, Productivity]
    homepage: https://pypi.org/project/nano-pdf/
---

# nano-pdf

Edit PDFs using natural-language instructions. Point it at a page and describe what to change.

## Prerequisites

```bash
# Install with uv (recommended)
uv pip install nano-pdf

# Or with pip
pip install nano-pdf
```

The tool uses an LLM under the hood - run `nano-pdf --help` to see how to configure your API key.

## Usage

```bash
nano-pdf edit <file.pdf> <page_number> "<instruction>"
```

## Examples

```bash
# Change a title on page 1
nano-pdf edit deck.pdf 1 "Change the title to 'Q3 Results' and fix the typo in the subtitle"

# Update a date on a specific page
nano-pdf edit report.pdf 3 "Update the date from January to February 2026"

# Fix content
nano-pdf edit contract.pdf 2 "Change the client name from 'Acme Corp' to 'Acme Industries'"
```

## Notes

- Page numbers may be 0-based or 1-based depending on version - if the edit hits the wrong page, retry with ±1
- Always verify the output PDF after editing (check file size with `shell_exec`, or open it)
- Works well for text changes; complex layout modifications may require a different approach
