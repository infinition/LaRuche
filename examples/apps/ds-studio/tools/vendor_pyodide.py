"""Vendor the Pyodide runtime into the DS Studio package.

Why this script exists. The App sandbox serves a Content-Security-Policy whose
script-src and connect-src allow only the App's own versioned asset directory,
so the notebook cannot pull Pyodide from a CDN at runtime the way the Obsidian
plugin does. The runtime has to travel inside the .laruche-app archive, and
that archive may not exceed 32 MiB compressed.

That budget is the whole difficulty. The core runtime is roughly 10 MiB
compressed; numpy and pandas fit alongside it; matplotlib usually does not.
The script measures what it downloads and refuses to leave you with a package
the installer would reject.

Usage, from the repository root:

    python examples/apps/ds-studio/tools/vendor_pyodide.py
    python examples/apps/ds-studio/tools/vendor_pyodide.py --packages numpy pandas
    python examples/apps/ds-studio/tools/vendor_pyodide.py --clean

Then rebuild:

    python examples/apps/ds-studio/build.py

Without this step the App still works: it starts its built-in kernel instead,
and reports which one is running.
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGET = ROOT / "package" / "ui" / "vendor" / "pyodide"

DEFAULT_VERSION = "0.26.4"
DEFAULT_BASE = "https://cdn.jsdelivr.net/pyodide/v{version}/full/"
DEFAULT_PACKAGES = ["numpy", "pandas"]

# The loader, the interpreter and the standard library. Without all of these
# loadPyodide cannot start.
CORE_FILES = [
    "pyodide.js",
    "pyodide.asm.js",
    "pyodide.asm.wasm",
    "pyodide-lock.json",
    "python_stdlib.zip",
]

ARCHIVE_BUDGET = 32 * 1024 * 1024
# Everything else in the package is well under a megabyte; leave room for it.
RUNTIME_BUDGET = 30 * 1024 * 1024


def human(size: float) -> str:
    for unit in ("B", "KiB", "MiB"):
        if size < 1024 or unit == "MiB":
            return f"{size:.1f} {unit}"
        size /= 1024
    return f"{size:.1f} MiB"


def fetch(url: str, destination: Path) -> int:
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        with urllib.request.urlopen(url, timeout=120) as response:
            payload = response.read()
    except urllib.error.HTTPError as error:
        raise SystemExit(f"error: {url} returned HTTP {error.code}")
    except urllib.error.URLError as error:
        raise SystemExit(f"error: cannot reach {url} ({error.reason})")
    destination.write_bytes(payload)
    return len(payload)


def resolve_package_files(lock: dict, wanted: list[str]) -> list[str]:
    """Walk pyodide-lock.json to collect each package and its dependencies."""
    packages = lock.get("packages", {})
    by_name = {name.lower(): entry for name, entry in packages.items()}

    seen: set[str] = set()
    ordered: list[str] = []

    def visit(name: str) -> None:
        key = name.lower()
        if key in seen:
            return
        seen.add(key)
        entry = by_name.get(key)
        if entry is None:
            raise SystemExit(
                f"error: package '{name}' is not in this Pyodide distribution.\n"
                f"       Available names are listed in pyodide-lock.json."
            )
        for dependency in entry.get("depends", []):
            visit(dependency)
        ordered.append(entry["file_name"])

    for name in wanted:
        visit(name)
    return ordered


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--version", default=DEFAULT_VERSION, help="Pyodide version (default %(default)s)")
    parser.add_argument("--base", default=None, help="Distribution base URL, or a local directory")
    parser.add_argument(
        "--packages",
        nargs="*",
        default=DEFAULT_PACKAGES,
        help="Python packages to bundle (default: %(default)s). matplotlib rarely fits the 32 MiB budget.",
    )
    parser.add_argument("--clean", action="store_true", help="Remove the vendored runtime and exit")
    parser.add_argument("--force", action="store_true", help="Write the files even if the budget is exceeded")
    arguments = parser.parse_args()

    if arguments.clean:
        if TARGET.exists():
            shutil.rmtree(TARGET)
            print(f"removed {TARGET}")
            print("The App will start its built-in kernel.")
        else:
            print("nothing to remove")
        return 0

    base = arguments.base or DEFAULT_BASE.format(version=arguments.version)
    local = None
    if not base.startswith("http"):
        local = Path(base)
        if not local.is_dir():
            raise SystemExit(f"error: {local} is not a directory")
    elif not base.endswith("/"):
        base += "/"

    TARGET.mkdir(parents=True, exist_ok=True)
    total = 0
    written: list[str] = []

    print(f"Pyodide {arguments.version}")
    print(f"source: {base}")
    print(f"target: {TARGET}\n")

    def grab(name: str) -> int:
        if local is not None:
            source = local / name
            if not source.is_file():
                raise SystemExit(f"error: {source} not found")
            payload = source.read_bytes()
            (TARGET / name).write_bytes(payload)
            return len(payload)
        return fetch(base + name, TARGET / name)

    for name in CORE_FILES:
        size = grab(name)
        total += size
        written.append(name)
        print(f"  {name:<24} {human(size):>10}")

    lock = json.loads((TARGET / "pyodide-lock.json").read_text(encoding="utf-8"))

    package_files: list[str] = []
    if arguments.packages:
        package_files = resolve_package_files(lock, arguments.packages)
        print()
        for name in package_files:
            size = grab(name)
            total += size
            written.append(name)
            print(f"  {name:<24} {human(size):>10}")

    print(f"\n  {'total':<24} {human(total):>10}   budget {human(RUNTIME_BUDGET)}")

    over_budget = total > RUNTIME_BUDGET
    if over_budget:
        print(
            f"\nwarning: the runtime alone is {human(total)}, and the installer refuses a\n"
            f"         package over {human(ARCHIVE_BUDGET)} compressed. Compression will claw some of\n"
            f"         that back, but do run build.py to see the real archive size.\n"
            f"         Dropping a package is usually the fix: matplotlib is the largest,\n"
            f"         and DS Studio renders its charts from the data anyway.",
            file=sys.stderr,
        )
        if not arguments.force:
            print("\nRerun with --force to keep these files, or with a shorter --packages list.", file=sys.stderr)
            for name in written:
                (TARGET / name).unlink(missing_ok=True)
            manifest = TARGET / "studio-manifest.json"
            manifest.unlink(missing_ok=True)
            return 1

    # The App probes for this manifest to decide whether the Python kernel is
    # available at all. It is written last, so a half-finished download never
    # looks like a working runtime.
    (TARGET / "studio-manifest.json").write_text(
        json.dumps(
            {
                "pyodideVersion": arguments.version,
                "packages": list(arguments.packages),
                "files": written,
                "bytes": total,
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )

    print("\nVendored. The App will start the Python kernel on next load.")
    print("Next: python examples/apps/ds-studio/build.py")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
