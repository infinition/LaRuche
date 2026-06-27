---
type: skill
name: evaluating-llms-harness
description: "Benchmark LLMs across 60+ tasks (MMLU, GSM8K, HumanEval)."
version: 1.0.1
author: Orchestra Research
license: MIT
dependencies: [lm-eval, transformers, vllm]
platforms: [linux, macos]
tools: [shell_exec, execute_code, file_write]
metadata:
  laruche:
    tags: [Evaluation, LM Evaluation Harness, Benchmarking, MMLU, HumanEval, GSM8K, EleutherAI, Model Quality, Academic Benchmarks, Industry Standard]

---

# lm-evaluation-harness — LLM Benchmarking

Evaluates LLMs across 60+ academic benchmarks (MMLU, HumanEval, GSM8K, TruthfulQA, HellaSwag) with standardized prompts and metrics. Industry standard used by EleutherAI, HuggingFace, and major labs.

## Installation

```bash
pip install lm-eval
pip install vllm   # optional: 5-10x faster inference
```

Run via `shell_exec`.

## Core commands

**List available tasks:**
```bash
lm_eval --tasks list
```

**Evaluate a HuggingFace model (standard suite):**
```bash
lm_eval --model hf \
  --model_args pretrained=meta-llama/Llama-2-7b-hf,dtype=bfloat16 \
  --tasks mmlu,gsm8k,hellaswag,truthfulqa,arc_challenge \
  --num_fewshot 5 \
  --batch_size auto \
  --output_path results/my-model-eval.json \
  --log_samples
```

**Evaluate with vLLM backend (5-10x faster):**
```bash
lm_eval --model vllm \
  --model_args pretrained=meta-llama/Llama-2-7b-hf,tensor_parallel_size=2,dtype=auto,gpu_memory_utilization=0.8 \
  --tasks mmlu,gsm8k,hellaswag,truthfulqa \
  --num_fewshot 5 \
  --batch_size auto \
  --output_path results/my-model-eval.json
```

**Evaluate a local checkpoint:**
```bash
lm_eval --model hf \
  --model_args pretrained=/path/to/checkpoint,tokenizer=/path/to/tokenizer \
  --tasks gsm8k,hellaswag \
  --num_fewshot 0 \
  --batch_size auto \
  --output_path results/checkpoint-eval.json
```

**Quantized model (8-bit or 4-bit):**
```bash
lm_eval --model hf \
  --model_args pretrained=model-name,load_in_8bit=True \
  --tasks mmlu \
  --device cuda:0
```

## Benchmarks reference

| Task | What it measures | Notes |
|------|-----------------|-------|
| `mmlu` | 57-subject knowledge (multiple choice) | ~2h on 7B, use 5-shot |
| `gsm8k` | Grade school math word problems | ~5min on 7B |
| `hellaswag` | Common sense reasoning | ~10min on 7B |
| `truthfulqa` | Truthfulness and factuality | |
| `arc_challenge` | Science reasoning | |
| `humaneval` | Python code generation (164 problems) | Requires `--allow_code_execution` + `pip install human-eval` |
| `mbpp` | Python coding basics | |
| `mmlu_stem` | MMLU subset — STEM subjects only | Faster than full MMLU |

Standard 5-shot is the paper default for MMLU; use `--num_fewshot 0` for speed during training runs.

## Workflow: Compare multiple models

Write the loop script with `file_write`, then run it with `shell_exec`:

```bash
TASKS="mmlu,gsm8k,hellaswag,truthfulqa"
for model in meta-llama/Llama-2-7b-hf mistralai/Mistral-7B-v0.1 microsoft/phi-2; do
    model_name=$(echo $model | sed 's/\//-/g')
    lm_eval --model vllm \
      --model_args pretrained=$model,dtype=bfloat16 \
      --tasks $TASKS \
      --num_fewshot 5 \
      --batch_size auto \
      --output_path results/$model_name.json
done
```

Parse results with `execute_code` (Python):
```python
import json, glob

for path in sorted(glob.glob("results/*.json")):
    data = json.load(open(path))
    model = data["config"]["model_args"].split("pretrained=")[1].split(",")[0]
    print(f"\n{model}")
    for task, metrics in data["results"].items():
        score = metrics.get("acc") or metrics.get("exact_match") or metrics.get("acc_norm")
        print(f"  {task}: {score:.3f}")
```

## Pitfalls & troubleshooting

**Out of memory:**
```bash
--batch_size 1                                                         # reduce batch
--model_args pretrained=model,load_in_8bit=True                        # quantize
--model_args pretrained=model,device_map=auto,offload_folder=offload   # CPU offload
```

**Wrong results vs. paper:**
- Match fewshot count: most papers use `--num_fewshot 5`
- Use exact task name (`mmlu` not `mmlu_direct`)
- Ensure tokenizer matches model: `--model_args pretrained=X,tokenizer=X`

**HumanEval requires code execution:**
```bash
pip install human-eval
lm_eval ... --tasks humaneval --allow_code_execution
```

**Evaluation too slow:** Switch to `--model vllm` (5-10x speedup) or subset tasks (`mmlu_stem`).

## Output format

Results land in the `--output_path` JSON:
```json
{
  "results": {
    "mmlu": {"acc": 0.459, "acc_stderr": 0.004},
    "gsm8k": {"exact_match": 0.142, "exact_match_stderr": 0.006}
  },
  "config": {"model": "hf", "num_fewshot": 5}
}
```

## Resources

- GitHub: https://github.com/EleutherAI/lm-evaluation-harness
- HF Leaderboard: https://huggingface.co/spaces/HuggingFaceH4/open_llm_leaderboard
