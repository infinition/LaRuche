---
type: skill
name: arxiv
description: >-
  Search arXiv + Semantic Scholar: papers, citations, BibTeX, via curl.
---

# arXiv Research

Search and retrieve academic papers from arXiv via their free REST API. No API key, no dependencies - just curl and Python stdlib. Augment with Semantic Scholar for citations.

## Quick Reference

| Action | Command |
|--------|---------|
| Search (clean output) | `python scripts/search_arxiv.py "QUERY"` |
| Get specific paper | `curl "https://export.arxiv.org/api/query?id_list=2402.03300"` |
| Read abstract | `read_extract(urls=["https://arxiv.org/abs/2402.03300"])` |
| Read full paper (PDF) | `read_extract(urls=["https://arxiv.org/pdf/2402.03300"])` |

## Helper Script

`scripts/search_arxiv.py` parses arXiv Atom XML and prints clean results. No third-party deps.

```bash
python scripts/search_arxiv.py "GRPO reinforcement learning"
python scripts/search_arxiv.py "transformer attention" --max 10 --sort date
python scripts/search_arxiv.py --author "Yann LeCun" --max 5
python scripts/search_arxiv.py --category cs.AI --sort date
python scripts/search_arxiv.py --id 2402.03300
python scripts/search_arxiv.py --id 2402.03300,2401.12345
```

## Search Query Syntax

| Prefix | Searches | Example |
|--------|----------|---------|
| `all:` | All fields | `all:transformer+attention` |
| `ti:` | Title | `ti:large+language+models` |
| `au:` | Author | `au:vaswani` |
| `abs:` | Abstract | `abs:reinforcement+learning` |
| `cat:` | Category | `cat:cs.AI` |
| `co:` | Comment | `co:accepted+NeurIPS` |

Boolean operators: `+AND+`, `+OR+`, `+ANDNOT+`. Exact phrase: `ti:"chain+of+thought"`.

## Sort and Pagination

| Parameter | Options |
|-----------|---------|
| `sortBy` | `relevance`, `lastUpdatedDate`, `submittedDate` |
| `sortOrder` | `ascending`, `descending` |
| `start` | Result offset (0-based) |
| `max_results` | Default 10, max 30000 |

```bash
# Latest 10 papers in cs.AI
curl -s "https://export.arxiv.org/api/query?search_query=cat:cs.AI&sortBy=submittedDate&sortOrder=descending&max_results=10"
```

## Fetching Specific Papers

```bash
# Single or multiple by arXiv ID
curl -s "https://export.arxiv.org/api/query?id_list=2402.03300,2401.12345,2403.00001"
```

## BibTeX Generation

```bash
curl -s "https://export.arxiv.org/api/query?id_list=1706.03762" | python3 -c "
import sys, xml.etree.ElementTree as ET
ns = {'a': 'http://www.w3.org/2005/Atom', 'arxiv': 'http://arxiv.org/schemas/atom'}
root = ET.parse(sys.stdin).getroot()
entry = root.find('a:entry', ns)
if entry is None: sys.exit('Paper not found')
title = entry.find('a:title', ns).text.strip().replace('\n', ' ')
authors = ' and '.join(a.find('a:name', ns).text for a in entry.findall('a:author', ns))
year = entry.find('a:published', ns).text[:4]
raw_id = entry.find('a:id', ns).text.strip().split('/abs/')[-1]
cat = entry.find('arxiv:primary_category', ns)
primary = cat.get('term') if cat is not None else 'cs.LG'
last_name = entry.find('a:author', ns).find('a:name', ns).text.split()[-1]
print(f'@article{{{last_name}{year}_{raw_id.replace(\".\", \"\")},')
print(f'  title     = {{{title}}},')
print(f'  author    = {{{authors}}},')
print(f'  year      = {{{year}}},')
print(f'  eprint    = {{{raw_id}}},')
print(f'  archivePrefix = {{arXiv}},')
print(f'  primaryClass  = {{{primary}}},')
print(f'  url       = {{https://arxiv.org/abs/{raw_id}}}')
print('}')
"
```

## Common Categories

