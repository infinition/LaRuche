---
type: skill
name: deep_research_synthesis
description: >-
  Research a question across web and academic sources and produce a cited report.
---

# Deep research and synthesis

Multi-pass research across web and academic sources (arXiv, PDFs), with adversarial
cross-check, producing a cited report saved to file and memory.

## 0. Declare the mission first

`research_mode` with `mode` set, and a short `reason`. Call it BEFORE the first search,
as soon as the user asks for something thorough, deep or exhaustive, or as soon as you
realise a single lookup will not be enough. It activates the long-running research
behaviour: budgets, pacing, and the right expectations about how many passes are allowed.

Declaring it late is the usual mistake. Three shallow searches happen first, the answer
is thin, and the mode is switched on when the work is nearly over.

Then check what you already know before spending anything:

- `memory_search` on the subject: this may have been researched before.
- `session_search` with `query` runs a full-text search across PAST sessions and returns
  matching excerpts. Use it when the user says "we looked at this already" or when the
  subject feels familiar. It is far cheaper than repeating the research.

## 1. Scope Definition

Before any search, lock the question:
- One-sentence research question.
- 3–5 domain keywords (e.g. "World Models", "RLHF", "in-context learning").
- Target: paper count (5–15), recency window (default: 18 months), output format (inline / file / memory).

## 2. Search - Broad then Narrow

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
# <Topic> - Research Report (<YYYY-MM-DD>)

## Overview
<2–3 sentence summary>

## Key Findings
- <finding> [Source: <title>, <url>]

## Conflicts & Uncertainties
- <claim A> vs <claim B> - unresolved

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
- **arXiv versioning**: check submission date AND latest version - v1 vs vN can differ significantly.
- **Query vagueness**: include domain terms + year; generic queries return noise.
- **Snippet trap**: `web_search` returns snippets - always follow up with `web_fetch` before citing.
- **`web_deep_search` cost**: use only for sub-questions where shallow search fails; it is slower.
