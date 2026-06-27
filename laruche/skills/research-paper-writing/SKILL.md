---
type: skill
name: research-paper-writing
title: Research Paper Writing Pipeline
description: "ML paper pipeline: experiments → LaTeX → NeurIPS/ICML/ICLR submission."
version: 1.2.0
author: Orchestra Research
license: MIT
dependencies: [semanticscholar, arxiv, habanero, requests, scipy, numpy, matplotlib, SciencePlots]
platforms: [linux, macos]
tools: [shell_exec, execute_code, file_write, file_read, web_search, web_fetch, read_extract, memory_write, memory_search, cron_create, skill_view, git_commit, git_status]
metadata:
  laruche:
    tags: [Research, Paper Writing, Experiments, ML, AI, NeurIPS, ICML, ICLR, ACL, AAAI, COLM, LaTeX, Citations, Statistical Analysis]
    category: research
    related_skills: [arxiv]

---

# Research Paper Writing Pipeline

End-to-end pipeline for ML/AI research papers targeting **NeurIPS, ICML, ICLR, ACL, AAAI, COLM**. Covers: experiment design → execution → analysis → writing → review → submission.

This is an **iterative loop**, not a linear sequence. Results trigger new experiments; reviews trigger new analysis.

**Core rules:**
1. Draft first, ask with the draft. Block only when: venue unclear, contradictory framings, results seem incomplete.
2. Never hallucinate citations. Fetch programmatically; mark unverifiable as `[CITATION NEEDED]`.
3. One-sentence contribution. If you cannot state it in one sentence, the paper is not ready.
4. Every experiment maps to a claim. Never run experiments that don't connect to the narrative.
5. Commit after every experiment batch. Git log is the experiment history.

---

## Phase 0: Project Setup

### 0.1 Explore the Repository

```bash
ls -la
find . -name "*.py" | head -30
find . -name "*.md" -o -name "*.txt" | xargs grep -l -i "result\|conclusion\|finding"
```

Look for: `README.md`, `results/`, `configs/`, `.bib` files, draft notes.

### 0.2 Workspace Structure

```
workspace/
  paper/          # LaTeX source, figures, compiled PDFs
  experiments/    # Experiment runner scripts
  code/           # Core method implementation
  results/        # Raw experiment results (auto-generated)
  tasks/          # Task/benchmark definitions
  human_eval/     # Human evaluation materials (if needed)
```

### 0.3 Version Control

```bash
git init
git checkout -b paper-draft
```

Commit discipline: every completed experiment batch → `git add -A && git commit -m "Add <experiment>: <key finding>"`.

### 0.4 Identify the Contribution

Before writing anything, articulate and propose to the scientist:
- **What**: Single thing this paper contributes (one sentence)
- **Why**: Evidence that supports it
- **So What**: Why readers should care

### 0.5 Estimate Compute Budget

```
- API costs: (price/token) × (tokens/run) × (number of runs)
- GPU hours: (time/experiment) × (experiments) × (seeds)
- Human eval: (annotators) × (hours) × (rate)
- Add 30–50% contingency for reruns
```

Run pilot experiments (1–2 seeds, task subset) before full sweeps.

---

## Phase 1: Literature Review

### 1.1 Find Seed Papers

```bash
grep -r "arxiv\|doi\|cite" --include="*.md" --include="*.bib" --include="*.py"
```

Load the `arxiv` skill for structured paper discovery: `skill_view("arxiv")`.

### 1.2 Search Iteratively (Breadth → Depth)

Use `web_search` for broad discovery, `web_fetch` for specific papers:

```
web_search("[method] + [domain] site:arxiv.org")
web_search("[baseline] comparison NeurIPS ICML 2024")
web_fetch("https://arxiv.org/abs/<id>")
```

