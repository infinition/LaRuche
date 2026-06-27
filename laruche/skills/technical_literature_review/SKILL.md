---
type: skill
name: technical_literature_review
description: Deep academic literature review → structured Markdown report (arXiv, top venues).
tools: [web_search, web_fetch, read_extract, file_write, memory_write]
---

# Technical Literature Review

Turns a technical research query into a structured synthesis grounded in academic sources (arXiv, NeurIPS, ICML, ICLR, etc.).

## Step 1 — Source Identification

Run multiple targeted searches combining the topic with venue signals and paper type:

```
web_search("<topic> arXiv survey 2024 2025")
web_search("<topic> site:arxiv.org")
web_search("<topic> NeurIPS ICML ICLR 2024")
web_search("<topic> overview review benchmark")
```

Prioritize:
- Survey / review papers ("survey", "overview", "tutorial" in title)
- Recent high-citation papers from arXiv cs.LG, cs.AI, cs.CV, cs.CL
- Papers from top venues (NeurIPS, ICML, ICLR, CVPR, ACL, EMNLP)

Collect 5–10 candidate URLs. If a topic is broad, narrow with subtopic searches before proceeding.

## Step 2 — Content Retrieval and Extraction

For each selected paper, prefer the abstract+HTML page over the raw PDF:

```
read_extract("https://arxiv.org/abs/<paper-id>")   # structured extraction, preferred
web_fetch("https://arxiv.org/html/<paper-id>")      # fallback: full HTML version
web_fetch("https://arxiv.org/abs/<paper-id>")       # fallback: abstract page
```

**Do not rely on search snippets.** Fetch the full page to get section-level detail.

Tag extracted content by type:
- **Definitions** — core concepts and formal definitions (use the paper's own wording)
- **Taxonomy** — how the field classifies approaches (dimensions, families, types)
- **Challenges / gaps** — open problems explicitly stated by the authors
- **Applications** — major use cases, benchmarks, datasets
- **Key references** — seminal works cited repeatedly across sources

If memory persistence is needed across a long run, checkpoint with:
```
memory_write("<topic>_lit_review_sources", "<list of collected references and tags>")
```

## Step 3 — Synthesis and Report

Assemble a structured Markdown report incrementally (draft each section as sources are processed, then consolidate):

```markdown
# [Topic] — Literature Review

## 1. Definition and Scope
## 2. Taxonomy
## 3. Key Approaches / Methods
## 4. Challenges and Open Problems
## 5. Applications and Benchmarks
## 6. Future Directions
## 7. Sources
```

Rules:
- Every claim must be traceable to a cited source (author, year, arXiv ID or URL).
- Do not mix concepts from different sources without explicit attribution.
- Keep definitions tight; prefer the paper's own wording with a brief paraphrase.
- The Sources section must list full references: Author(s), Title, Venue/arXiv ID, Year, URL.

Save the final report:
```
file_write("<topic>_literature_review.md", "<report content>")
```

## Failure Handling

| Problem | Fix |
|---|---|
| `read_extract` returns empty | Fall back to `web_fetch` on the `/html/` or `/abs/` URL |
| Paper behind paywall | Try Semantic Scholar: `web_fetch("https://api.semanticscholar.org/graph/v1/paper/search?query=<topic>")` |
| Too many results, hard to rank | Add `site:arxiv.org` + `"survey"` or filter by year in the query |
| Topic too broad | Break into 2–3 subtopics, run separate searches, merge in Step 3 |
