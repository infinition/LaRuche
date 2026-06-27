---
type: skill
name: spike
description: "Throwaway experiments to validate feasibility before building."
version: 1.0.0
license: MIT
platforms: [linux, macos, windows]
tools: [web_search, web_fetch, shell_exec, file_write, file_read, execute_code]
metadata:
  laruche:
    tags: [spike, prototype, experiment, feasibility, throwaway, exploration, research, planning, mvp, proof-of-concept]
    related_skills: [sketch, plan]
---

# Spike

Use when the user wants to **feel out an idea** before committing to a real build — validating feasibility, comparing approaches, or surfacing unknowns. Spikes are disposable by design. Throw them away once they've paid their debt.

Trigger: "let me try this", "I want to see if X works", "spike this out", "before I commit to Y", "quick prototype of Z", "is this even possible?", "compare A vs B".

## When NOT to use

- The answer is knowable from docs or reading code — just research, don't build.
- The work is on the production path — use the `plan` skill instead.
- The idea is already validated — jump straight to implementation.

## Core loop

```
decompose  →  research  →  build  →  verdict
   ↑__________________________________________↓
                  iterate on findings
```

### 1. Decompose

Break the idea into **2-5 independent feasibility questions**. Present as a table with Given/When/Then framing:

| # | Spike | Validates (Given/When/Then) | Risk |
|---|-------|----------------------------|------|
| 001 | websocket-streaming | Given a WS connection, when LLM streams tokens, then client receives chunks < 100ms | High |
| 002a | pdf-parse-pdfjs | Given a multi-page PDF, when parsed with pdfjs, then structured text is extractable | Medium |
| 002b | pdf-parse-camelot | Given a multi-page PDF, when parsed with camelot, then structured text is extractable | Medium |

**Spike types:**
- **standard** — one approach, one question
- **comparison** — same question, different approaches (shared number, letter suffix `a`/`b`/`c`)

**Order by risk.** The spike most likely to kill the idea runs first.

**Skip decomposition** only if the user already knows exactly what to spike — take it as a single spike.

### 2. Align (multi-spike only)

Present the table. Ask: "Build all in this order, or adjust?" Let the user drop, reorder, or re-frame before writing any code.

### 3. Research (per spike, before building)

Research enough to pick the right approach, then build.

1. **Brief it.** 2-3 sentences: what this spike is, why it matters, key risk.
2. **Surface competing approaches** if there's real choice:

   | Approach | Tool/Library | Pros | Cons | Status |
   |----------|-------------|------|------|--------|
   | ... | ... | ... | ... | maintained / abandoned / beta |

3. **Pick one.** State why. If 2+ are credible, build quick variants within the spike.
4. **Skip research** for pure logic with no external dependencies.

LaRuche tools for research:

```
web_search("python websocket streaming libraries 2025")
web_fetch(urls=["https://websockets.readthedocs.io/..."])
shell_exec("pip show websockets | grep Version")
file_read("path/to/cloned/README.md")
```

### 4. Build

One directory per spike. Keep it standalone.

```
spikes/
├── 001-websocket-streaming/
│   ├── README.md
│   └── main.py
├── 002a-pdf-parse-pdfjs/
│   ├── README.md
│   └── parse.js
└── 002b-pdf-parse-camelot/
    ├── README.md
    └── parse.py
```

**Bias toward something the user can interact with.** Default choices, in order:

1. A runnable CLI that takes input and prints observable output
2. A minimal HTML page demonstrating the behavior
3. A small web server with one endpoint
4. A unit test exercising the question with recognizable assertions

**Depth over speed.** Never declare "it works" after one happy-path run. Test edge cases. Follow surprising findings.

**Avoid** (unless the spike specifically requires it): complex package management, build tools/bundlers, Docker, env files, config systems. Hardcode everything — it's a spike.

**Typical tool sequence for one spike:**

```
shell_exec("mkdir -p spikes/001-websocket-streaming")
file_write("spikes/001-websocket-streaming/README.md", "# 001: websocket-streaming\n\n...")
file_write("spikes/001-websocket-streaming/main.py", "...")
shell_exec("cd spikes/001-websocket-streaming && python3 main.py")
# Observe output, iterate.
```

**Parallel comparison spikes (002a / 002b):** run them sequentially (a then b). For non-trivial parallel workloads, use `execute_code` to run both scripts and capture outputs side by side, or build them in separate `shell_exec` calls and compare results.

### 5. Verdict

Each spike's `README.md` closes with:

```markdown
## Verdict: VALIDATED | PARTIAL | INVALIDATED

### What worked
- ...

### What didn't
- ...

### Surprises
- ...

### Recommendation for the real build
- ...
```

- **VALIDATED** — core question answered yes, with evidence.
- **PARTIAL** — works under constraints X, Y, Z — document them.
- **INVALIDATED** — doesn't work, for this reason. This is a successful spike.

## Comparison spikes

When two approaches answer the same question (002a / 002b), build them back to back, then do a head-to-head:

```markdown
## Head-to-head: pdfjs vs camelot

| Dimension | pdfjs (002a) | camelot (002b) |
|-----------|--------------|----------------|
| Extraction quality | 9/10 structured | 7/10 table-only |
| Setup complexity | npm install, 1 line | pip + ghostscript |
| Perf on 100-page PDF | 3s | 18s |
| Handles rotated text | no | yes |

**Winner:** pdfjs for our use case.
```

## Frontier mode (what to spike next)

If spikes already exist and the user asks "what should I spike next?", walk the existing directories and look for:

- **Integration risks** — two validated spikes that touch the same resource but were tested independently
- **Data handoffs** — spike A's output was assumed compatible with spike B's input; never proven
- **Gaps in the vision** — capabilities assumed but unproven
- **Alternative angles** — different approaches for PARTIAL or INVALIDATED spikes

Propose 2-4 candidates as Given/When/Then. Let the user pick.

## Output conventions

- Create `spikes/` in the repo root (one dir per spike: `NNN-descriptive-name/`)
- `README.md` per spike captures question, approach, results, verdict
- Keep the code throwaway — a spike that takes 2 days to "clean up for production" was a bad spike
