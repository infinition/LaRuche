"""Resolve THIRD_PARTY_HOME for standalone skill scripts.

Skill scripts may run outside the third-party process (e.g. system Python,
nix env, CI) where ``third-party_constants`` is not importable.  This module
provides the same ``get_THIRD_PARTY_HOME()`` and ``display_THIRD_PARTY_HOME()``
contracts as ``third-party_constants`` without requiring it on ``sys.path``.

When ``third-party_constants`` IS available it is used directly so that any
future enhancements (profile resolution, Docker detection, etc.) are
picked up automatically.  The fallback path replicates the core logic
from ``third-party_constants.py`` using only the stdlib.

All scripts under ``google-workspace/scripts/`` should import from here
instead of duplicating the ``THIRD_PARTY_HOME = Path(os.getenv(...))`` pattern.
"""

from __future__ import annotations

import os
from pathlib import Path

try:
    from third-party_constants import display_THIRD_PARTY_HOME as display_THIRD_PARTY_HOME
    from third-party_constants import get_THIRD_PARTY_HOME as get_THIRD_PARTY_HOME
except (ModuleNotFoundError, ImportError):

    def get_THIRD_PARTY_HOME() -> Path:
        """Return the third-party home directory (default: ~/.third-party).

        Mirrors ``third-party_constants.get_THIRD_PARTY_HOME()``."""
        val = os.environ.get("THIRD_PARTY_HOME", "").strip()
        return Path(val) if val else Path.home() / ".third-party"

    def display_THIRD_PARTY_HOME() -> str:
        """Return a user-friendly ``~/``-shortened display string.

        Mirrors ``third-party_constants.display_THIRD_PARTY_HOME()``."""
        home = get_THIRD_PARTY_HOME()
        try:
            return "~/" + str(home.relative_to(Path.home()))
        except ValueError:
            return str(home)
