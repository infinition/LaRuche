---
type: skill
name: arxiv-search
description: "arXiv + Semantic Scholar: search papers, citations, BibTeX via curl."
tools: [shell_exec, read_extract]
scripts: []
---

# arXiv Research

Search and retrieve academic papers from arXiv via their free REST API. No API key, no dependencies - just curl and Python stdlib.

## Quick Reference

| Action | Command |
|--------|---------|
| Search papers | `curl "https://export.arxiv.org/api/query?search_query=all:QUERY&max_results=5"` |
| Get specific paper | `curl "https://export.arxiv.org/api/query?id_list=2402.03300"` |
| Read abstract | `read_extract(urls=["https://arxiv.org/abs/2402.03300"])` |
| Read full paper (PDF) | `read_extract(urls=["https://arxiv.org/pdf/2402.03300"])` |

## Searching Papers

The API returns Atom XML. Parse with python3 for clean output.

### Clean search output

```bash
curl -s "https://export.arxiv.org/api/query?search_query=all:GRPO+reinforcement+learning&max_results=5&sortBy=submittedDate&sortOrder=descending" | python3 -c "
import sys, xml.etree.ElementTree as ET
ns = {'a': 'http://www.w3.org/2005/Atom'}
root = ET.parse(sys.stdin).getroot()
for i, entry in enumerate(root.findall('a:entry', ns)):
    title = entry.find('a:title', ns).text.strip().replace('\n', ' ')
    arxiv_id = entry.find('a:id', ns).text.strip().split('/abs/')[-1]
    published = entry.find('a:published', ns).text[:10]
    authors = ', '.join(a.find('a:name', ns).text for a in entry.findall('a:author', ns))
    summary = entry.find('a:summary', ns).text.strip()[:200]
    cats = ', '.join(c.get('term') for c in entry.findall('a:category', ns))
    print(f'{i+1}. [{arxiv_id}] {title}')
    print(f'   Authors: {authors}')
    print(f'   Published: {published} | Categories: {cats}')
    print(f'   Abstract: {summary}...')
    print(f'   PDF: https://arxiv.org/pdf/{arxiv_id}')
    print()
"
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

### Boolean operators

```
all:transformer+attention              # AND (default, use +)
all:GPT+OR+all:BERT                   # OR
all:language+model+ANDNOT+all:vision  # AND NOT
ti:"chain+of+thought"                 # Exact phrase
au:hinton+AND+cat:cs.LG               # Combined
```

## Sort and Pagination

| Parameter | Options |
|-----------|---------|
| `sortBy` | `relevance`, `lastUpdatedDate`, `submittedDate` |
| `sortOrder` | `ascending`, `descending` |
| `start` | Result offset (0-based) |
| `max_results` | Number of results (default 10, max 30000) |

```bash
# Latest 10 papers in cs.AI
curl -s "https://export.arxiv.org/api/query?search_query=cat:cs.AI&sortBy=submittedDate&sortOrder=descending&max_results=10"
```

## Fetching Specific Papers

```bash
# Single paper by arXiv ID
curl -s "https://export.arxiv.org/api/query?id_list=2402.03300"

# Multiple papers
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

## Reading Paper Content

```
# Abstract page (fast - metadata + abstract)
read_extract(urls=["https://arxiv.org/abs/2402.03300"])

# Full paper (PDF → text)
read_extract(urls=["https://arxiv.org/pdf/2402.03300"])

# HTML version (when available, cleaner than PDF)
read_extract(urls=["https://arxiv.org/html/2402.03300"])
```

For local PDF processing, see the `ocr-and-documents` skill.

## Common Categories

| Category | Field |
|----------|-------|
| `cs.AI` | Artificial Intelligence |
| `cs.CL` | Computation and Language (NLP) |
| `cs.CV` | Computer Vision |
| `cs.LG` | Machine Learning |
| `cs.CR` | Cryptography and Security |
| `stat.ML` | Machine Learning (Statistics) |
| `math.OC` | Optimization and Control |
| `physics.comp-ph` | Computational Physics |

