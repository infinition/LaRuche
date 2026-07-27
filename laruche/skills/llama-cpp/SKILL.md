---
type: skill
name: llama-cpp
description: Run a GGUF model locally with llama.cpp, and find one on the HF Hub.
---

# llama.cpp and GGUF

Run a model on this machine: no account, no per-token cost, nothing leaving the disk.
llama.cpp does the running; GGUF is the single-file format it reads; the quantisation
level is the dial that trades quality for memory.

Two jobs live here. Finding the right file on the Hub, which is mostly a research task,
and running it, which is mostly an arithmetic one about how much memory the user has.

## Finding the file

The Hub's web pages are the map, but **the tree API is the territory.** A model card can
list quants that were renamed, split or never uploaded. Confirm against the API before
handing anyone a command.

1. **Find candidate repositories**, with `web_fetch`:

   ```
   https://huggingface.co/models?apps=llama.cpp&sort=trending
   https://huggingface.co/models?search=<term>&apps=llama.cpp&sort=trending
   https://huggingface.co/models?search=<term>&apps=llama.cpp&num_parameters=min:0,max:24B&sort=trending
   ```

   `apps=llama.cpp` is the filter that matters: it excludes everything with no GGUF.

2. **Open the repository's local-app view**, which carries the maintainer's own command:

   ```
   https://huggingface.co/<repo>?local-app=llama.cpp
   ```

   If the snippet is readable, take the command and the recommended quant verbatim. The
   `Hardware compatibility` block is better than any general table here, because it was
   written against these specific files.

3. **Confirm against the tree API**, which lists what actually exists:

   ```
   https://huggingface.co/api/models/<repo>/tree/main?recursive=true
   ```

   It returns a JSON array; keep entries whose `type` is `file` and whose `path` ends in
   `.gguf`. `path` and `size` are the truth for filenames and bytes.

   Three kinds of file show up together and must not be confused:

   - the quantised checkpoints, which is what you want;
   - `mmproj-*.gguf`, the vision projector for a multimodal model, which is loaded
     ALONGSIDE the main file and is useless alone;
   - shards such as `BF16/` or `*-00001-of-0000N.gguf`, an unquantised model split across
     files.

4. **Build the command.** Shorthand when the quant tag is standard, exact file when the
   repository names things its own way:

   ```bash
   llama-server -hf bartowski/Llama-3.2-3B-Instruct-GGUF:Q4_K_M
   llama-server --hf-repo <repo> --hf-file <exact-name-from-the-tree-api.gguf> -c 4096
   ```

5. Suggest converting from Transformers weights only when the repository publishes no
   GGUF at all. It is a long job and someone has usually already done it: search for
   `<model name> GGUF` first.

**Report the label exactly as the repository writes it.** `UD-Q4_K_M` and `IQ4_NL_XL` are
real, specific names. Normalising one to `Q4_K_M` produces a command that downloads
nothing.

## Install

```bash
brew install llama.cpp      # macOS, Linux
winget install llama.cpp    # Windows
```

From source, when you need a specific backend compiled in:

```bash
git clone https://github.com/ggml-org/llama.cpp
cd llama.cpp
cmake -B build
cmake --build build --config Release
```

Verify with `llama-cli --version` before anything else. A missing binary and a failed
model load produce different errors, and confusing them costs an hour.

## The arithmetic that decides everything

Before recommending a quant, work out what fits. Roughly:

- **The file size is the floor**, not the total. Add context on top.
- **Context costs memory too**, and it grows with `-c`. A large context window on a small
  machine is what turns a working setup into an out-of-memory crash halfway through a
  conversation.
- **Leave headroom.** A model whose file is 90% of available RAM will swap, and a swapping
  model is slower than the CPU-only path it was supposed to beat.

Then pick:

| Situation | Quant |
|---|---|
| General chat, the default worth starting from | `Q4_K_M` |
| Code or anything technical, if memory allows | `Q5_K_M`, `Q6_K` |
| It must fit, quality second | `Q3_K_M`, or an `IQ` variant |
| Memory is not a constraint | `Q8_0` |

