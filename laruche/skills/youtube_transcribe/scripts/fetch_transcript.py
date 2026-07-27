#!/usr/bin/env python3
"""Fetch the transcript of a YouTube video and print it as text or JSON.

Exits non-zero on every failure, with a one-line reason on stderr, so the caller can
tell "no transcript" apart from "the script itself broke". A script that prints an
error and exits 0 reports success to the agent, which then builds on nothing.

    python fetch_transcript.py "URL_OR_ID"                    # JSON with metadata
    python fetch_transcript.py "URL_OR_ID" --text-only        # plain text
    python fetch_transcript.py "URL_OR_ID" --text-only --timestamps
    python fetch_transcript.py "URL_OR_ID" --language fr,en   # preference order
    python fetch_transcript.py "URL_OR_ID" --list             # available languages
"""

import argparse
import json
import re
import sys

# A transcript carries whatever the captions carry: music notes, CJK, emoji. On Windows
# stdout defaults to the ANSI code page and raises UnicodeEncodeError on the first one,
# after the fetch has already succeeded. Force UTF-8 before writing anything.
for stream in (sys.stdout, sys.stderr):
    if hasattr(stream, "reconfigure"):
        stream.reconfigure(encoding="utf-8", errors="replace")

# Every shape youtube.com hands out, plus the bare 11-character id.
URL_PATTERNS = [
    r"(?:youtube\.com|youtube-nocookie\.com)/watch\?(?:.*&)?v=([0-9A-Za-z_-]{11})",
    r"youtu\.be/([0-9A-Za-z_-]{11})",
    r"youtube\.com/shorts/([0-9A-Za-z_-]{11})",
    r"youtube\.com/live/([0-9A-Za-z_-]{11})",
    r"youtube\.com/embed/([0-9A-Za-z_-]{11})",
    r"youtube\.com/v/([0-9A-Za-z_-]{11})",
]


def fail(message, code=1):
    sys.stderr.write("fetch_transcript: %s\n" % message)
    sys.exit(code)


def video_id(value):
    value = value.strip()
    if re.fullmatch(r"[0-9A-Za-z_-]{11}", value):
        return value
    for pattern in URL_PATTERNS:
        found = re.search(pattern, value)
        if found:
            return found.group(1)
    fail("could not read a video id out of %r" % value)


def timestamp(seconds):
    seconds = int(seconds)
    hours, rest = divmod(seconds, 3600)
    minutes, secs = divmod(rest, 60)
    if hours:
        return "%d:%02d:%02d" % (hours, minutes, secs)
    return "%d:%02d" % (minutes, secs)


def load_api():
    try:
        from youtube_transcript_api import YouTubeTranscriptApi
    except ImportError:
        fail("youtube-transcript-api is not installed. Run: uv pip install youtube-transcript-api")
    return YouTubeTranscriptApi()


def main():
    parser = argparse.ArgumentParser(description="Fetch a YouTube transcript.")
    parser.add_argument("video", help="YouTube URL or bare 11-character video id")
    parser.add_argument("--text-only", action="store_true", help="print plain text, no JSON")
    parser.add_argument("--timestamps", action="store_true", help="prefix each line with MM:SS")
    parser.add_argument(
        "--language",
        default="",
        help="comma-separated preference order, e.g. fr,en. Default: any available.",
    )
    parser.add_argument("--list", action="store_true", help="list available languages and exit")
    args = parser.parse_args()

    ident = video_id(args.video)
    api = load_api()

    # Import the named exceptions lazily: they only exist once the package imported.
    from youtube_transcript_api import (
        CouldNotRetrieveTranscript,
        NoTranscriptFound,
        TranscriptsDisabled,
        VideoUnavailable,
    )

    if args.list:
        try:
            available = api.list(ident)
        except TranscriptsDisabled:
            fail("the owner disabled captions on %s" % ident, 2)
        except VideoUnavailable:
            fail("video %s is private, deleted, or region-locked" % ident, 3)
        except CouldNotRetrieveTranscript as exc:
            fail("%s" % exc, 4)
        for entry in available:
            kind = "auto-generated" if entry.is_generated else "manual"
            print("%s\t%s\t%s" % (entry.language_code, entry.language, kind))
        return

    languages = [code.strip() for code in args.language.split(",") if code.strip()]
    try:
        if languages:
            fetched = api.fetch(ident, languages=languages)
        else:
            # No preference: take whatever the video actually has, rather than
            # defaulting to English and failing on a French-only video.
            listing = api.list(ident)
            first = next(iter(listing))
            fetched = api.fetch(ident, languages=[first.language_code])
    except TranscriptsDisabled:
        fail("the owner disabled captions on %s" % ident, 2)
    except VideoUnavailable:
        fail("video %s is private, deleted, or region-locked" % ident, 3)
    except NoTranscriptFound:
        fail(
            "no transcript in %s. Run with --list to see what exists."
            % ",".join(languages),
            5,
        )
    except CouldNotRetrieveTranscript as exc:
        fail("%s" % exc, 4)
    except StopIteration:
        fail("video %s has no transcript in any language" % ident, 5)

    snippets = fetched.to_raw_data()
    if not snippets:
        fail("the transcript for %s came back empty" % ident, 5)

    # Captions wrap mid-sentence, so a snippet often contains its own newlines. Collapse
    # them, otherwise --timestamps stops being one line per segment and the caller's
    # line-based parsing silently drifts.
    texts = [" ".join(part["text"].split()) for part in snippets]
    plain = " ".join(text for text in texts if text)
    stamped = "\n".join(
        "%s %s" % (timestamp(part["start"]), text)
        for part, text in zip(snippets, texts)
        if text
    )
    last = snippets[-1]
    duration = timestamp(last["start"] + last.get("duration", 0))

    if args.text_only:
        sys.stdout.write((stamped if args.timestamps else plain) + "\n")
        return

    payload = {
        "video_id": ident,
        "segment_count": len(snippets),
        "duration": duration,
        "full_text": plain,
    }
    if args.timestamps:
        payload["timestamped_text"] = stamped
    sys.stdout.write(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
