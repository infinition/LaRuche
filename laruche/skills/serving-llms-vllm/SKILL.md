---
type: skill
name: serving-llms-vllm
description: "Deploy high-throughput LLMs via vLLM: OpenAI API, quantization, multi-GPU."
version: 1.0.0
author: Orchestra Research
license: MIT
dependencies: [vllm, torch, transformers]
platforms: [linux, macos]
tools: [shell_exec, execute_code, file_write, file_read]
metadata:
  laruche:
    tags: [vLLM, Inference Serving, PagedAttention, Continuous Batching, High Throughput, Production, OpenAI API, Quantization, Tensor Parallelism]

---

# vLLM — High-Performance LLM Serving

## When to use

Deploy production LLM APIs, serve OpenAI-compatible endpoints, or fit large models into limited GPU memory via quantization. vLLM achieves high throughput through PagedAttention (block-based KV cache) and continuous batching.

**Prefer alternatives when:**
- CPU/edge/single-user: use **llama.cpp**
- Research/prototyping: use **HuggingFace transformers**
- NVIDIA-only, max perf: use **TensorRT-LLM**

## Installation

```bash
pip install vllm
```

## Quick offline inference

```python
from vllm import LLM, SamplingParams

llm = LLM(model="meta-llama/Llama-3-8B-Instruct")
sampling = SamplingParams(temperature=0.7, max_tokens=256)
outputs = llm.generate(["Explain quantum computing"], sampling)
print(outputs[0].outputs[0].text)
```

## OpenAI-compatible server

```bash
vllm serve meta-llama/Llama-3-8B-Instruct --port 8000
```

Query it:
```bash
curl http://localhost:8000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model": "meta-llama/Llama-3-8B-Instruct", "messages": [{"role": "user", "content": "Hello!"}]}'
```

## Production deployment

### Single GPU (7B–13B models)

```bash
vllm serve meta-llama/Llama-3-8B-Instruct \
  --gpu-memory-utilization 0.9 \
  --max-model-len 8192 \
  --enable-prefix-caching \
  --enable-metrics \
  --metrics-port 9090 \
  --host 0.0.0.0 \
  --port 8000
```

### Multi-GPU (30B–70B models)

```bash
vllm serve meta-llama/Llama-2-70b-hf \
  --tensor-parallel-size 4 \
  --gpu-memory-utilization 0.9 \
  --quantization awq \
  --port 8000
```

Tensor parallelism requires power-of-2 GPU count (2, 4, 8).

### Docker

```bash
docker run --gpus all -p 8000:8000 \
  vllm/vllm-openai:latest \
  --model meta-llama/Llama-3-8B-Instruct \
  --gpu-memory-utilization 0.9 \
  --enable-prefix-caching
```

## Batch inference (offline, large datasets)

```python
from vllm import LLM, SamplingParams
import json

with open("prompts.txt") as f:
    prompts = [line.strip() for line in f]

llm = LLM(
    model="meta-llama/Llama-3-8B-Instruct",
    tensor_parallel_size=2,
    gpu_memory_utilization=0.9,
    max_model_len=4096
)
sampling = SamplingParams(temperature=0.7, top_p=0.95, max_tokens=512)

outputs = llm.generate(prompts, sampling)  # vLLM batches internally

with open("results.jsonl", "w") as f:
    for out in outputs:
        f.write(json.dumps({
            "prompt": out.prompt,
            "generated": out.outputs[0].text,
            "tokens": len(out.outputs[0].token_ids)
        }) + "\n")
```

## Quantization (fit large models in limited VRAM)

| Method | Best for | Hardware |
|--------|----------|----------|
| AWQ | 70B models, minimal accuracy loss | Any |
| GPTQ | Wide model support | Any |
| FP8 | Fastest inference | H100 only |

Use pre-quantized models from HuggingFace (e.g. `TheBloke/Llama-2-70B-AWQ`):

```bash
vllm serve TheBloke/Llama-2-70B-AWQ \
  --quantization awq \
  --gpu-memory-utilization 0.95
# Fits 70B in ~40GB VRAM
```

## Monitoring (Prometheus)

```bash
curl http://localhost:9090/metrics | grep vllm
```

Key metrics:
- `vllm:time_to_first_token_seconds` — latency
- `vllm:num_requests_running` — active requests
- `vllm:gpu_cache_usage_perc` — KV cache utilization

Target: TTFT < 500ms, GPU utilization > 80%.

## Common issues

**OOM during model loading**
```bash
vllm serve MODEL --gpu-memory-utilization 0.7 --max-model-len 4096
# Or add --quantization awq
```

**High TTFT (> 1s)**
```bash
vllm serve MODEL --enable-prefix-caching   # for repeated prompts
vllm serve MODEL --enable-chunked-prefill  # for long prompts
```

**Low throughput (< 50 req/sec)**
```bash
vllm serve MODEL --max-num-seqs 512
# Check: nvidia-smi — GPU util should be > 80%
```

**Model not found / custom architecture**
```bash
vllm serve MODEL --trust-remote-code
```

**Speculative decoding (faster generation)**
```bash
vllm serve MODEL --speculative-model DRAFT_MODEL
```

## Hardware reference

| Model size | Minimum GPU |
|------------|-------------|
| 7B–13B | 1× A10 (24GB) or A100 (40GB) |
| 30B–40B | 2× A100 (40GB) with tensor parallelism |
| 70B+ | 4× A100 (40GB) or 2× A100 (80GB) + AWQ/GPTQ |

Supported: NVIDIA (primary), AMD ROCm, Intel GPUs, TPUs.

## Resources

- Docs: https://docs.vllm.ai
- GitHub: https://github.com/vllm-project/vllm
- Community: https://discuss.vllm.ai
