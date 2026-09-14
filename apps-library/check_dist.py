"""Refuse a built archive that no longer matches its source.

An archive is a snapshot. Edit a file under `package/` without rebuilding, and
`dist/` still holds the previous one, with the same name and a plausible date.
That is how a correction gets handed over without being in what was installed.

Run from the repository root:

    python apps-library/check_dist.py

Exit code 0 when every archive matches the source beside it and carries its
manifest version, 1 otherwise. An App with no archive at all is reported too:
its `dist/` is simply not built yet.
"""

import hashlib
import json
import sys
import zipfile
from pathlib import Path

RACINE = Path(__file__).resolve().parent


def empreinte(octets: bytes) -> bytes:
    return hashlib.sha256(octets).digest()


def verifier(archive: Path, source: Path) -> tuple[list[str], list[str]]:
    """Returns what is wrong, and what is merely generated.

    A file present in the archive and absent from the source is not a fault:
    WASM Lab writes its compiled module straight into the archive, and nothing
    on disk corresponds to it. Reported, never counted.
    """
    ecarts, generes = [], []
    with zipfile.ZipFile(archive) as zip_archive:
        noms = set(zip_archive.namelist())
        manifeste = json.loads(zip_archive.read("app.json"))
        if manifeste["version"] not in archive.name:
            ecarts.append(
                "the name does not carry version %s" % manifeste["version"]
            )
        attendus = set()
        for fichier in sorted(p for p in source.rglob("*") if p.is_file()):
            relatif = fichier.relative_to(source).as_posix()
            attendus.add(relatif)
            if relatif not in noms:
                ecarts.append("missing from the archive: " + relatif)
            elif empreinte(zip_archive.read(relatif)) != empreinte(fichier.read_bytes()):
                ecarts.append("differs from the source: " + relatif)
        for surplus in sorted(noms - attendus):
            generes.append(surplus)
    return ecarts, generes


def main() -> int:
    apps = sorted(p.parent.parent for p in RACINE.glob("*/package/app.json"))
    if not apps:
        print("no App found next to this script", file=sys.stderr)
        return 1

    problemes = 0
    for app in apps:
        version = json.loads((app / "package" / "app.json").read_text(encoding="utf-8"))["version"]
        archives = sorted((app / "dist").glob("*.laruche-app")) if (app / "dist").is_dir() else []
        if not archives:
            print("%-12s v%-8s not built" % (app.name, version))
            continue
        for archive in archives:
            ecarts, generes = verifier(archive, app / "package")
            etat = "ok" if not ecarts else "STALE"
            print("%-12s v%-8s %-6s %s" % (app.name, version, etat, archive.name))
            for ecart in ecarts:
                print("               " + ecart)
            for genere in generes:
                print("               built into the archive: " + genere)
            problemes += len(ecarts)

    print()
    if problemes:
        print("%d difference(s). Rebuild with the App's build.py." % problemes)
        return 1
    print("every archive matches the source beside it")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