`cs.AI` Artificial Intelligence | `cs.CL` NLP | `cs.CV` Computer Vision | `cs.LG` Machine Learning | `cs.CR` Security | `stat.ML` ML/Statistics | `math.OC` Optimization

Full taxonomy: https://arxiv.org/category_taxonomy

---

## Semantic Scholar (Citations, Related Papers, Author Profiles)

arXiv has no citation data. Semantic Scholar API is free, no key for basic use (1 req/sec), returns JSON.

```bash
# Paper details + citation counts
curl -s "https://api.semanticscholar.org/graph/v1/paper/arXiv:2402.03300?fields=title,authors,citationCount,referenceCount,influentialCitationCount,year,abstract" | python3 -m json.tool

# Who cited this paper
curl -s "https://api.semanticscholar.org/graph/v1/paper/arXiv:2402.03300/citations?fields=title,authors,year,citationCount&limit=10" | python3 -m json.tool

# What this paper cites
curl -s "https://api.semanticscholar.org/graph/v1/paper/arXiv:2402.03300/references?fields=title,authors,year,citationCount&limit=10" | python3 -m json.tool

# Paper search (JSON alternative to arXiv search)
curl -s "https://api.semanticscholar.org/graph/v1/paper/search?query=GRPO+reinforcement+learning&limit=5&fields=title,authors,year,citationCount,externalIds" | python3 -m json.tool

# Paper recommendations
curl -s -X POST "https://api.semanticscholar.org/recommendations/v1/papers/" \
  -H "Content-Type: application/json" \
  -d '{"positivePaperIds": ["arXiv:2402.03300"], "negativePaperIds": []}' | python3 -m json.tool

# Author profile
curl -s "https://api.semanticscholar.org/graph/v1/author/search?query=Yann+LeCun&fields=name,hIndex,citationCount,paperCount" | python3 -m json.tool
```

Useful fields: `title`, `authors`, `year`, `abstract`, `citationCount`, `influentialCitationCount`, `isOpenAccess`, `openAccessPdf`, `fieldsOfStudy`, `publicationVenue`, `externalIds` (arXiv ID, DOI).

---

## Complete Research Workflow

1. **Discover**: `python scripts/search_arxiv.py "your topic" --sort date --max 10`
2. **Assess impact**: `curl -s "https://api.semanticscholar.org/graph/v1/paper/arXiv:ID?fields=citationCount,influentialCitationCount"`
3. **Read abstract**: `read_extract(urls=["https://arxiv.org/abs/ID"])`
4. **Read full paper**: `read_extract(urls=["https://arxiv.org/pdf/ID"])`
5. **Find related work**: `curl -s "https://api.semanticscholar.org/graph/v1/paper/arXiv:ID/references?fields=title,citationCount&limit=20"`
6. **Get recommendations**: POST to Semantic Scholar recommendations endpoint (see above)
7. **Track authors**: `curl -s "https://api.semanticscholar.org/graph/v1/author/search?query=NAME"`

## Rate Limits

| API | Rate | Auth |
|-----|------|------|
| arXiv | ~1 req / 3 seconds | None |
| Semantic Scholar | 1 req/sec | None (100/sec with API key) |

## Notes

- arXiv returns Atom XML - use the helper script or inline parsing for clean output.
- Semantic Scholar returns JSON - pipe through `python3 -m json.tool`.
- arXiv IDs: old format (`hep-th/0601001`) vs new (`2402.03300`).
- URL patterns: abstract → `arxiv.org/abs/{id}` | PDF → `arxiv.org/pdf/{id}` | HTML (when available) → `arxiv.org/html/{id}`.
- For local PDF processing, see the `ocr-and-documents` skill.

## ID Versioning

- `arxiv.org/abs/1706.03762` resolves to the **latest** version.
- `arxiv.org/abs/1706.03762v1` pins a **specific** immutable version.
- When citing, preserve the version suffix you actually read - later versions may change content substantially.
- The API `<id>` field returns the versioned URL (e.g., `http://arxiv.org/abs/1706.03762v7`).

## Withdrawn Papers

- The `<summary>` field contains a withdrawal notice (look for "withdrawn" or "retracted").
- Always check the summary before treating a result as valid.
