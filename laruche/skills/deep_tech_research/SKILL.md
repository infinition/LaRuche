---
type: skill
name: deep_tech_research
description: Deep-dive tech trend research: search, extract, synthesize, cite.
tools: [web_search, web_deep_search, web_fetch, read_extract, memory_write]
---

# Deep Tech Research

## Purpose
Produce a structured, cited analysis of future trends in a given tech domain.
Input: a domain or question (e.g. "AI agents 2026", "quantum computing roadmap").

## Steps

### 1. Source Discovery
Run 3–5 targeted queries via `web_search`. If results are thin, escalate to `web_deep_search`.

Target high-authority sources: research labs, analyst firms (Gartner, McKinsey, IDC), major tech vendors, peer-reviewed outlets, government roadmaps.

Example queries for "AI agents":
- `"AI agents trends 2026 site:gartner.com OR site:mckinsey.com"`
- `"agentic AI roadmap 2025 2026 forecast"`
- `"multi-agent systems research 2026"`

Collect 6–10 candidate URLs. Drop product landing pages, press releases, and articles with no data or forecasts.

### 2. Content Extraction
For the 4–6 most relevant URLs, extract full content - never rely on snippets:

```
read_extract(url)          # preferred: clean article text, strips nav/ads
web_fetch(url, render=true) # fallback for JS-rendered pages
```

Use `read_extract` first; fall back to `web_fetch(render=true)` if the page is JavaScript-heavy or `read_extract` returns truncated content.

### 3. Analysis
Scan extracted content for:
- **Recurring themes** across sources (convergence = higher confidence)
- **Concrete timelines and forecasts** with named sources
- **Conflicting claims** - note disagreements explicitly and flag which sources disagree
- **Forecasts older than 18 months** - mark as potentially stale

### 4. Persist Key Findings (optional)
If the research will be referenced later, store a summary:

```
memory_write(key="research/<domain>/<YYYY-MM>", value="<summary + source list>")
```

### 5. Final Report
Output structure:

```markdown
## [Domain] - Future Trends Report

### Key Trends
1. [Trend] - [Evidence] - Source: [Title](URL)
2. ...

### Emerging Risks / Open Questions
- ...

### Conflicting Views
- [Claim A] (Source X) vs [Claim B] (Source Y) - unresolved

### Sources
| Title | URL | Authority | Date |
|-------|-----|-----------|------|
| ...   | ... | high/med  | ...  |
```

## Failure Handling
- **No results from `web_search`**: switch to `web_deep_search` with broader or alternate phrasing.
- **`read_extract` returns empty**: fall back to `web_fetch(render=true)`.
- **Paywall / access denied**: skip that source, note it in the report, find an alternate.
- **All sources are older than 18 months**: flag the report as potentially stale; recommend re-running in 3 months.
