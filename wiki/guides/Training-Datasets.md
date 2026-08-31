# Training Datasets

LaRuche can turn completed LaReine reviews into training data. This is more useful than
a plain conversation export because a supervised review can preserve four related
objects at once: the request, the first draft, the answer eventually released and the
judge's explanation.

Capture is off by default. It starts only after an administrator enables **Collect
reviews as training data** in **Settings > LaReine** and saves the configuration. It is
not retroactive.

## What one review records

Each completed review appends one JSON object to
`evals/reine-dataset.jsonl` inside the hive data directory.

| Field | Content |
|---|---|
| `ts` | UTC capture time |
| `mode` | LaReine mode used for the review |
| `rounds` | Number of review rounds |
| `paire_preference` | Whether the record contains a genuine rejected and chosen pair |
| `prompt` | User request |
| `rejected` | Initial draft reviewed by LaReine |
| `chosen` | Best answer released after review |
| `critique` | Concrete correction instructions returned to the worker |
| `reasoning` | Judge analysis |
| `scores` | Relevance, methodology, objective, brand and confidence scores |
| `avis` | `approve`, `revise` or `escalate` |

The raw journal is intentionally richer than any one trainer format. Keep it as the
source dataset and derive narrower exports from it.

## Three export formats

The LaReine settings page provides three download buttons. The endpoint is also
available to an authenticated administrator at
`GET /api/reine/dataset?format=sft|dpo|judge`.

### SFT

SFT exports every valid request and accepted answer using the common chat format:

```json
{"messages":[{"role":"user","content":"..."},{"role":"assistant","content":"..."}]}
```

Use it to fine-tune a model toward the answers LaReine allowed through. A review does
not need a rejected draft to become an SFT example.

### DPO preference pairs

DPO exports only reviews that produced a real revision:

```json
{"prompt":"...","chosen":"...","rejected":"..."}
```

The rejected and chosen answers come from the same request. Records where LaReine
accepted the first draft, or where both answers are identical, are excluded. This shape
can feed preference-training pipelines such as DPO or ORPO after any adapter-specific
conversion required by the trainer.

### Judge distillation

The judge export turns the request and reviewed draft into a user message. LaReine's
verdict, scores, reasoning and critique become the assistant message:

```json
{"messages":[{"role":"user","content":"Request:\n...\n\nDraft answer:\n..."},{"role":"assistant","content":"verdict: revise\nscores: ...\n..."}]}
```

This format is designed to distil LaReine's reviewing behavior into a smaller model.
That smaller model can then score drafts without paying the latency and cost of the
larger judge on every turn.

## A practical collection workflow

1. Run the worker model you want to improve.
2. Configure LaReine with a stronger and, when possible, different model profile.
3. Enable dataset capture and let normal reviewed work accumulate.
4. Inspect the raw JSONL regularly. Remove private, low-value, ambiguous and duplicate
   examples.
5. Export the format that matches the training objective.
6. Create train and validation splits outside LaRuche.
7. Evaluate the resulting model on held-out tasks before replacing the current model.

The relationship between worker and judge matters. Training a model on judgments from
a model with the same weaknesses can reinforce those weaknesses. The cleanest signal
usually comes from a judge that is stronger than the target model.

## Security and data quality

Every text field passes through LaRuche's secret masker before it is written. Exact
values stored in the vault are replaced with `[SECRET:NAME]`. The export endpoint is
restricted to administrators.

This is not a complete privacy scrub. Personal data, proprietary text, copyrighted
material, transformed secrets and sensitive tool output can still be present. Review
the JSONL before copying it to another machine or uploading it to a training service.

LaRuche does not deduplicate, balance, tokenize or split the dataset. It also does not
train a model itself. It captures high-value supervision events and converts them into
portable JSONL; dataset curation and training remain explicit downstream steps.

## Files and naming

- Raw capture: `evals/reine-dataset.jsonl`
- Numeric quality journal: `evals/reine-scorecards.jsonl`
- SFT download: `laruche-sft-N.jsonl`
- Preference download: `laruche-dpo-N.jsonl`
- Judge download: `laruche-judge-N.jsonl`

`N` is the number of exported records after format-specific filtering. The scorecard
journal is useful for trends and model comparisons, but it does not contain enough text
to train a model.