Full list: https://arxiv.org/category_taxonomy

---

## Semantic Scholar (Citations, Related Papers, Author Profiles)

arXiv has no citation data. Use the **Semantic Scholar API** - free, no key needed (1 req/sec), returns JSON.

### Paper details + citation counts

```bash
curl -s "https://api.semanticscholar.org/graph/v1/paper/arXiv:2402.03300?fields=title,authors,citationCount,referenceCount,influentialCitationCount,year,abstract" | python3 -m json.tool
```

### Who cited this paper

```bash
curl -s "https://api.semanticscholar.org/graph/v1/paper/arXiv:2402.03300/citations?fields=title,authors,year,citationCount&limit=10" | python3 -m json.tool
```

### What this paper cites

```bash
curl -s "https://api.semanticscholar.org/graph/v1/paper/arXiv:2402.03300/references?fields=title,authors,year,citationCount&limit=10" | python3 -m json.tool
```

### Search (JSON alternative to arXiv search)

```bash
curl -s "https://api.semanticscholar.org/graph/v1/paper/search?query=GRPO+reinforcement+learning&limit=5&fields=title,authors,year,citationCount,externalIds" | python3 -m json.tool
```

### Paper recommendations

```bash
curl -s -X POST "https://api.semanticscholar.org/recommendations/v1/papers/" \
  -H "Content-Type: application/json" \
  -d '{"positivePaperIds": ["arXiv:2402.03300"], "negativePaperIds": []}' | python3 -m json.tool
```

### Author profile

```bash
curl -s "https://api.semanticscholar.org/graph/v1/author/search?query=Yann+LeCun&fields=name,hIndex,citationCount,paperCount" | python3 -m json.tool
```

**Useful fields:** `title`, `authors`, `year`, `abstract`, `citationCount`, `referenceCount`, `influentialCitationCount`, `isOpenAccess`, `openAccessPdf`, `fieldsOfStudy`, `publicationVenue`, `externalIds` (contains arXiv ID, DOI)

---

## Complete Research Workflow

1. **Discover**: search via arXiv API (curl + python3 parser above)
2. **Assess impact**: `curl .../paper/arXiv:ID?fields=citationCount,influentialCitationCount`
3. **Read abstract**: `read_extract(urls=["https://arxiv.org/abs/ID"])`
4. **Read full paper**: `read_extract(urls=["https://arxiv.org/pdf/ID"])`
5. **Find related work**: `curl .../paper/arXiv:ID/references?fields=title,citationCount&limit=20`
6. **Get recommendations**: POST to Semantic Scholar recommendations endpoint
7. **Track authors**: `curl .../author/search?query=NAME`

## Rate Limits

| API | Rate | Auth |
|-----|------|------|
| arXiv | ~1 req / 3 seconds | None needed |
| Semantic Scholar | 1 req / second | None (100/sec with free API key - sign up at semanticscholar.org/product/api, then pass `-H "x-api-key: ${SEMANTIC_SCHOLAR_API_KEY}"`) |

## Pitfalls

- arXiv returns **Atom XML** - pipe through the python3 snippet above; raw XML is unreadable
- Semantic Scholar returns **JSON** - pipe through `python3 -m json.tool`
- arXiv IDs: old format (`hep-th/0601001`) vs new (`2402.03300`) - both work in `id_list`
- `arxiv.org/abs/1706.03762` → latest version; `arxiv.org/abs/1706.03762v1` → immutable version. **Preserve the version suffix in citations** to prevent citation drift
- The API `<id>` field returns the versioned URL (e.g., `http://arxiv.org/abs/1706.03762v7`)
- **Withdrawn papers**: `<summary>` will contain "withdrawn" or "retracted" - check before treating as valid
