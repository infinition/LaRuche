---
type: skill
name: youtube_transcribe
description: Fetch the transcript of a YouTube video as text or JSON.
---

# YouTube transcribe

Turn a YouTube URL into the words that were said, so you can summarise, quote or search
them. The transcript comes from YouTube's own caption track, not from audio: it costs
nothing, takes a second, and exists for most videos with more than a handful of views.

Use it whenever the user points at a video and asks what is in it. Do not try to watch
the video, do not download the audio, and do not answer from the title.

## Prerequisites

```bash
uv pip install youtube-transcript-api
```

Verify, from the skill folder:

```bash
python scripts/fetch_transcript.py "dQw4w9WgXcQ" --list
```

Success prints one line per available caption track: language code, language name, and
whether it is manual or auto-generated. If it prints
`youtube-transcript-api is not installed`, run the install above and retry. That install
is your job, not a question for the user.

## Procedure

1. **Get the video id right.** The bundled script accepts every YouTube URL shape
   (`watch?v=`, `youtu.be/`, `/shorts/`, `/live/`, `/embed/`, `/v/`) and a bare
   11-character id, so pass whatever the user gave you, unmodified and quoted.
2. **Check what languages exist**, when the video is not obviously English:
   `python scripts/fetch_transcript.py "<URL>" --list`.
3. **Fetch it.** With no `--language`, the script takes the FIRST track the video
   actually has, rather than assuming English and failing on a French-only video.

   ```bash
   python scripts/fetch_transcript.py "<URL>" --text-only
   python scripts/fetch_transcript.py "<URL>" --text-only --timestamps
   python scripts/fetch_transcript.py "<URL>"
   python scripts/fetch_transcript.py "<URL>" --language fr,en
   ```

4. **Check the exit code, not the shape of the output.** Every failure exits non-zero
   with one line on stderr. If stdout is empty, read stderr before concluding anything.
5. **Use the text.** For a long video, write it to a file with `file_write` before
   summarising, so a context compaction does not take the transcript with it.

Run all of these through `shell_exec`, from `skills/youtube_transcribe/`, or with the
absolute path to the script.

## Output

| Invocation | stdout |
|---|---|
| `--text-only` | one paragraph, caption line breaks collapsed |
| `--text-only --timestamps` | `M:SS text`, exactly one line per segment |
| default | JSON: `video_id`, `segment_count`, `duration`, `full_text` |
| `--timestamps` without `--text-only` | the JSON above plus `timestamped_text` |
| `--list` | three tab-separated columns: code, language, manual or auto-generated |

## Traps

- **A transcript is not a summary.** It is unpunctuated, repetitive, and full of filler.
  Read it, then write the answer. Pasting it back at the user is not doing the work.
- **Auto-generated tracks mishear names and numbers.** `--list` tells you which kind you
  got. Before quoting a figure or a proper noun from an auto-generated track, say that it
  came from automatic captions, or verify it elsewhere.
- **Timestamps mark the START of a segment**, so a quote usually spans the segment after
  it too. Widen the window before citing a moment.
- **Non-Latin characters crash a naive script on Windows.** The bundled one forces UTF-8
  on stdout. If you write your own, do the same, or the fetch succeeds and the print
  fails with `UnicodeEncodeError` after all the work is done.
- **A very long video produces a very long string.** A three hour talk is hundreds of
  kilobytes. Write it to disk, then work on it in pieces.

## Failure modes

**Exit 2, `the owner disabled captions`.** There is no workaround at this layer. Tell the
user the video has captions turned off, and offer to work from the description or from a
search about the video instead.

**Exit 3, `private, deleted, or region-locked`.** Re-read the URL you were given; a
truncated id is the usual cause. If the URL is right, the video is not reachable from
here.

**Exit 5, `no transcript in <codes>`.** Your `--language` list matches nothing. Run
`--list` and use a code from the output. Codes are specific: `pt-BR` is not `pt`.

**Exit 4, anything else from the API.** Usually YouTube rate limiting or blocking the
request from this IP. Wait and retry once. If it persists, say so rather than looping.

**`ModuleNotFoundError` despite the install.** The install went to a different interpreter
than the one running the script. Run `python -m pip install youtube-transcript-api` with
the same `python` you invoke the script with.
