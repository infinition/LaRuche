#!/usr/bin/env python3
"""Importe les skills third-party -> LaRuche (skills/<slug>/SKILL.md).

- Filtre par plateforme (skip macOS/iOS-only sur Windows/Linux).
- Remap les noms de tools third-party -> LaRuche (sur les invocations `tool(` uniquement, pour ne pas
  corrompre la prose).
- Injecte `type: skill` dans le frontmatter (requis par LaRuche).
- Copie les dossiers scripts/ (et references/, templates/, assets/).
Au prochain boot, sync_skills_disk_to_sql les indexe dans capacities.skills.*.
"""
import os, re, shutil, sys

third-party = r"C:\Users\infinition\Desktop\third-party agent\skills"
DEST = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "skills")

# Remap des INVOCATIONS d'outils `name(` -> `autre(` (sûr : suivi de parenthèse).
TOOL_REMAP = {
    "terminal": "shell_exec",
    "write_file": "file_write",
    "read_file": "file_read",
    "patch": "file_edit",
    "search_files": "file_search",
    "web_extract": "web_fetch",
    "vision_analyze": "image_search",
}
# Inchangés (existent tels quels) : web_search, session_search, skill_view, todo.

def slugify(name):
    s = re.sub(r"[^a-z0-9]+", "-", (name or "").strip().lower()).strip("-")
    return s or "skill"

def frontmatter(content):
    m = re.match(r"^---\s*\n(.*?)\n---\s*\n", content, re.S)
    return m.group(1) if m else ""

def platform_ok(fm):
    m = re.search(r"^platforms:\s*\[([^\]]*)\]", fm, re.M)
    if not m:
        return True  # pas de contrainte -> portable
    plats = [p.strip().lower() for p in m.group(1).split(",") if p.strip()]
    if not plats:
        return True
    # garde si linux ou windows présent ; skip si macos/ios-only
    return ("linux" in plats) or ("windows" in plats)

def remap_tools(body):
    for h, l in TOOL_REMAP.items():
        body = re.sub(r"\b" + re.escape(h) + r"\(", l + "(", body)
    return body

def ensure_type_skill(content):
    if re.search(r"^type:\s*skill", content, re.M):
        return content
    # insère 'type: skill' juste après le premier '---'
    return re.sub(r"^---\s*\n", "---\ntype: skill\n", content, count=1)

def main():
    if not os.path.isdir(third-party):
        print("third-party skills introuvable:", third-party); return 1
    os.makedirs(DEST, exist_ok=True)
    imported, skipped_plat, no_md = 0, 0, 0
    for root, _, files in os.walk(third-party):
        if "SKILL.md" not in files:
            continue
        path = os.path.join(root, "SKILL.md")
        try:
            content = open(path, encoding="utf-8").read()
        except Exception:
            no_md += 1; continue
        fm = frontmatter(content)
        if not platform_ok(fm):
            skipped_plat += 1; continue
        nm = re.search(r"^name:\s*['\"]?([^'\"\n]+)", fm, re.M)
        slug = slugify(nm.group(1) if nm else os.path.basename(root))
        # remap tools dans le corps (après le frontmatter)
        parts = content.split("\n---\n", 1)
        if len(parts) == 2:
            content = parts[0] + "\n---\n" + remap_tools(parts[1])
        content = ensure_type_skill(content)
        dest_dir = os.path.join(DEST, slug)
        os.makedirs(dest_dir, exist_ok=True)
        open(os.path.join(dest_dir, "SKILL.md"), "w", encoding="utf-8").write(content)
        # copie scripts/references/templates/assets
        for sub in ("scripts", "references", "templates", "assets"):
            src = os.path.join(root, sub)
            if os.path.isdir(src):
                shutil.copytree(src, os.path.join(dest_dir, sub), dirs_exist_ok=True)
        imported += 1
    print(f"Importes: {imported} | skip plateforme(macOS/iOS): {skipped_plat} | erreurs: {no_md}")
    return 0

if __name__ == "__main__":
    sys.exit(main())