`IQ` quants are smaller at equal quality but need more compute to unpack, so on a slow CPU
they can be the wrong trade. Below `Q3` the model degrades in ways that look like
stupidity rather than compression.

If the local-app view named a quant for the user's hardware, prefer it over this table.

## Serving

```bash
llama-server -hf <repo>:<QUANT> -c 4096 --port 8080
```

It exposes an OpenAI-compatible API, which is what lets existing clients talk to it
unchanged:

```bash
curl -s http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"messages":[{"role":"user","content":"say hello"}]}'
```

A JSON reply with a `choices` array means the model is loaded and serving. Anything else,
read stderr: llama-server reports the load there, including the layer count it offloaded
and the memory it took.

## Python bindings

```bash
pip install llama-cpp-python

CMAKE_ARGS="-DGGML_CUDA=on"  pip install llama-cpp-python --force-reinstall --no-cache-dir
CMAKE_ARGS="-DGGML_METAL=on" pip install llama-cpp-python --force-reinstall --no-cache-dir
```

The plain install is CPU-only. GPU support is compiled in, so switching backends means
reinstalling with `--force-reinstall --no-cache-dir`: without those flags pip serves the
cached CPU wheel and the GPU is silently never used.

```python
from llama_cpp import Llama

llm = Llama(model_path="C:/models/model-q4_k_m.gguf",
            n_ctx=4096, n_gpu_layers=35, n_threads=8)

reply = llm.create_chat_completion(
    messages=[{"role": "user", "content": "what is a GGUF file"}],
    max_tokens=256,
)
print(reply["choices"][0]["message"]["content"])

for chunk in llm("explain quantisation:", max_tokens=256, stream=True):
    print(chunk["choices"][0]["text"], end="", flush=True)

hub = Llama.from_pretrained(repo_id="bartowski/Llama-3.2-3B-Instruct-GGUF",
                            filename="*Q4_K_M.gguf", n_gpu_layers=35)
```

`n_gpu_layers` is how many transformer layers move to the GPU. `0` is CPU-only, `-1` is
all of them, and a number too high for the available VRAM fails at load rather than
falling back.

## Reporting a discovery

```text
Repo: <repo>
Recommended: <label exactly as published> (<size>)
Command: llama-server -hf <repo>:<label> -c 4096
Also available:
  <filename>  <size>
  mmproj-<...>.gguf  <size>   (vision projector, load with the main model)
Sources:
  https://huggingface.co/<repo>?local-app=llama.cpp
  https://huggingface.co/api/models/<repo>/tree/main?recursive=true
```

Give the sizes. "Q4_K_M is recommended" is not actionable; "Q4_K_M, 2.0 GB" is.

## Traps

- **A quant on the model card that does not exist as a file.** Cards go stale. The tree
  API does not.
- **Downloading before checking the size.** These are gigabytes on someone's connection
  and disk. State the number first.
- **`mmproj-*.gguf` treated as the model.** It is the vision half and produces nothing
  alone.
- **Normalising a repository's own quant label.** It is part of the filename.
- **Reinstalling llama-cpp-python without `--no-cache-dir`.** pip serves the cached
  CPU build and the GPU flags are silently ignored.
- **Raising `-c` to the model's maximum by reflex.** Context is memory. A 128k window on a
  laptop is an out-of-memory error waiting for a long conversation.

## Failure modes

**`llama-server: command not found`.** Not installed, or not on PATH after a source build:
the binaries land in `build/bin/`.

**The model loads, then the process dies.** Out of memory. Lower `-c` first, since it is
free to change, then drop to a smaller quant.

**Generation is far slower than expected, and the GPU is idle.** `n_gpu_layers` is 0, or
the wheel is the CPU build. Read llama-server's stderr: it prints how many layers were
offloaded.

**`ValueError: Model file not found`.** The path or the `filename` glob matches nothing.
Check it against the tree API output rather than guessing at the pattern.

**The download stops partway, repeatedly.** Large files over an unstable connection. Fetch
it with `curl -C -` to resume, and point `--hf-file` at the local path instead.

**Output is fluent but wrong in a way a smaller model would not be.** The quant is too
aggressive for the task. Move up one level before concluding the model is unsuitable.
