---
type: skill
name: deep_academic_research
description: Multi-source academic research: web, arXiv, Semantic Scholar, structured report.
tools: [web_search, web_deep_search, web_fetch, read_extract, file_write, memory_write]
---

# Deep Academic Research Procedure

## Step 1 - Scope Definition

Before any search, pin down:
- Primary topic + 3–5 core keywords
- Depth target: overview / state-of-the-art / comparative survey
- Output: inline answer or persisted file

## Step 2 - Broad Web Search (Foundation)

```
web_search("<topic> overview introduction")
web_search("<topic> history key milestones")
```

Use `web_fetch` on the most cited or authoritative pages to get full content beyond snippets. Use `read_extract` on PDFs or paper pages to pull structured text.

## Step 3 - Academic Literature (Depth)

Prioritize peer-reviewed sources. Append qualifiers:

```
web_search("<topic> arXiv survey 2024 2025")
web_search("<topic> site:arxiv.org OR site:semanticscholar.org")
web_search("<topic> benchmark state of the art NeurIPS ICML ACL IEEE ACM")
```

If standard `web_search` returns weak academic results, escalate to:

```
web_deep_search("<topic> recent preprint survey")
```

For each relevant paper: `web_fetch` the abstract/PDF page, then `read_extract` for methodology, results, citations.

**Citation chaining**: from a key paper, extract its references and search for the 2–3 most cited ones.

## Step 4 - Synthesis and Report

Structure the report:

1. **Definition** - precise scope, key terms
2. **History** - origins, major milestones
3. **State of the Art** - leading methods, benchmarks, key papers (with links)
4. **Open Challenges & Trends** - unsolved problems, emerging directions

Persist with:

```
file_write("research_<topic>_<YYYYMMDD>.md", <report_content>)
```

Store a compact summary for future recall:

```
memory_write("deep_research/<topic>", "<key findings + top references>")
```

## Pitfalls & Failure Handling

- Never rely on snippets alone - always `web_fetch` + `read_extract` key sources.
- Distinguish blogs/news from peer-reviewed evidence (arXiv, ACL, NeurIPS, IEEE, ACM).
- If `web_fetch` on an arXiv PDF URL fails, try the `/abs/` page instead of `/pdf/`.
- If results are thin, try alternative phrasings: "deep learning X", "X neural network", "X transformer", "X survey paper".
- Cross-check any claim that appears in only one source before including it.
- Critical synthesis only - no copy-paste aggregation.