Run 2–3 rounds:
- **Round 1 (Breadth)**: 4–6 parallel queries on different angles. Collect key concepts.
- **Round 2 (Depth)**: Follow-up on new terminology and references from Round 1.
- **Round 3 (Targeted)**: Fill gaps — missing baselines, concurrent work, negative results.
- Stop when a new round returns >80% papers already collected.

### 1.3 Verify Every Citation (MANDATORY)

```
Per citation:
1. SEARCH   → Query Semantic Scholar with specific keywords
2. VERIFY   → Confirm paper exists in 2+ sources
3. RETRIEVE → Fetch BibTeX via DOI content negotiation
4. VALIDATE → Confirm the cited claim actually appears in the paper
5. ADD      → Add verified BibTeX to bibliography
If ANY step fails → mark [CITATION NEEDED]
```

```python
import requests

def doi_to_bibtex(doi: str) -> str:
    response = requests.get(
        f"https://doi.org/{doi}",
        headers={"Accept": "application/x-bibtex"}
    )
    response.raise_for_status()
    return response.text
```

### 1.4 Organize Related Work

Group papers by methodology, not paper-by-paper.

**Good**: "One line of work uses X's assumption [refs] whereas we use Y's because..."
**Bad**: "Smith et al. introduced X. Jones et al. introduced Y."

---

## Phase 2: Experiment Design

### 2.1 Map Claims to Experiments

| Claim | Experiment | Expected Evidence |
|-------|-----------|-------------------|
| "Method outperforms baselines" | Main comparison | Win rate, statistical significance |
| "Effect larger for weaker models" | Scaling study | Monotonic improvement curve |

**Rule**: If an experiment doesn't map to a claim, don't run it.

### 2.2 Design Strong Baselines

- **Naive**: Simplest possible approach
- **Strong**: Best known existing method
- **Ablation**: Your method minus one component
- **Compute-matched**: Same budget, different allocation

### 2.3 Define Evaluation Protocol

