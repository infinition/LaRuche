---
type: skill
name: deep_research_synthesis
description: Research a question across web and academic sources and produce a cited report.
---

# Deep research and synthesis

Several passes over web and academic sources, each claim cross-checked against a second
source, ending in a written report with its citations, saved to a file and summarised
into memory.

This is the expensive path. For a question two or three searches would settle, use
`web-research` instead: it is the same discipline at a tenth of the cost, and choosing it
correctly is part of doing this well.

## 0. Declare the mission before spending anything

`research_mode` with `mode` set, and a short `reason`. Call it BEFORE the first search,
as soon as the user asks for something thorough, deep or exhaustive, or as soon as you
realise a single lookup will not be enough. It activates the long-running research
behaviour: budgets, pacing, and the right expectations about how many passes are allowed.

Declaring it late is the usual mistake. Three shallow searches happen first, the answer is
thin, and the mode is switched on when the work is nearly over.

Then check what you already know, which is free:

- `memory_search` with `query` set to the subject. This may have been researched before.
- `session_search` with `query` runs a full-text search across PAST sessions and returns
  matching excerpts. Use it when the user says "we looked at this already", or when the
  subject feels familiar. It is far cheaper than repeating the research.

## 1. Lock the question

Before any search, write down, for yourself:

- the research question, in ONE sentence;
- three to five domain keywords, in the vocabulary the sources use, not yours;
- the target: how many sources, how recent, and where the output goes.

Defaults when the user did not say: five to fifteen sources, an eighteen month recency
window, a report written to a file. A question that has not been reduced to one sentence
produces a report about several subjects and answers none of them.

## 2. Search, broad then narrow

```
web_search(query="<keywords> survey OR review 2026", num_results=10)
web_search(query="<keywords>", allowed_domains=["arxiv.org"], num_results=10)
web_deep_search(query="<one specific sub-question>")
```

`num_results` defaults to 8 and is capped at 15. `allowed_domains` and `blocked_domains`
are arrays: use the first to pin a pass to a primary source, the second to cut a content
farm that keeps winning the ranking.

Run the passes in PARALLEL, one angle per call. They are read-only, so several fit in one
message. Collect arXiv ids, titles, official docs. De-duplicate as you go, not at the end.

`web_deep_search` returns snippets AND the full text of the top results. It is slower and
worth it on a sub-question that shallow search keeps answering with noise. It is not the
tool for the first, wide pass.

## 3. Read the sources, not the snippets

```
web_fetch(url="https://arxiv.org/abs/<id>")
web_fetch(url="https://arxiv.org/pdf/<id>")
```

`web_fetch` takes ONE `url`, as a string, and extracts a PDF served over HTTP by itself.
A long page arrives in chunks: the output states the total size and the `offset` for the
next one. If the section you need is not in the first chunk, read on with `offset`.

When a server refuses the fetch, download and read from disk instead:

```bash
curl -sL "https://arxiv.org/pdf/<id>" -o "C:\\tmp\\paper.pdf"
```

then `read_extract` with `path` set to that absolute path. `read_extract` reads LOCAL
PDF, `.txt` and `.md` files only; it has no `urls` argument and no field selection.

Discard as you read: no date, no author, a preprint over two years old with no citations,
a page that restates the question without answering it.

**Record every decisive fact with `finding` the moment you read it**, passing `fact` and
`source`. The findings ledger survives context compaction, and a multi-pass mission WILL
be compacted. A fact you did not record can be gone before you write the report.

## 4. Cross-check adversarially

For each claim that carries weight:

- find a second, INDEPENDENT source, or mark the claim `[UNVERIFIED]` in the report;
- two pages repeating the same press release are one source, not two;
- when sources disagree, write both positions and say the question is open. Picking a
  side silently is the failure this step exists to prevent.

## 5. Write the report

```markdown
# <Topic>, research report, <YYYY-MM-DD>

## Answer
<the answer to the one-sentence question, first, in two or three sentences>

## Key findings
- <finding> [<title>, <url>]

## Disagreements and gaps
- <claim A> against <claim B>, unresolved because <reason>
- <what could not be established, and what was tried>

## Sources
| # | Title | URL | Date | Kind |

## Open questions
```

Then save it, and leave a trace that survives the session:

```
file_write(path="C:/dev/research/<topic>_report.md", content="<the report>")
memory_write(node_id="research.<topic>", content="<two sentences and the file path>")
```

`memory_write` takes `node_id` and `content`. It has no `key` or `value`. `node_id` is
dotted and snake_case (`research.world_models`), never a slash path. Use an absolute path
in `file_write`: a relative one lands in the server's working directory, not the user's.

## Traps

- **Answering from abstracts.** An abstract states what the authors hoped. For a claim
  the report leans on, read the results and the limitations sections.
- **arXiv versioning.** `arxiv.org/abs/1706.03762` is the LATEST version and can differ
  substantially from what you read last month. Cite the version you actually read, with
  its `v` suffix.
- **Recency mistaken for relevance.** The most recent paper is not the most important
  one. Check citation counts before ranking.
- **The single-source finding.** One page saying something is a claim. Two pages copying
  each other is still one claim.
- **Reporting the research instead of the answer.** The user asked a question. Lead with
  the answer; the method belongs below it.

## Failure modes

**Every result is the same syndicated article.** You found a press release. Search for
what it is based on: the paper, the filing, the repository, the official announcement.

**A source is paywalled or returns 403.** An obstacle, not a conclusion. Try `web_fetch`
with `render: true`, then the Wayback Machine, then another source. Only report the angle
as blocked after all of those.

**Context ran out mid-mission and the findings are gone.** They were never written with
`finding`. Restart the pass, recording as you read this time.

**The report has ten sources and no answer.** The question was never reduced to one
sentence. Go back to step 1 and write it, then re-read what you already collected against
it: most of it usually answers something else.
