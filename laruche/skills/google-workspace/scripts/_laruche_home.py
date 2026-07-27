"""Resolve LARUCHE_HOME for standalone skill scripts.

Skill scripts may run outside the third-party process (e.g. system Python,
nix env, CI) where ``third-party_constants`` is not importable.  This module
provides the same ``get_laruche_home()`` and ``display_laruche_home()``
contracts as ``third-party_constants`` without requiring it on ``sys.path``.

When ``third-party_constants`` IS available it is used directly so that any
future enhancements (profile resolution, Docker detection, etc.) are
picked up automatically.  The fallback path replicates the core logic
from ``third-party_constants.py`` using only the stdlib.

All scripts under ``google-workspace/scripts/`` should import from here
instead of duplicating the ``LARUCHE_HOME = Path(os.getenv(...))`` pattern.
"""

from __future__ import annotations

import os
from pathlib import Path

try:
    from third-party_constants import display_laruche_home as display_laruche_home
    from third-party_constants import get_laruche_home as get_laruche_home
except (ModuleNotFoundError, ImportError):

    def get_laruche_home() -> Path:
        """Return the third-party home directory (default: ~/.laruche).

        Mirrors ``third-party_constants.get_laruche_home()``."""
        val = os.environ.get("LARUCHE_HOME", "").strip()
        return Path(val) if val else Path.home() / ".laruche"

    def display_laruche_home() -> str:
        """Return a user-friendly ``~/``-shortened display string.

        Mirrors ``third-party_constants.display_laruche_home()``."""
        home = get_laruche_home()
        try:
            return "~/" + str(home.relative_to(Path.home()))
        except ValueError:
            return str(home)
