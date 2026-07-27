---
type: skill
name: arxiv
description: Find academic papers on arXiv, with citation counts and BibTeX.
---

# arXiv

Find the paper, judge whether it matters, read it, cite it. Two free APIs, no key and no
account: arXiv for the papers themselves, Semantic Scholar for what arXiv does not carry,
which is chiefly who cited whom.

Use this when the user names a paper, an author, a research area, or asks what the state
of the art is on something academic. For a general web question, use `web-research`. For
a multi-pass cited report, use `deep_research_synthesis`, which calls this one.

## Prerequisites

`curl` and Python 3, both already present. Nothing to install: the bundled script uses
the standard library only.

```bash
python skills/arxiv/scripts/search_arxiv.py "attention is all you need" --max 3
```

Success prints titles, authors, dates and arXiv ids. A traceback means the path is wrong;
locate the script with `file_search` rather than guessing another prefix.

## Procedure

1. **Find candidates.** The script parses arXiv's Atom XML and prints something readable.

   ```bash
   python skills/arxiv/scripts/search_arxiv.py "GRPO reinforcement learning"
   python skills/arxiv/scripts/search_arxiv.py "world models" --max 10 --sort date
   python skills/arxiv/scripts/search_arxiv.py --author "Yann LeCun" --max 5
   python skills/arxiv/scripts/search_arxiv.py --category cs.AI --sort date
   python skills/arxiv/scripts/search_arxiv.py --id 2402.03300,1706.03762
   ```

2. **Judge before reading.** arXiv is not peer reviewed and holds no citation data. A
   preprint with no citations after two years is rarely the state of the art, whatever its
   abstract claims. Ask Semantic Scholar:

   ```bash
   curl -s "https://api.semanticscholar.org/graph/v1/paper/arXiv:2402.03300?fields=title,year,citationCount,influentialCitationCount,isOpenAccess" | python -m json.tool
   ```

   `influentialCitationCount` is the number worth reading: it counts papers that built on
   this one, rather than papers that listed it in related work.

3. **Read it.** `web_fetch` takes ONE `url`, as a string, and extracts a PDF served over
   HTTP by itself.

   ```
   web_fetch(url="https://arxiv.org/abs/2402.03300")
   web_fetch(url="https://arxiv.org/pdf/2402.03300")
   ```

   The abstract states what the authors hoped for. For any claim you are going to repeat,
   read the results and the limitations sections in the PDF itself.

   If the fetch is refused, download and read from disk:
   `curl -sL "https://arxiv.org/pdf/<id>" -o "C:\tmp\paper.pdf"`, then `read_extract`
   with `path` set to that absolute path. `read_extract` reads LOCAL files only.

4. **Follow the citations**, in whichever direction the question needs.

   ```bash
   # who built on it
   curl -s "https://api.semanticscholar.org/graph/v1/paper/arXiv:ID/citations?fields=title,year,citationCount&limit=10" | python -m json.tool
   # what it rests on
   curl -s "https://api.semanticscholar.org/graph/v1/paper/arXiv:ID/references?fields=title,year,citationCount&limit=20" | python -m json.tool
   ```

5. **Record what matters with `finding`**, passing `fact` and `source`, as you read it. A
   paper you read three calls ago can be gone from context by the time you write.

## Query syntax

The API matches on prefixed fields, and `+` stands for the space, so quote the whole URL.

| Prefix | Searches |
|---|---|
| `all:` | everywhere |
| `ti:` | title |
| `au:` | author |
| `abs:` | abstract |
| `cat:` | category |
| `co:` | the comment field, where "accepted at NeurIPS" lives |

Combine with `+AND+`, `+OR+`, `+ANDNOT+`. An exact phrase goes in quotes:
`ti:"chain+of+thought"`.

Sorting takes `sortBy` as `relevance`, `lastUpdatedDate` or `submittedDate`, and
`sortOrder` as `ascending` or `descending`. `start` pages through the results.

```bash
curl -s "https://export.arxiv.org/api/query?search_query=cat:cs.AI&sortBy=submittedDate&sortOrder=descending&max_results=10"
```

Common categories: `cs.AI`, `cs.CL` for language, `cs.CV` for vision, `cs.LG` for machine
learning, `cs.CR` for security, `stat.ML`, `math.OC`. Full taxonomy at
<https://arxiv.org/category_taxonomy>.

## BibTeX

```bash
curl -s "https://export.arxiv.org/api/query?id_list=1706.03762" | python -c "
import sys, xml.etree.ElementTree as ET
ns = {'a': 'http://www.w3.org/2005/Atom', 'arxiv': 'http://arxiv.org/schemas/atom'}
entry = ET.parse(sys.stdin).getroot().find('a:entry', ns)
if entry is None: sys.exit('paper not found')
title = ' '.join(entry.find('a:title', ns).text.split())
authors = ' and '.join(a.find('a:name', ns).text for a in entry.findall('a:author', ns))
year = entry.find('a:published', ns).text[:4]
ident = entry.find('a:id', ns).text.strip().split('/abs/')[-1]
cat = entry.find('arxiv:primary_category', ns)
surname = entry.find('a:author', ns).find('a:name', ns).text.split()[-1]
print(f'@article{{{surname}{year}_{ident.replace(\".\", \"\")},')
print(f'  title         = {{{title}}},')
print(f'  author        = {{{authors}}},')
print(f'  year          = {{{year}}},')
print(f'  eprint        = {{{ident}}},')
print(f'  archivePrefix = {{arXiv}},')
print(f'  primaryClass  = {{{cat.get(\"term\") if cat is not None else \"cs.LG\"}}},')
print(f'  url           = {{https://arxiv.org/abs/{ident}}}')
print('}')
"
```

The `<id>` field carries the version, so the entry pins the version you actually read.

## Traps

- **`arxiv.org/abs/1706.03762` is the LATEST version**, and versions are not editorial
  touch-ups: results change and claims get withdrawn between them. Cite with the suffix
  you read, `1706.03762v7`, or the citation silently drifts under you.
- **Check for a withdrawal.** The `<summary>` of a withdrawn paper says so in words. Read
  it before treating a result as real.
- **arXiv carries no citation data at all.** Any citation count in an answer came from
  Semantic Scholar or was invented. There is no third possibility.
- **A high citation count is not agreement.** Papers are cited in order to be refuted.
  When the number carries weight, look at what the citing papers actually say.
- **Two id formats coexist.** Old is `hep-th/0601001`, new is `2402.03300`. Both resolve.
  Do not "correct" an old one.
- **The rate limits are real.** arXiv wants roughly one request every three seconds,
  Semantic Scholar one per second without a key. Do not fan these out in parallel: the
  block lands on the machine and outlives the session.

## Failure modes

**The search returns nothing for a paper you know exists.** The query used your words
rather than the paper's. Try `ti:` with three distinctive words from the real title, or
`--id` if you have the identifier.

**Semantic Scholar returns 404 for a valid arXiv id.** It has not indexed that preprint
yet, which is normal in the first days. Report the paper without citation data, rather
than reporting no paper.

**Semantic Scholar returns 429.** Rate limited. Wait, then retry once, spacing calls to
one per second.

**The PDF fetch returns something unreadable.** The paper is a scan, or extraction failed.
Try `arxiv.org/html/<id>`, which exists for recent submissions, then fall back to
downloading it and using the `ocr-and-documents` skill.

**The script prints a traceback about XML.** arXiv returned an HTML error page instead of
Atom, which is what rate limiting looks like from here. Wait thirty seconds and retry.