Before running anything:
- Metrics (direction: ↑ or ↓)
- Aggregation across runs/tasks
- Statistical tests (McNemar's for pairwise, bootstrapped CIs for key results)
- Sample sizes (runs, problems, seeds)

### 2.4 Write Experiment Scripts

**Incremental saving** — crash recovery:
```python
result_path = f"results/{task}/{strategy}/result.json"
if os.path.exists(result_path):
    continue  # Skip completed work
# ... run ...
with open(result_path, 'w') as f:
    json.dump(result, f, indent=2)
```

**Artifact structure**:
```
results/<experiment>/<task>/<strategy>/
  final_output.md
  history.json
  pass_01/version_a.md, version_b.md, critic.md
```

**Separation of concerns**:
```
run_experiment.py      # Core runner
run_baselines.py       # Baseline comparison
analyze_results.py     # Statistical analysis
make_charts.py         # Visualization
```

### 2.5 Human Evaluation (If Applicable)

Required when automated metrics don't capture what you care about (fluency, helpfulness, safety) or venue expects it (ACL generation tasks).

Key design decisions:
- **Annotators**: Expert / crowdworker / end-user — match to claims
- **Scale**: Pairwise comparison more reliable than Likert for LLM outputs
- **Sample size**: Power analysis or minimum 100 items, 3+ annotators
- **Agreement**: Krippendorff's alpha for >2 annotators; report raw agreement too
- **Platform**: Prolific (quality), MTurk (scale), internal (domain expertise)

Annotation checklist:
```
- [ ] Clear task description with good/bad examples
- [ ] At least 2 worked examples per category
- [ ] Attention checks / gold items (10–15% of total)
- [ ] Qualification or screening round
- [ ] Fair compensation (>= local minimum wage)
- [ ] IRB/ethics review if required
```

Reporting requirements: annotator count, qualifications, inter-annotator agreement (metric + value), compensation details, annotation interface description.

---

## Phase 3: Experiment Execution & Monitoring

### 3.1 Launch Experiments

```bash
nohup python run_experiment.py --config config.yaml > logs/experiment_01.log 2>&1 &
echo $!  # Record PID
```

4+ concurrent API experiments will hit rate limits — run sequentially or stagger.

### 3.2 Set Up Monitoring

Use `cron_create` for periodic status checks:

```
Monitor prompt template:
1. shell_exec("ps aux | grep <pattern>")
2. shell_exec("tail -30 <logfile>")
3. shell_exec("ls <result_dir>")
4. If results exist: read_extract them, compute metrics
5. If all done: git add -A && git commit -m "<message>" && git push
6. Report structured table with key metrics and next step
7. If nothing changed since last check: respond [SILENT]
```

`[SILENT]` suppresses user notification — use when nothing is new.

### 3.3 Handle Failures

| Failure | Detection | Recovery |
|---------|-----------|----------|
| API rate limit / credit exhaustion | 402/429 in logs | Wait, re-run (scripts skip completed work) |
| Process crash | PID gone, incomplete results | Re-run from last checkpoint |
| Timeout | No log progress | Kill, skip, note in results |
| Wrong model ID | Model name error | Fix and re-run |

### 3.4 Maintain an Experiment Journal

```json
{
  "id": "exp_003",
  "parent": "exp_001",
  "hypothesis": "Scope constraints will fix convergence failure from exp_001",
  "config": {"model": "haiku", "strategy": "autoreason"},
  "status": "completed",
  "result_path": "results/exp_003/",
  "key_metrics": {"win_rate": 0.85},
  "analysis": "Constraints fixed convergence. Win rate: 0.42 → 0.85.",
  "next_steps": ["Try on Sonnet", "Test without structure template"]
}
```

Copy the script snapshot per experiment:
```bash
cp experiment.py results/exp_003/experiment_snapshot.py
```

---

## Phase 4: Result Analysis

### 4.1 Aggregate Results

```python
import json
from pathlib import Path
import numpy as np

results = {}
for result_file in Path("results/").rglob("result.json"):
    data = json.loads(result_file.read_text())
    strategy = result_file.parent.name
    task = result_file.parent.parent.name
    results.setdefault(strategy, {})[task] = data

for strategy, tasks in results.items():
    scores = [t["score"] for t in tasks.values()]
    print(f"{strategy}: mean={np.mean(scores):.3f}, std={np.std(scores):.3f}")
```

### 4.2 Statistical Significance

Always compute:
- Error bars: std dev or std error — specify which
- 95% confidence intervals for key results
- Pairwise tests: McNemar's test for comparing two methods
- Effect sizes: Cohen's d or h

See `references/experiment-patterns.md` for full implementations.

### 4.3 Identify the Story

After analysis, explicitly answer:
1. **Main finding?** One sentence.
2. **Surprising results?** Unexpected findings make the best papers.
3. **What failed?** Honest failure reporting strengthens the paper.
4. **Follow-up experiments?**

**Handling negative/null results:**

| Situation | Action |
|-----------|--------|
| Wrong hypothesis but **why** is informative | Frame around the analysis |
| Method doesn't beat baselines but **reveals something** | Reframe as understanding paper |
| Clean negative on popular claim | Write it up — field needs to know |
| Inconclusive, no clear story | Pivot or run different experiments |

Venues welcoming negative results: NeurIPS Datasets & Benchmarks, TMLR, workshops.

### 4.4 Create Figures and Tables

**Figures** — always vector:
```python
import matplotlib.pyplot as plt
import scienceplots
with plt.style.context(['science', 'no-latex']):
    fig, ax = plt.subplots(figsize=(3.5, 2.5))  # Single-column
    ax.plot(x, y, label='Ours', color='#0072B2')
    ax.plot(x, y2, label='Baseline', color='#D55E00', linestyle='--')
    ax.legend()
    fig.savefig('paper/fig_results.pdf', bbox_inches='tight')
```
Standard sizes: single column `(3.5, 2.5)`, double column `(7.0, 3.0)`.
Colorblind-safe palette: Okabe-Ito (`#0072B2`, `#E69F00`, `#009E73`, `#D55E00`, `#CC79A7`).

**Tables** — use `booktabs`:
```latex
\usepackage{booktabs}
\begin{tabular}{lcc}
\toprule
Method & Accuracy $\uparrow$ & Latency $\downarrow$ \\
\midrule
Baseline & 85.2 & 45ms \\
\textbf{Ours} & \textbf{92.1} & 38ms \\
\bottomrule
\end{tabular}
```
Bold best value per metric. Include direction symbols. Right-align numbers.

### 4.5 Write the Experiment Log (Bridge to Writeup)

Create `experiment_log.md` before moving to writing:

```markdown
## Contribution (one sentence)
[Main claim]

## Experiments
### Experiment 1: [Name]
- Claim tested: [which paper claim]
- Setup: [model, dataset, runs]
- Key result: [one number]
- Result files: results/exp1/final_info.json
- Figures: figures/exp1_comparison.pdf
- Surprising findings: [if any]

## Figures
| Filename | Description | Section |

## Failed Experiments
- [What was tried, why it failed, what it tells us]

## Open Questions
- [What the results raised]
```

This file is the primary context bridge for drafting — load it instead of raw JSON/CSV.

---

## Phase 5: Paper Drafting

### Context Management

For large projects, load only what's needed per task:

| Task | Load | Do NOT load |
|------|------|-------------|
| Writing Introduction | `experiment_log.md`, 5–10 paper abstracts | Raw result JSONs, full scripts |
| Writing Methods | Configs, pseudocode, architecture | Raw logs, other experiments |
| Writing Results | Result summary tables, figure list | Analysis scripts, raw data |
| Writing Related Work | Citation notes, `.bib` file | Experiment files |

For very large projects, create a `context/` directory:
```
context/
  contribution.md       # 1 sentence
  experiment_summary.md # Key results table
  literature_map.md     # Organized citation notes
  figure_inventory.md   # List of figures with descriptions
```

### Writing Checklist

```
- [ ] 1. One-sentence contribution
- [ ] 2. Draft Figure 1 (core idea or most compelling result)
- [ ] 3. Abstract (5-sentence formula)
- [ ] 4. Introduction (1–1.5 pages max)
- [ ] 5. Methods
- [ ] 6. Experiments & Results
- [ ] 7. Related Work
- [ ] 8. Conclusion & Discussion
- [ ] 9. Limitations (REQUIRED at all venues)
- [ ] 10. Appendix plan
- [ ] 11. LaTeX quality checklist
```

### Two-Pass Drafting

**Pass 1**: Write + immediately refine each section (catches local issues while fresh).
**Pass 2**: After all sections exist, revisit each with full-paper context (catches cross-section redundancy, inconsistent terminology, broken narrative flow).

Pass 2 prompt per section:
```
Review [SECTION] in context of the complete paper.
- Redundancies with other sections?
- Terminology consistent with Introduction and Methods?
- Can anything be cut without weakening the message?
- Does it flow from the previous section and into the next?
Make minimal, targeted edits. Do not rewrite from scratch.
```

### Title

Good: states contribution ("Autoreason: When Iterative LLM Refinement Works and Why It Fails").
Bad: generic ("An Approach to Improving LLM Outputs") or >15 words.
Include method name and 1–2 searchable keywords. Test: can a reviewer infer domain and contribution from the title alone?

### Abstract (5-Sentence Formula)

From Sebastian Farquhar (DeepMind):
1. What you achieved: "We introduce...", "We prove...", "We demonstrate..."
2. Why this is hard and important
3. How you do it (with specialist keywords)
4. What evidence you have
5. Your most remarkable result

Delete generic openings like "Large language models have achieved remarkable success...".

### Figure 1

Draft before writing Introduction — forces you to clarify the core idea.

| Type | When | Example |
|------|------|---------|
| Method diagram | New architecture/pipeline | TikZ flowchart |
| Results teaser | One result tells the story | Bar chart with clear gap |
| Problem illustration | Problem is unintuitive | Before/after failure mode |
| Conceptual diagram | Abstract contribution | 2×2 matrix |

Caption alone must communicate the core idea. Figure must be interpretable without reading the text.

### Introduction (1–1.5 pages max)

Must include: problem statement, approach overview, 2–4 bullet contribution list (max 1–2 lines each). Methods should start by page 2–3.

### Methods

Enable reimplementation: conceptual outline or pseudocode, all hyperparameters, architectural details for reproduction. Present final design decisions — ablations go in Experiments.

### Experiments & Results

For each experiment: state the claim it tests, how it connects to the contribution, what to observe. Report error bars (specify std dev vs std error), hyperparameter search ranges, compute infrastructure, random seeds.

### Related Work

Organize methodologically. Cite generously — reviewers likely authored relevant papers.

### Limitations (REQUIRED)

Required at all major venues. Be specific. Pre-empt criticisms. Explain why limitations don't undermine core claims. "We foresee no negative impacts" is almost never credible.

### Conclusion (0.5–1 page)

- Restate contribution in one sentence (different wording from abstract)
- Summarize key findings (2–3 sentences, not a list)
- Implications for the field
- 2–3 concrete future steps
- Do NOT introduce new results

### Appendix

Unlimited at all major venues. Sections: Proofs & Derivations, Additional Experiments, Implementation Details, Dataset Documentation, Prompts & Templates, Human Evaluation, Additional Figures. Main paper must be self-contained — reviewers are not required to read appendices. Always cross-reference: "Full results in Table 5 (Appendix B)".

### Writing Style

**Sentence-level (Gopen & Swan)**: keep subject and verb close; place emphasis at sentence ends; put context before new information; one paragraph, one point; use verbs not nominalizations.

**Word choice (Lipton, Steinhardt)**: be specific ("accuracy" not "performance"); eliminate hedging (drop "may" unless genuinely uncertain); consistent terminology throughout.

### LaTeX Quality Checklist

Run after every edit:
```
- [ ] Math symbols balanced ($ signs)
- [ ] \ref matches \label for all figures/tables
- [ ] \cite matches .bib entries (no fabricated citations)
- [ ] Every \begin{env} has matching \end{env}
- [ ] No HTML contamination (</end{figure}> etc.)
- [ ] No unescaped underscores outside math mode
- [ ] No duplicate \label definitions
- [ ] Numbers in text match actual results
- [ ] All figures have captions and labels
```

### LaTeX Templates

```bash
# 1. Copy entire template directory
cp -r templates/neurips2025/ ~/papers/my-paper/
# 2. Verify it compiles before any changes
latexmk -pdf main.tex
# 3. Replace content section by section; compile after each
# 4. Never modify .sty files
```

| Conference | Main File | Style File | Pages |
|------------|-----------|------------|-------|
| NeurIPS 2025 | `main.tex` | `neurips.sty` | 9 |
| ICML 2026 | `example_paper.tex` | `icml2026.sty` | 8 |
| ICLR 2026 | `iclr2026_conference.tex` | `iclr2026_conference.sty` | 9 |
| ACL 2025 | `acl_latex.tex` | `acl.sty` | 8 |
| AAAI 2026 | `aaai2026-unified-template.tex` | `aaai2026.sty` | 7 |
| COLM 2025 | `colm2025_conference.tex` | `colm2025_conference.sty` | 9 |

Universal: double-blind, references don't count, appendices unlimited, LaTeX required.

### Page Budget (When Over Limit)

| Strategy | Saves | Risk |
|----------|-------|------|
| Move proofs to appendix | 0.5–2 pages | Low |
| Condense related work | 0.5–1 page | Medium |
| Combine tables with subfigures | 0.25–0.5 page | Low |
| Remove qualitative examples | 0.5–1 page | Medium |
| Reduce figure sizes | 0.25–0.5 page | High |

Do NOT reduce font size, change margins, or remove required sections.

### Ethics & Broader Impact Statement

Required or expected at NeurIPS, ICML, ICLR, ACL.

```latex
\section*{Broader Impact Statement}
% 1. Positive applications (1–2 sentences)
% 2. Risks and specific mitigations (1–3 sentences)
% 3. Limitations of impact claims (1 sentence)
```

Pitfalls: "no negative impacts" (almost never credible), vague risk statements, forgetting LLM use disclosure (mandatory at ICLR, ACL).

---

## Phase 6: Self-Review & Revision

### 6.1 Simulate Reviews (Ensemble)

Generate N=3–5 independent reviews. Use different models or temperatures. Prompt with **negative bias** (LLMs have documented positivity bias):

```
You are an expert reviewer for [VENUE]. Be critical and thorough.
Flag weaknesses clearly in your scores. Do not give the benefit of the doubt.
Evaluate: Soundness, Clarity, Significance, Originality.
Return JSON: {summary, strengths[], weaknesses[], questions[],
  missing_references[], soundness(1-4), presentation(1-4),
  contribution(1-4), overall(1-10), confidence(1-5)}
```

Then meta-review: feed all N reviews to a meta-reviewer that identifies consensus, resolves disagreements, averages scores. Be conservative on unresolved weaknesses.

**Visual review pass**: If you have a vision model, run it on the compiled PDF checking figure quality, layout issues, caption accuracy, colorblind readability, grayscale readability.

**Claim verification**: Extract every factual claim → trace each to a result file → verify number matches. Use a fresh sub-agent for this to avoid confirmation bias.

### 6.2 Prioritize Feedback

| Priority | Action |
|----------|--------|
| Critical (technical flaw, missing baseline) | Must fix; may require new experiments → Phase 2 |
| High (clarity issue, missing ablation) | Fix in this revision |
| Medium (minor writing, extra experiments) | Fix if time allows |
| Low (style preferences) | Note for future |

### 6.3 Rebuttal Writing

Point-by-point format:
```
> R1-W1: "The paper lacks comparison with Method X."
We added Method X in Table 3 (revised). Our method outperforms X by 3.2pp (p<0.05).
Note: X requires 2× our compute budget.
```

Rules: address every concern; lead with strongest responses; be concise; never defensive; use `latexdiff` to generate a marked-up diff PDF for supplement.

```bash
latexdiff paper_v1.tex paper_v2.tex > paper_diff.tex
pdflatex paper_diff.tex
```

### 6.4 Version Snapshots

```
paper_v1_first_draft.tex
paper_v2_post_review.tex
paper_v3_pre_submission.tex
paper_v4_camera_ready.tex
```

---

## Phase 7: Submission Preparation

### 7.1 Anonymization Checklist

```
- [ ] No author names/affiliations in PDF
- [ ] No acknowledgments (add after acceptance)
- [ ] Self-citations in third person ("Smith et al. [1]..." not "We previously...")
- [ ] No personal GitHub URLs (use https://anonymous.4open.science/)
- [ ] No institutional logos in figures
- [ ] No identifying file metadata
- [ ] No "our previous work" phrasing
- [ ] Supplementary materials clean
```

Common mistakes: Git commit messages in supplementary code, institutional watermarks, acknowledgments left from previous draft.

### 7.2 Formatting Verification

```
- [ ] Page limit respected (excluding references and appendix)
- [ ] All figures vector PDF or 600 DPI PNG
- [ ] All figures readable in grayscale
- [ ] All tables use booktabs
- [ ] References compile (no "?" in citations)
- [ ] Required sections present (limitations, broader impact)
```

### 7.3 Pre-Submission Validation

```bash
# 1. Lint
chktex main.tex -q -n2 -n24 -n13 -n1

# 2. Check citations exist in .bib
python3 -c "
import re
tex = open('main.tex').read()
bib = open('references.bib').read()
cites = set(re.findall(r'\\\\cite[tp]?{([^}]+)}', tex))
for cite_group in cites:
    for cite in cite_group.split(','):
        cite = cite.strip()
        if cite and cite not in bib:
            print(f'MISSING: {cite}')
"

# 3. Check figure files exist
python3 -c "
import re, os
tex = open('main.tex').read()
for fig in re.findall(r'\\\\includegraphics(?:\[.*?\])?{([^}]+)}', tex):
    if not os.path.exists(fig):
        print(f'MISSING FIGURE: {fig}')
"

# 4. Check duplicate labels
python3 -c "
import re
from collections import Counter
labels = re.findall(r'\\\\label{([^}]+)}', open('main.tex').read())
for k,v in Counter(labels).items():
    if v > 1: print(f'DUPLICATE LABEL: {k} ({v}x)')
"
```

### 7.4 Final Compilation

```bash
rm -f *.aux *.bbl *.blg *.log *.out
latexmk -pdf main.tex
ls -la main.pdf
```

Common errors: "Undefined control sequence" → missing package; "Missing $ inserted" → math outside math mode; "File not found" → wrong path; "Citation undefined" → run bibtex first.

### 7.5 Conference-Specific Requirements

| Venue | Special Requirements |
|-------|---------------------|
| NeurIPS | Paper checklist in appendix, lay summary if accepted |
| ICML | Broader Impact Statement (after conclusion, doesn't count toward limit) |
| ICLR | LLM disclosure mandatory, reciprocal reviewing |
| ACL | Mandatory Limitations section, Responsible NLP checklist |
| AAAI | Strict style file — no modifications |
| COLM | Frame contribution for language model community |

### 7.6 Format Conversion Between Venues

```bash
# Start fresh with target template — never copy preambles
cp -r templates/icml2026/ new_submission/
# Copy ONLY content: abstract text, sections, figures, tables, bib entries
```

| From → To | Page Change | Key Adjustments |
|-----------|-------------|-----------------|
| NeurIPS → ICML | 9→8 | Cut 1 page, add Broader Impact |
| ICML → ICLR | 8→9 | Expand experiments, add LLM disclosure |
| NeurIPS → ACL | 9→8 | NLP conventions, add Limitations |
| ICLR → AAAI | 9→7 | Significant cuts, strict style |

After rejection: address reviewer concerns but don't reference the previous submission.

### 7.7 Camera-Ready (Post-Acceptance)

```
- [ ] De-anonymize: add names, affiliations, emails
- [ ] Add Acknowledgments
- [ ] Add public code/data URL
- [ ] Address mandatory meta-reviewer revisions
- [ ] Switch template to camera-ready mode if applicable
- [ ] Update all "anonymous" placeholders
- [ ] Verify final PDF compiles
- [ ] Upload supplementary materials to venue portal
```

### 7.8 arXiv Strategy

| Situation | Recommendation |
|-----------|---------------|
| Double-blind venue (NeurIPS, ICML, ACL) | Post after submission deadline |
| ICLR | Explicitly allows pre-submission arXiv |
| Workshop | Post anytime |
| Priority concern | Post immediately (accept anonymity tradeoff) |

Categories: `cs.LG` (ML), `cs.CL` (NLP), `cs.AI` (reasoning/agents), `cs.CV` (vision). List primary + 1–2 cross-listed.

### 7.9 Code Packaging

Repository structure:
```
your-method/
  README.md           # Setup, usage, reproduction
  requirements.txt
  LICENSE             # MIT or Apache 2.0
  configs/
  src/
  scripts/
    train.py
    evaluate.py
    reproduce_table1.sh
  data/download_data.sh
```

Pre-release checklist:
```
- [ ] Runs from clean clone (test on fresh machine or Docker)
- [ ] All dependencies pinned
- [ ] No hardcoded absolute paths
- [ ] No API keys or personal data
- [ ] Results reproducible within expected variance
```

---

## LaRuche Tool Reference

| Tool | Usage |
|------|-------|
| `shell_exec` | LaTeX compilation (`latexmk -pdf`), git, launching experiments (`nohup python run.py &`), process checks |
| `execute_code` | Python for citation verification, statistical analysis, data aggregation |
| `file_write` / `file_read` | Paper editing, experiment scripts, result files |
| `web_search` | Literature discovery |
| `web_fetch` / `read_extract` | Fetch paper content, verify citations |
| `memory_write` / `memory_search` | Persist contribution framing, venue choice, reviewer feedback across sessions |
| `cron_create` | Schedule experiment monitoring, deadline countdowns |
| `skill_view("arxiv")` | Load arxiv skill for structured paper discovery |

**Session startup protocol:**
```
1. file_read("experiment_log.md")         # Recall results
2. memory_search("paper status")          # Recall key decisions
3. shell_exec("git log --oneline -10")    # Check recent commits
4. shell_exec("ps aux | grep python")     # Check running experiments
5. Report status, ask for direction
```

**Parallel section drafting** — spawn isolated sub-agents with scoped context:
```
# Methods agent: receives configs, pseudocode, architecture details only
# Related Work agent: receives citation notes and .bib file only
# Results agent: receives experiment_log.md and result summary tables only
# Each agent has no shared context — provide everything needed in the prompt
```

**Experiment monitoring cron**:
```
cron_create({
  "schedule": "*/30 * * * *",
  "prompt": "Check experiment status:
    1. shell_exec('ps aux | grep run_experiment')
    2. shell_exec('tail -30 logs/experiment.log')
    3. shell_exec('ls results/')
    4. If complete: read results, compute metrics,
       shell_exec('git add -A && git commit -m \"Add results\" && git push')
    5. Report: table with key metrics and next step
    6. If nothing changed: respond [SILENT]"
})
```

**Decision points requiring human input** (pause and ask the user):
- Target venue (affects page limits, framing)
- Multiple valid contribution framings
- Experiment priority when TODO exceeds time
- Submission readiness

Do NOT ask about word choice, section ordering, which results to highlight — draft with a choice, flag it.

---

## Common Issues

| Issue | Fix |
|-------|-----|
| Abstract too generic | Delete first sentence if any ML paper could use it. Start with your specific contribution. |
| Introduction > 1.5 pages | Move background to Related Work. Front-load contribution bullets. |
| Experiments lack explicit claims | Add "This experiment tests whether [specific claim]..." before each. |
| Missing statistical significance | Add error bars, run count, statistical tests, CIs. |
| Scope creep | Cut any experiment without a mapped claim. |
| Missing broader impact | See Phase 5 Ethics section. "No negative impacts" is almost never credible. |
| Reviewers question reproducibility | Release code (Phase 7.9), document hyperparameters and seeds. |
| Negative/null results | See Phase 4.3. Consider workshops, TMLR, or reframe as analysis. |

---

## Reference Documents

| Document | Contents |
|----------|----------|
| `references/writing-guide.md` | Gopen & Swan 7 principles, Perez micro-tips, Lipton word choice, figure design |
| `references/citation-workflow.md` | Citation APIs, Python code, CitationManager class, BibTeX management |
| `references/checklists.md` | NeurIPS 16-item, ICML, ICLR, ACL requirements, universal pre-submission checklist |
| `references/reviewer-guidelines.md` | Evaluation criteria, scoring, common concerns, rebuttal strategies |
| `references/experiment-patterns.md` | Design patterns, evaluation protocols, monitoring, error recovery, stats implementations |
| `references/autoreason-methodology.md` | Iterative refinement loop, strategy selection, model guide, prompts, Borda scoring |
| `references/human-evaluation.md` | Annotation guidelines, agreement metrics, crowdsourcing QC, IRB guidance |
| `references/paper-types.md` | Theory, survey, benchmark, position papers |
| `templates/README.md` | LaTeX template compilation (VS Code, CLI, Overleaf) |
