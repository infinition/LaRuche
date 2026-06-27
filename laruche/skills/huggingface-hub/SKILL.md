---
type: skill
name: huggingface-hub
description: "HuggingFace hf CLI: search/download/upload models, datasets."
version: 1.0.0
author: Hugging Face
license: MIT
tags: [huggingface, hf, models, datasets, hub, mlops]
platforms: [linux, macos, windows]
tools: [shell_exec]
---

# Hugging Face CLI (`hf`) Reference Guide

The `hf` command is the modern CLI for the Hugging Face Hub. It replaces the deprecated `huggingface-cli`.

**Installation:** `curl -LsSf https://hf.co/cli/install.sh | bash -s`
**Auth:** Use `${HF_TOKEN}` (LaRuche secrets vault) or pass `--token ${HF_TOKEN}` to any command.
**Help:** `hf --help` — full command list with examples.

In LaRuche, run all `hf` commands via `shell_exec`.

---

## Core Commands

### Download / Upload
- `hf download REPO_ID` — download files from Hub
- `hf upload REPO_ID [LOCAL_PATH]` — single-commit upload
- `hf upload-large-folder REPO_ID LOCAL_PATH` — resumable upload for large dirs
- `hf sync` — sync local dir with a bucket

### Auth (`hf auth`)
- `hf auth login` / `hf auth logout`
- `hf auth list` / `hf auth switch` — manage multiple tokens
- `hf auth whoami`

### Repos (`hf repos`)
- `hf repos create REPO_ID [--type dataset|model|space]`
- `hf repos delete REPO_ID`
- `hf repos duplicate SRC_ID DEST_ID`
- `hf repos move REPO_ID NEW_ID`
- `hf repos branch` / `hf repos tag` — Git-like refs
- `hf repos delete-files REPO_ID PATTERN`

---

## Specialized Commands

### Datasets & Models
- `hf datasets list [--search QUERY]` / `hf datasets info REPO_ID`
- `hf datasets parquet REPO_ID` — list parquet URLs
- `hf datasets sql "SELECT ..." REPO_ID` — SQL via DuckDB on parquet
- `hf models list [--search QUERY]` / `hf models info REPO_ID`
- `hf papers list` — daily papers

### Discussions & PRs (`hf discussions`)
- `hf discussions list REPO_ID`
- `hf discussions create REPO_ID --title "..."` / `hf discussions comment`
- `hf discussions diff / merge / close / reopen / rename`

### Inference Endpoints
- `hf endpoints deploy / pause / resume / scale-to-zero / catalog`

### Jobs
- `hf jobs uv SCRIPT.py` — run Python script with inline deps on HF infra
- `hf jobs stats` — resource monitoring

### Spaces
- `hf spaces dev-mode REPO_ID` / `hf spaces hot-reload FILE` — iterate without full restart

### Buckets (S3-like)
- `hf buckets create / cp / mv / rm / sync`

### Cache
- `hf cache list` / `hf cache prune` / `hf cache verify`

### Webhooks & Collections
- `hf webhooks create / watch / enable / disable`
- `hf collections add-item / update / list`

---

## Global Flags
- `--format json` — machine-readable output (use for parsing in LaRuche)
- `-q` / `--quiet` — IDs only

## Extensions
- `hf extensions install GITHUB_REPO_ID` — extend CLI functionality
- `hf skills add` — add AI assistant skills

---

## LaRuche Usage Pattern

```
# Search and download a model
shell_exec("hf models list --search mistral --format json")
shell_exec("hf download mistralai/Mistral-7B-v0.1 --local-dir /data/models/mistral")

# Upload a dataset
shell_exec("hf upload my-org/my-dataset ./local-data --repo-type dataset")

# SQL query on a public dataset
shell_exec('hf datasets sql "SELECT * FROM train LIMIT 10" datasets/glue')
```

## Pitfalls
- `${HF_TOKEN}` must be set in LaRuche secrets vault before any authenticated operation; missing token gives a 401, not a clear error. Always inject via `HF_TOKEN=${HF_TOKEN} hf ...` in shell_exec.
- `hf upload` does a single commit — for >10 GB dirs always use `hf upload-large-folder` (supports resume).
- `hf datasets sql` requires DuckDB to be installed locally; it is NOT a server-side query.
- Windows paths in `shell_exec` should use forward slashes or raw strings to avoid escape issues.
