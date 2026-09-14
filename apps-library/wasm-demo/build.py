"""Build a tiny self-contained WASM reference package, without external compilers."""
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile

root = Path(__file__).resolve().parent
source = root / "package"
dist = root / "dist"
dist.mkdir(exist_ok=True)
# (module (func (export "add") (param i32 i32) (result i32)
#   local.get 0 local.get 1 i32.add))
# Standard WASM binary: header, type, function, export, code sections.
module = bytes.fromhex(
    "0061736d01000000 01070160027f7f017f 03020100 "
    "070701036164640000 0a09010700200020016a0b"
)
output = dist / "laruche-wasm-demo-1.0.0.laruche-app"
with ZipFile(output, "w", ZIP_DEFLATED) as archive:
    for path in sorted(source.rglob("*")):
        if path.is_file():
            archive.write(path, path.relative_to(source).as_posix())
    archive.writestr("ui/add.wasm", module)
print(output)
