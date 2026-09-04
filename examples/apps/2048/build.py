from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile
import json

ROOT = Path(__file__).resolve().parent
SOURCE = ROOT / "package"
DIST = ROOT / "dist"

manifest = json.loads((SOURCE / "app.json").read_text(encoding="utf-8"))
DIST.mkdir(exist_ok=True)
output = DIST / f"laruche-2048-{manifest['version']}.laruche-app"

with ZipFile(output, "w", compression=ZIP_DEFLATED, compresslevel=9) as archive:
    for path in sorted(SOURCE.rglob("*")):
        if path.is_file():
            archive.write(path, path.relative_to(SOURCE).as_posix())

print(output)
