"""Package an App source directory without compiling LaRuche."""
import argparse
import json
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("source", type=Path, help="Directory containing app.json and ui/")
parser.add_argument("--output", type=Path, default=Path("dist"))
args = parser.parse_args()
source = args.source.resolve(strict=True)
manifest = json.loads((source / "app.json").read_text(encoding="utf-8"))
import re
if not re.fullmatch(r"[a-z0-9]+(?:[.-][a-z0-9]+)+", manifest["id"]):
    parser.error("Invalid App id")
if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[A-Za-z0-9.-]+)?", manifest["version"]):
    parser.error("Invalid App version")
output_dir = args.output.resolve()
if output_dir == source or source in output_dir.parents:
    parser.error("Output directory must be outside the source package")
files = sorted(path for path in source.rglob("*") if path.is_file())
for path in files:
    if path.is_symlink() or not path.resolve().is_relative_to(source):
        parser.error(f"Linked files are not supported: {path.name}")
output_dir.mkdir(parents=True, exist_ok=True)
output = output_dir / f"{manifest['id']}-{manifest['version']}.laruche-app"
with ZipFile(output, "x", compression=ZIP_DEFLATED, compresslevel=9) as archive:
    for path in files:
        archive.write(path, path.relative_to(source).as_posix())
print(output)
