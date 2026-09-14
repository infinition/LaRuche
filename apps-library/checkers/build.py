from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile
import json

ROOT = Path(__file__).resolve().parent
SOURCE = ROOT / "package"
DIST = ROOT / "dist"

# Keep the installed games self-contained, with one scheduler source.
(SOURCE / "ui" / "game-agent.js").write_bytes((ROOT.parent / "shared" / "game-agent.js").read_bytes())

manifest = json.loads((SOURCE / "app.json").read_text(encoding="utf-8"))
DIST.mkdir(exist_ok=True)
output = DIST / f"{manifest['id']}-{manifest['version']}.laruche-app"

with ZipFile(output, "w", compression=ZIP_DEFLATED, compresslevel=9) as archive:
    for path in sorted(SOURCE.rglob("*")):
        if path.is_file():
            archive.write(path, path.relative_to(SOURCE).as_posix())

print(output)
