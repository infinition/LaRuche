---
type: skill
name: llama-cpp
description: >-
  Local GGUF inference + HF Hub model discovery via llama.cpp.
---

# llama.cpp + GGUF

Use this skill for local GGUF inference, quant selection, or Hugging Face repo discovery for llama.cpp.

## When to use

- Run local models on CPU, Apple Silicon, CUDA, ROCm, or Intel GPUs
- Find the right GGUF for a specific Hugging Face repo
- Build a `llama-server` or `llama-cli` command from the Hub
- Search the Hub for models that support llama.cpp
- Enumerate available `.gguf` files and sizes for a repo
- Decide between Q4/Q5/Q6/IQ variants for the user's RAM or VRAM

## Model Discovery workflow

Prefer URL workflows before falling back to Python or custom scripts. Use `web_fetch` for all URL steps.

1. **Search candidate repos** via `web_fetch`:
   - `https://huggingface.co/models?apps=llama.cpp&sort=trending`
   - Add `search=<term>` for a model family
   - Add `num_parameters=min:0,max:24B` when the user has size constraints

2. **Open the repo local-app view**:
   - `https://huggingface.co/<repo>?local-app=llama.cpp`
   - If the snippet is text-visible, copy the exact `llama-server` / `llama-cli` command and recommended quant as shown.
   - Extract the `Hardware compatibility` section - prefer its exact quant labels (e.g., `UD-Q4_K_M`, `IQ4_NL_XL`) over generic tables.

3. **Query the tree API** to confirm what actually exists:
   - `https://huggingface.co/api/models/<repo>/tree/main?recursive=true`
   - Keep entries where `type` is `file` and `path` ends with `.gguf`.
   - Use `path` and `size` as the source of truth for filenames and byte sizes.
   - Separate quantized checkpoints from `mmproj-*.gguf` projector files and `BF16/` shard files.

4. **Reconstruct the command** if the local-app snippet is not visible:
   - Shorthand: `llama-server -hf <repo>:<QUANT>`
   - Exact file: `llama-server --hf-repo <repo> --hf-file <filename.gguf>`

5. Only suggest conversion from Transformers weights if the repo exposes no GGUF files.

## Install llama.cpp

Run via `shell_exec`:

```bash
# macOS / Linux
brew install llama.cpp

# Windows
winget install llama.cpp

# Build from source (all platforms)
git clone https://github.com/ggml-org/llama.cpp
cd llama.cpp
cmake -B build
cmake --build build --config Release
```

## Run from the Hub

```bash
# Shorthand (quant tag)
llama-cli -hf bartowski/Llama-3.2-3B-Instruct-GGUF:Q8_0
llama-server -hf bartowski/Llama-3.2-3B-Instruct-GGUF:Q8_0

# Exact file (when tree API shows custom naming)
llama-server \
    --hf-repo microsoft/Phi-3-mini-4k-instruct-gguf \
    --hf-file Phi-3-mini-4k-instruct-q4.gguf \
    -c 4096
```

## Verify the server

```bash
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"messages":[{"role":"user","content":"Write a limerick about Python exceptions"}]}'
```

If the server isn't responding: check the port with `llama-server --port 8080`, and confirm the model loaded without OOM by inspecting stderr output.

## Python bindings (llama-cpp-python)

Install via `shell_exec`:

```bash
pip install llama-cpp-python
# CUDA:  CMAKE_ARGS="-DGGML_CUDA=on" pip install llama-cpp-python --force-reinstall --no-cache-dir
# Metal: CMAKE_ARGS="-DGGML_METAL=on" pip install llama-cpp-python --force-reinstall --no-cache-dir
```

Use with `execute_code`:

```python
from llama_cpp import Llama

# Basic generation
llm = Llama(model_path="./model-q4_k_m.gguf", n_ctx=4096, n_gpu_layers=35, n_threads=8)
out = llm("What is machine learning?", max_tokens=256, temperature=0.7)
print(out["choices"][0]["text"])

# Chat completion
llm2 = Llama(model_path="./model-q4_k_m.gguf", n_ctx=4096, n_gpu_layers=35, chat_format="llama-3")
resp = llm2.create_chat_completion(
    messages=[
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "What is Python?"},
    ],
    max_tokens=256,
)
print(resp["choices"][0]["message"]["content"])

# Streaming
for chunk in llm("Explain quantum computing:", max_tokens=256, stream=True):
    print(chunk["choices"][0]["text"], end="", flush=True)

# Embeddings
llm3 = Llama(model_path="./model-q4_k_m.gguf", embedding=True, n_gpu_layers=35)
vec = llm3.embed("This is a test sentence.")
print(f"Embedding dimension: {len(vec)}")

# Load directly from Hub
llm4 = Llama.from_pretrained(
    repo_id="bartowski/Llama-3.2-3B-Instruct-GGUF",
    filename="*Q4_K_M.gguf",
    n_gpu_layers=35,
)
```

**Common failure**: `llama_cpp` raises `ValueError: Model file not found` - verify the path or the `filename` glob pattern against the tree API output.

## Choosing a quant

- Prefer the exact quant HF marks as compatible for the user's hardware.
- General chat: `Q4_K_M`
- Code / technical: `Q5_K_M` or `Q6_K` if memory allows
- Tight RAM: `Q3_K_M` or `IQ` variants - only if the user prioritizes fit over quality
- Multimodal repos: mention `mmproj-*.gguf` separately - it is the vision projector, not the main model
- Do not normalize repo-native labels: if HF says `UD-Q4_K_M`, report `UD-Q4_K_M`

## Output format for discovery requests

```text
Repo: <repo>
Recommended quant from HF: <label> (<size>)
llama-server: <command>
Other GGUFs:
- <filename> - <size> [projector?]
Source URLs:
- <local-app URL>
- <tree API URL>
```

## Key URLs

```text
https://huggingface.co/models?apps=llama.cpp&sort=trending
https://huggingface.co/models?search=<term>&apps=llama.cpp&sort=trending
https://huggingface.co/models?search=<term>&apps=llama.cpp&num_parameters=min:0,max:24B&sort=trending
https://huggingface.co/<repo>?local-app=llama.cpp
https://huggingface.co/api/models/<repo>/tree/main?recursive=true
```

## External references

- GitHub: https://github.com/ggml-org/llama.cpp
- HF GGUF + llama.cpp docs: https://huggingface.co/docs/hub/gguf-llamacpp
- HF Local Apps docs: https://huggingface.co/docs/hub/main/local-apps
