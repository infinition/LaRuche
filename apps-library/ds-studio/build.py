"""Package DS Studio into an installable .laruche-app archive.

The host refuses a compressed package over 32 MiB, so the archive size is
checked here rather than discovered at install time.
"""

from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile
import json
import sys

ROOT = Path(__file__).resolve().parent
SOURCE = ROOT / "package"
DIST = ROOT / "dist"

MAX_ARCHIVE_BYTES = 32 * 1024 * 1024
MAX_FILES = 4096


def human(size: int) -> str:
    for unit in ("B", "KiB", "MiB"):
        if size < 1024 or unit == "MiB":
            return f"{size:.1f} {unit}" if unit != "B" else f"{size} {unit}"
        size /= 1024
    return f"{size:.1f} MiB"


def main() -> int:
    manifest = json.loads((SOURCE / "app.json").read_text(encoding="utf-8"))
    DIST.mkdir(exist_ok=True)
    output = DIST / f"{manifest['id']}-{manifest['version']}.laruche-app"

    files = [path for path in sorted(SOURCE.rglob("*")) if path.is_file()]
    if len(files) > MAX_FILES:
        print(f"error: {len(files)} files, the host accepts at most {MAX_FILES}", file=sys.stderr)
        return 1

    with ZipFile(output, "w", compression=ZIP_DEFLATED, compresslevel=9) as archive:
        for path in files:
            archive.write(path, path.relative_to(SOURCE).as_posix())

    size = output.stat().st_size
    vendored = (SOURCE / "ui" / "vendor" / "pyodide" / "studio-manifest.json").exists()

    print(output)
    print(f"  files:  {len(files)}")
    print(f"  size:   {human(size)} / {human(MAX_ARCHIVE_BYTES)}")
    print(f"  kernel: {'Python (vendored) with built-in fallback' if vendored else 'built-in only'}")

    if size > MAX_ARCHIVE_BYTES:
        print(
            "\nerror: the archive exceeds the 32 MiB the installer accepts.\n"
            "       Remove packages from ui/vendor/pyodide/ and rerun tools/vendor_pyodide.py\n"
            "       with a smaller --packages list.",
            file=sys.stderr,
        )
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
