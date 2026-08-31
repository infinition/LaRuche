#!/usr/bin/env python3
"""Embed the wiki/ folder into docs/wiki.html.

Why a build step rather than fetching at runtime: GitHub Pages only publishes the
folder it serves from (docs/), so wiki/*.md would not even be reachable, and the
GitHub contents API is rate limited to 60 requests per hour per IP without a token,
which would leave the wiki blank for visitors once the quota is spent. The whole
corpus is ~45 KB, so it simply travels inside the page. No request, no quota, and
it works offline like the product it documents.

Usage, from the repository root:

    python scripts/build_wiki.py

It rewrites only the block delimited by WIKI-DATA markers inside docs/wiki.html,
so the theme and the renderer stay editable as normal HTML.
"""

import io
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WIKI = os.path.join(ROOT, "wiki")
TARGET = os.path.join(ROOT, "docs", "wiki.html")

BEGIN = "/* WIKI-DATA:BEGIN */"
END = "/* WIKI-DATA:END */"

# Folder order in the sidebar. Anything not listed lands at the end, alphabetically.
SECTION_ORDER = ["", "getting-started", "concepts", "guides", "reference"]
SECTION_LABELS = {
    "": "Overview",
    "getting-started": "Getting started",
    "concepts": "Concepts",
    "guides": "Guides",
    "reference": "Reference",
}
# Page order inside a folder. Unlisted pages follow, alphabetically.
PAGE_ORDER = {
    "": ["Home", "FAQ", "Security"],
    "getting-started": ["Installation", "Desktop-App", "Quick-Start", "Local-Models"],
    "concepts": [
        "Architecture",
        "Butinage-Engine",
        "Cognitive-Memory",
        "LaReine",
        "Table-Ronde",
        "Watchers",
        "Automation",
        "Skills-and-Curator",
    ],
    "guides": [
        "Computer-and-Browser",
        "Chrome-Extension",
        "Training-Datasets",
        "Voice",
        "Telegram",
        "MCP",
        "Secrets",
        "Troubleshooting",
    ],
    "reference": [
        "Configuration",
        "Providers-and-Profiles",
        "Tools",
        "Evals",
        "Brand-Glossary",
    ],
}


def titre(markdown, slug):
    """First level-1 heading, falling back to the slug."""
    m = re.search(r"(?m)^#\s+(.+?)\s*$", markdown)
    return m.group(1).strip() if m else slug.replace("-", " ")


def rang(liste, valeur):
    return liste.index(valeur) if valeur in liste else len(liste)


def collecter():
    pages, sections = {}, {}
    for base, _dirs, fichiers in os.walk(WIKI):
        for nom in fichiers:
            if not nom.endswith(".md"):
                continue
            chemin = os.path.join(base, nom)
            dossier = os.path.relpath(base, WIKI).replace("\\", "/")
            dossier = "" if dossier == "." else dossier
            slug = nom[:-3]
            with io.open(chemin, encoding="utf-8") as fh:
                md = fh.read()
            if slug in pages:
                print("  ! duplicate page name: %s (names must be unique)" % slug)
            pages[slug] = md
            sections.setdefault(dossier, []).append(slug)

    ordre = sorted(sections, key=lambda d: (rang(SECTION_ORDER, d), d))
    sortie = []
    for dossier in ordre:
        prefere = PAGE_ORDER.get(dossier, [])
        slugs = sorted(sections[dossier], key=lambda s: (rang(prefere, s), s))
        sortie.append(
            {
                "label": SECTION_LABELS.get(dossier, dossier.replace("-", " ").title()),
                "pages": [{"slug": s, "title": titre(pages[s], s)} for s in slugs],
            }
        )
    return sortie, pages


def main():
    if not os.path.isdir(WIKI):
        sys.exit("wiki/ not found next to scripts/, run this from the repository root")
    if not os.path.isfile(TARGET):
        sys.exit("docs/wiki.html not found")

    sections, pages = collecter()
    connus = {p["slug"] for s in sections for p in s["pages"]}

    # Report internal links that point nowhere, they would render as dead entries.
    casses = set()
    for slug, md in pages.items():
        for cible in re.findall(r"\[[^\]]*\]\(([^)]+)\)", md):
            if cible.startswith(("http", "#", "mailto:", "/")):
                continue
            page = cible.split("#", 1)[0]
            if page and page not in connus:
                casses.add("%s -> %s" % (slug, cible))

    data = json.dumps(
        {"sections": sections, "pages": pages}, ensure_ascii=False, separators=(",", ":")
    )

    with io.open(TARGET, encoding="utf-8", newline="") as fh:
        html = fh.read()
    if BEGIN not in html or END not in html:
        sys.exit("markers %s / %s missing from docs/wiki.html" % (BEGIN, END))

    debut = html.index(BEGIN) + len(BEGIN)
    fin = html.index(END)
    html = html[:debut] + "\nvar WIKI = " + data + ";\n" + html[fin:]

    with io.open(TARGET, "w", encoding="utf-8", newline="") as fh:
        fh.write(html)

    total = sum(len(m) for m in pages.values())
    print("%d pages, %d sections, %.1f KB of markdown embedded" % (len(pages), len(sections), total / 1024.0))
    for s in sections:
        print("  %-16s %s" % (s["label"], ", ".join(p["slug"] for p in s["pages"])))
    if casses:
        print("\n  ! internal links pointing at no page:")
        for c in sorted(casses):
            print("    %s" % c)
    else:
        print("\n  every internal link resolves")


if __name__ == "__main__":
    main()
