---
type: skill
name: youtube-transcribe
description: "Fetch raw YouTube transcript (text or JSON) via youtube-transcript-api."
platforms: [linux, macos, windows]
tools: [shell_exec]
scripts: []
dependencies:
  python: ">=3.9"
  uv_packages: [youtube-transcript-api]
---

# YouTube Transcribe

## When to use

Use when the user wants the raw transcript of a YouTube video - no formatting, no summarization. For downstream content generation (summaries, threads, chapters), use the `youtube-content` skill instead.

## Setup

Install the dependency once:

```bash
uv pip install youtube-transcript-api
```

## Script

The shared helper lives at `../youtube-content/scripts/fetch_transcript.py`. It handles all standard YouTube URL formats (watch?v=, youtu.be/, /shorts/, /live/, /embed/), as well as raw 11-character video IDs.

## Commands

```bash
# Plain text - best for direct reading or further processing
uv run python3 ../youtube-content/scripts/fetch_transcript.py "URL" --text-only

# Timestamped plain text
uv run python3 ../youtube-content/scripts/fetch_transcript.py "URL" --text-only --timestamps

# Full JSON with metadata (video_id, segment_count, duration, full_text)
uv run python3 ../youtube-content/scripts/fetch_transcript.py "URL"

# Force a specific language (comma-separated fallback chain)
uv run python3 ../youtube-content/scripts/fetch_transcript.py "URL" --language en,fr
```

Run via `shell_exec`. Replace `URL` with the full YouTube URL or a bare video ID.

## Output

- `--text-only`: plain string, ready to pipe or display.
- `--text-only --timestamps`: `MM:SS text` lines, one per segment.
- JSON (default): `{"video_id": "...", "segment_count": N, "duration": "M:SS", "full_text": "..."}`. Add `--timestamps` to include `"timestamped_text"` in JSON.

## Error Handling

| Error | Cause | Fix |
|---|---|---|
| `Transcripts are disabled` | Video owner disabled captions | Inform the user; no workaround |
| `No transcript found` | Language mismatch | Retry without `--language` to accept any available language |
| `youtube-transcript-api not installed` | Missing dep | Run `uv pip install youtube-transcript-api` and retry |
| Private/unavailable video | Bad URL or private video | Ask the user to verify the URL |
| Exit code 1 with JSON error object | Script-level failure | Print the JSON error to the user and follow the table above |
