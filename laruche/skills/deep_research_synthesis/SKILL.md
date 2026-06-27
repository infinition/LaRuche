---
type: skill
name: deep_research_synthesis
description: Multi-pass web/academic research → cited structured report.
tools: [web_search, web_deep_search, web_fetch, read_extract, file_write, memory_write]
---

# Deep Research & Synthesis

Multi-pass research across web and academic sources (arXiv, PDFs), with adversarial cross-check, producing a cited report saved to file and memory.

## 1. Scope Definition

Before any search, lock the question:
- One-sentence research question.
- 3–5 domain keywords (e.g. "World Models", "RLHF", "in-context learning").
- Target: paper count (5–15), recency window (default: 18 months), output format (inline / file / memory).

## 2. Search — Broad then Narrow

```
web_search("<keywords> site:arxiv.org OR filetype:pdf", n=10)
web_search("<topic> review survey 2025", n=10)
web_deep_search("<specific sub-question>")          # deeper pass on key sub-topics
```

Collect: arXiv IDs, paper titles, blog posts, official docs URLs. De-duplicate immediately.

## 3. Fetch & Extract

For each source retained:

```
web_fetch("https://arxiv.org/abs/<id>")             # abstract + metadata
web_fetch("https://arxiv.org/pdf/<id>.pdf")         # full text when needed
read_extract("<url>", fields=["abstract","contributions","results","limitations"])
```

Discard: no date, clearly outdated claims, preprints with no citations and > 2 years old.

## 4. Adversarial Cross-Check

For each major claim:
- Find at least one corroborating source OR note it as unverified.
- Flag conflicts explicitly: "Source A claims X; Source B contradicts with Y."
- If a key fact is uncorroborated, mark `[UNVERIFIED]` in the report.

## 5. Write Report

```markdown
# <Topic> — Research Report (<YYYY-MM-DD>)

## Overview
<2–3 sentence summary>

## Key Findings
- <finding> [Source: <title>, <url>]

## Conflicts & Uncertainties
- <claim A> vs <claim B> — unresolved

## Sources & Limitations
| # | Title | URL | Date |

## Open Questions
```

Save:

```
file_write("research/<topic>_report.md", content)
memory_write(key="research/<topic>", value=summary_2_sentences)
```

## Pitfalls

- **Abstracts only**: always `read_extract` or `web_fetch` the full paper for load-bearing claims.
- **arXiv versioning**: check submission date AND latest version — v1 vs vN can differ significantly.
- **Query vagueness**: include domain terms + year; generic queries return noise.
- **Snippet trap**: `web_search` returns snippets — always follow up with `web_fetch` before citing.
- **`web_deep_search` cost**: use only for sub-questions where shallow search fails; it is slower.
