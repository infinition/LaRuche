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
# Second output: the same corpus as a skill, so the agent can answer questions about
# LaRuche from the documentation instead of from memory. Shipped skills are baked into
# the binary (`include_dir!` on laruche/skills), so this folder travels in the exe.
SKILL = os.path.join(ROOT, "laruche", "skills", "laruche")

BEGIN = "/* WIKI-DATA:BEGIN */"
END = "/* WIKI-DATA:END */"

# The skill's front matter and prose. Only the routing table is generated, so the
# wording stays reviewable here rather than buried in string concatenation.
#
# The description is 76 characters. The skill index injects `name - description` for
# EVERY skill into EVERY turn and truncates at 80, so anything longer is both paid for
# and lost. Keep it under that if you edit it.
MODELE_SKILL = """---
type: skill
name: laruche
description: What LaRuche is: architecture, butinage, memory, LaReine, and its full wiki.
tools: [file_read]
---

# LaRuche, expliquee par elle-meme

Ce skill embarque le wiki complet de LaRuche, %(n)d pages, %(ko)d Ko de markdown. C'est
la MEME source que le site publie: `wiki/` a la racine du depot, dont `docs/wiki.html`
et ce dossier sont deux sorties generees par `scripts/build_wiki.py`. Ce qui est ecrit
ici fait donc foi. Ne repondez jamais de memoire sur le fonctionnement de LaRuche:
ouvrez la page et citez-la.

## Quand se servir de ce skill

Toute question sur LaRuche elle-meme: ce qu'elle sait faire, comment elle est faite, ce
qu'un mot du vocabulaire designe, pourquoi elle se comporte d'une certaine facon, ce
qu'elle fait de vos donnees. Y compris la demande d'accueil "Presente-toi".

Ce skill dit ce que LaRuche EST. Pour FAIRE quelque chose, les skills voisins sont plus
directs: `configure-laruche` (reglages, fournisseur, canaux, secrets),
`cognitive-memory` (retenir et retrouver), `extend-toolset` (plugin, MCP),
`delegation` (sous-agent), `long-running-work` (mission, kanban, plan).

## En une phrase

LaRuche est un agent local: le butinage est sa boucle de raisonnement et d'action, les
abeilles sont ses outils, les eclaireuses ses sous-agents, LaReine sa supervision, et sa
memoire est une carte de faits au format OKF, lisible et modifiable a la main.

## La carte des pages

Ouvrez la page qui repond, avec `file_read`, sur le chemin donne relatif au dossier de
ce skill (`skills/laruche/`). Les corps sont integraux: ils ne coutent rien tant que
vous ne les demandez pas.

| Page | Titre | Ce qu'elle couvre |
|---|---|---|
%(table)s

## Deux reflexes

Citez la page d'ou vient votre reponse, l'utilisateur peut la relire.
Une page absente de ce tableau n'existe pas: dites-le plutot que de l'inventer.
"""

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
    pages, sections, dossiers = {}, {}, {}
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
            dossiers[slug] = dossier
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
    return sortie, pages, dossiers



def resume(markdown):
    """First SENTENCE of prose under the H1: what the page is about, in the author's words.

    The paragraph is rejoined before cutting. The wiki sources are hard-wrapped at about
    ninety columns, so reading a single physical line handed back half a sentence: the
    routing table advertised Home as "a local-first AI agent with a desktop application,
    a" and stopped there, which guides nobody.
    """
    para, vu = [], False
    for ligne in markdown.splitlines():
        if ligne.startswith("# "):
            vu = True
            continue
        t = ligne.strip()
        if not vu:
            continue
        if not t or t.startswith(("#", ">", "|", "-", "*", "`")):
            # Stop at the end of the paragraph, unless what we hold is too short to
            # mean anything: the FAQ opens on a heading followed by the single word
            # "No.", which as a table row guides nobody. Keep reading in that case.
            if para and len(" ".join(para)) >= 45:
                break
            continue           # not started yet, or not informative yet
        para.append(t)
    if not para:
        return ""
    texte = " ".join(para)
    if len(texte) <= 130:
        return texte
    # Whole sentences, never a cut mid-clause, and enough of them to actually guide.
    # One sentence is the right unit until the page opens on a short one: Home began
    # with "Welcome to the hive." and the FAQ with "No.", which say nothing about what
    # the page holds. Keep taking sentences until the line earns its row.
    fini, reste = "", texte
    while reste:
        point = reste.find(". ")
        if point < 0:
            phrase, reste = reste, ""
        else:
            phrase, reste = reste[: point + 1], reste[point + 2 :]
        if fini and len(fini) + 1 + len(phrase) > 130:
            break
        fini = (fini + " " + phrase).strip() if fini else phrase
        if len(fini) >= 45:
            break
    # A single short sentence followed by one too long to append leaves us holding
    # "No." again: below the useful threshold, fall back to a plain cut.
    if len(fini) < 45:
        fini = texte
    return fini if len(fini) <= 130 else fini[:127].rsplit(" ", 1)[0] + "..."


def forger_skill(sections, pages, dossiers):
    """Rewrite laruche/skills/laruche/ from the same corpus that feeds the site.

    Without this the skill was a hand-made copy that nothing kept in step: editing a
    wiki page updated the site and left the agent quoting last month's documentation,
    with no warning that the two had parted ways. One source, one command, two outputs.
    """
    racine_md = os.path.join(SKILL, "wiki")
    for base, _dirs, fichiers in os.walk(racine_md, topdown=False):
        for nom in fichiers:
            os.remove(os.path.join(base, nom))
        if base != racine_md:
            os.rmdir(base)
    os.makedirs(racine_md, exist_ok=True)

    lignes, total = [], 0
    for section in sections:
        lignes.append("| **%s** | | |" % section["label"])
        for page in section["pages"]:
            slug = page["slug"]
            dossier = dossiers[slug]
            rel = ("%s/%s.md" % (dossier, slug)) if dossier else ("%s.md" % slug)
            cible = os.path.join(racine_md, rel.replace("/", os.sep))
            os.makedirs(os.path.dirname(cible), exist_ok=True)
            with io.open(cible, "w", encoding="utf-8", newline="\n") as fh:
                fh.write(pages[slug])
            total += len(pages[slug])
            lignes.append("| `wiki/%s` | %s | %s |"
                          % (rel, page["title"], resume(pages[slug]).replace("|", "/")))

    corps = MODELE_SKILL % {
        "n": sum(len(s["pages"]) for s in sections),
        "ko": int(round(total / 1024.0)),
        "table": "\n".join(lignes),
    }
    with io.open(os.path.join(SKILL, "SKILL.md"), "w", encoding="utf-8", newline="\n") as fh:
        fh.write(corps)
    return total


def main():
    if not os.path.isdir(WIKI):
        sys.exit("wiki/ not found next to scripts/, run this from the repository root")
    if not os.path.isfile(TARGET):
        sys.exit("docs/wiki.html not found")

    sections, pages, dossiers = collecter()
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

    octets = forger_skill(sections, pages, dossiers)

    total = sum(len(m) for m in pages.values())
    print("%d pages, %d sections, %.1f KB of markdown embedded" % (len(pages), len(sections), total / 1024.0))
    print("  skill laruche rebuilt: %d pages, %.1f KB (baked into the binary)" % (len(pages), octets / 1024.0))
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
