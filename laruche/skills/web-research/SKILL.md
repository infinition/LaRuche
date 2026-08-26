---
type: skill
name: web-research
description: Answer a factual question from the web, or enumerate what exists (files, mirrors, sources), with cross-checked sources.
---

# Web research

The everyday case: the user asks something you cannot answer from what you know, and the
answer must be TRUE today. Two or three well-chosen calls, a cross-check, an answer with
its sources.

For a multi-pass cited report on a broad subject, use `deep_research_synthesis` instead
and call `research_mode` first. This skill is the fast, correct lookup.

## Choose the right tool, it decides everything

| Tool | Takes | Returns | Use when |
|---|---|---|---|
| `web_search` | `query`, `num_results` | titles, URLs, snippets | you need to SEE what exists and pick |
| `web_deep_search` | `query`, `num_results` | snippets AND the full text of the top results | you want the answer, not a list |
| `web_fetch` | `url` (ONE, a string) | the clean text of that page | you already know the page that holds it |
| `browser_navigate` | `url`, `wait_seconds` | a real rendered browser | the content needs JavaScript or a session |
| `web_discover` | `url`, `ext`, `mode` | the files and pages a site does NOT link to | a site looks empty, or you need to LIST what it holds |
| `image_search` | `query`, `limit` | real image URLs | you need an illustration, instead of inventing a link |

`web_deep_search` is the default for a real question. `web_search` alone returns snippets,
and a snippet is an advertisement for a page, not evidence from it. Answering from
snippets is the most common way to be confidently wrong.

`web_search` also takes `allowed_domains` and `blocked_domains`, both arrays. Use
`allowed_domains` to pin a search to a primary source (`["docs.rust-lang.org"]`) and
`blocked_domains` to cut a content farm that keeps winning the ranking.

**`read_extract` is not a web tool.** It takes `path`, a LOCAL file, and reads PDF, `.txt`
or `.md`. Passing it a URL returns "File not found". The web equivalent is `web_fetch`,
which already extracts a PDF served over HTTP. `read_extract` is for the file you have
downloaded to disk: see the paragraph below.

## A PDF that `web_fetch` will not give you

Some servers refuse a plain fetch of a PDF, or the document is behind a redirect chain.
Download it, then read it from disk.

```bash
curl -sL "https://example.com/paper.pdf" -o "C:\\tmp\\paper.pdf"
```

Then `read_extract` with `path` set to `C:\tmp\paper.pdf`. It returns the text, and for a
long document a head and tail excerpt rather than a truncation with no warning. Use an
absolute path: a relative one resolves against the server's working directory, not yours.

## Procedure

1. **Anchor the question in time.** Read the authoritative date at the end of your
   context. For anything current, put the year in the query. Your training prior is old
   and will quietly answer with a stale fact.
2. **Query like a document, not like a person.** Search engines match the words that
   appear on the target page. `laruche release notes 2026` beats `what is new in
   LaRuche this year`. Two or three distinct phrasings beat one repeated.
3. **Run the searches in PARALLEL.** They are read-only, so several calls travel in the
   same message. One angle per call. Sequential searching is the slowest possible way to
   do this.
4. **Open the pages that matter** with `web_fetch`. A long page is paginated: the output
   tells you the total size and the `offset` for the next chunk. If the answer is not in
   the first chunk, read on, do not guess.
5. **Record every decisive fact with `finding`, the moment you learn it**, with its URL.
   The findings ledger survives context compaction. A fact you did not record can be gone
   before you write the final answer.
6. **Cross-check anything that matters.** Two independent sources, not two pages copying
   the same press release. A number, a date, a price or a claim about a person needs a
   second source. If the sources disagree, say so instead of picking one.
7. **Answer with the sources**, and state plainly what you could not establish.

## When the ask is "find me X", not "what is X"

These are two different jobs and the procedure above only covers the second one.

"What is the current version of Rust" is a QUESTION: one answer, cross-checked, done.
"Find me save files for this game", "find me papers on this", "find me every mirror of
this dataset" is an ENUMERATION: the answer is a LIST, and stopping at the first two
results is not an answer, it is a sample. The tell is a plural with no bound: *des*
fichiers, some papers, mirrors. Nothing in the request says how many, which means the
user wants what exists, not what is easy.

Enumeration is done when the CHANNELS are exhausted, not when a result is found:

1. **Search wide first.** Several phrasings, and in the languages the artifact would
   plausibly be published in. An old French fan site does not rank for an English query.
2. **Harvest the DOMAINS, not just the hits.** Every domain named anywhere in the results
   is a candidate, including ones mentioned in passing inside a forum post or a snippet.
   A forum answer saying "put the files you downloaded from site X into your Save folder"
   is a pointer to site X, which no search engine will rank for your query.
3. **Run `web_discover` on each candidate domain.** This is the step that separates a
   sample from an inventory. A site whose menu is JavaScript, whose directory has an
   index page, or that simply stopped being indexed in 2016 returns NOTHING to
   `web_fetch` and everything to `web_discover`. Use `ext` to filter to the file type
   you want.
4. **Only then report**, and say what you covered: the domains you swept and the channels
   that answered. "I found two" is worth much less than "I swept four sites, here are the
   seven files that exist, and site X is dead but still in the archive".

**A site that "has nothing" has not been checked until `web_discover` says so.** An empty
`web_fetch` on a home page means the page is empty, which on an old site usually means it
is a frameset. It says nothing about the files sitting one directory away.

## Prefer the primary source

Official documentation, the project repository, the standards body, the company's own
announcement. A blog summarising a release is one telephone game away from the truth and
is usually months behind. When a page cites a source, go and read that source.

## Traps

- **The snippet trap.** Deciding from search snippets alone. Open the page.
- **The stale answer.** Answering from memory on a question that has a current answer.
  Anything with "latest", "current", "now", "this year" or a price in it must be searched.
- **The single source.** One page saying something is a claim, not a fact.
- **Content farms and AI slop.** A page with no author, no date, and text that restates
  the question is worthless. Prefer a dated page with a named author.
- **The paginated page.** Reading the first 12000 characters and concluding the answer is
  absent. Check the size hint and continue with `offset`. When you are after something
  specific, pass `focus` instead: on a long page, the first chunk often holds none of it.
- **Stopping at the first find on an enumeration.** Two results reported as the answer to
  "find me X" is a sample presented as an inventory. See the enumeration section.
- **Believing an empty page.** A home page that returns nothing is a frameset or a JS
  shell, not an empty site. `web_discover` before you conclude anything is absent.
- **Fabricating a URL.** Never write a link you have not fetched. If you need an image,
  use `image_search`; do not invent image URLs.

## Failure modes

**403, paywall, captcha, or an empty result.** An obstacle, never a conclusion. In order:
try `web_fetch` with `render: true`, then the Wayback Machine
(`https://web.archive.org/web/2026/<url>`), then a search-engine cache, then a mirror,
then another source entirely. Only after all of those do you report the angle as blocked.

**The search returns nothing useful.** The query is phrased as a question, or uses your
vocabulary rather than the page's. Rewrite it with the words that would literally appear
on the page you want. Try the domain's own terminology.

**Every result is the same article, syndicated.** You found a press release. Search for
the primary source it is based on: the paper, the filing, the repository, the official
post.

**The page loads but is empty.** It is JavaScript-rendered. `web_fetch` with
`render: true`, and if that still fails, `browser_navigate`.

**You cannot establish the fact.** Say so, name what you tried, and give the closest
thing you did establish. A clear "I could not confirm this" is a correct answer. An
invented one is not.
