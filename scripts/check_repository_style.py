#!/usr/bin/env python3
"""Reject forbidden typography and commit trailers.

With no arguments, scans every tracked text file in the current checkout.
With ``--commits BASE HEAD``, also scans messages introduced by BASE..HEAD.
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys


REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FORBIDDEN_DASH = chr(0x2014)
COAUTHOR = "co-" + "authored-by"


def git(*args: str) -> bytes:
    return subprocess.check_output(["git", *args], cwd=REPO)


def tracked_files() -> list[str]:
    raw = git("ls-files", "-z")
    return [part.decode("utf-8") for part in raw.split(b"\0") if part]


def check_files() -> list[str]:
    errors: list[str] = []
    for relative in tracked_files():
        path = os.path.join(REPO, relative)
        try:
            with open(path, "rb") as handle:
                raw = handle.read()
            text = raw.decode("utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        if FORBIDDEN_DASH in text:
            errors.append(f"{relative}: forbidden U+2014 character")
        if COAUTHOR in text.casefold():
            errors.append(f"{relative}: forbidden co-author trailer")
    return errors


def valid_commit(value: str) -> bool:
    return (
        bool(value)
        and set(value) != {"0"}
        and subprocess.run(
            ["git", "cat-file", "-e", f"{value}^{{commit}}"],
            cwd=REPO,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        ).returncode
        == 0
    )


def check_commits(base: str, head: str) -> list[str]:
    if not valid_commit(head):
        return [f"invalid HEAD commit: {head}"]
    revision = f"{base}..{head}" if valid_commit(base) else head
    raw = git("log", "--format=%H%x00%B%x00", revision)
    parts = raw.decode("utf-8", errors="replace").split("\0")
    errors: list[str] = []
    for index in range(0, len(parts) - 1, 2):
        commit = parts[index].strip()
        message = parts[index + 1]
        if not commit:
            continue
        if FORBIDDEN_DASH in message:
            errors.append(f"commit {commit}: forbidden U+2014 character")
        if COAUTHOR in message.casefold():
            errors.append(f"commit {commit}: forbidden co-author trailer")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--commits", nargs=2, metavar=("BASE", "HEAD"))
    args = parser.parse_args()

    errors = check_files()
    if args.commits:
        errors.extend(check_commits(*args.commits))
    if errors:
        print("Repository style check failed:", file=sys.stderr)
        for error in errors:
            print(f"  {error}", file=sys.stderr)
        return 1
    print("Repository style check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
