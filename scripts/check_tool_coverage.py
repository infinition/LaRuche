#!/usr/bin/env python3
"""Fail if a registered tool is neither documented in a skill nor declared skill-less.

Reads every `fn nom(&self) -> &str { "..." }` in the tool modules, cross-references the
skill bodies, and checks the remainder against the table in skills/TOOL-COVERAGE.md.

Usage: python scripts/check_tool_coverage.py   (runs from anywhere)
Exit code 0 when every tool is accounted for, 1 otherwise.
"""

import io
import os
import re
import sys

# scripts/ sits at the repository root, next to laruche/. Paths are derived from
# __file__ rather than the working directory so the check runs from anywhere.
ROOT = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "laruche")
SOURCES = [
    os.path.join(ROOT, "laruche-essaim", "src", "abeilles"),
    os.path.join(ROOT, "laruche-node", "src"),
]
SKILLS = os.path.join(ROOT, "skills")
MANIFEST = os.path.join(SKILLS, "TOOL-COVERAGE.md")

TOOL_NAME = re.compile(r'fn nom\(&self\) -> &str \{\s*"([a-z][a-z0-9_]+)"')


def read(path):
    with io.open(path, encoding="utf-8", errors="replace") as f:
        return f.read()


def registered_tools():
    found = set()
    for base in SOURCES:
        if not os.path.isdir(base):
            continue
        for folder, _, files in os.walk(base):
            for name in files:
                if name.endswith(".rs"):
                    found.update(TOOL_NAME.findall(read(os.path.join(folder, name))))
    return sorted(found)


def skill_bodies():
    bodies = {}
    if not os.path.isdir(SKILLS):
        return bodies
    for entry in sorted(os.listdir(SKILLS)):
        path = os.path.join(SKILLS, entry, "SKILL.md")
        if os.path.isfile(path):
            bodies[entry] = read(path)
    return bodies


def declared_skill_less():
    """Tool names listed in the `Deliberately without a skill` table of the manifest."""
    if not os.path.isfile(MANIFEST):
        return set()
    text = read(MANIFEST)
    start = text.find("## Deliberately without a skill")
    if start == -1:
        return set()
    end = text.find("\n## ", start + 1)
    section = text[start : end if end != -1 else len(text)]
    return set(re.findall(r"^\|\s*`([a-z][a-z0-9_]+)`\s*\|", section, re.M))


def main():
    tools = registered_tools()
    if not tools:
        print("No tool found. Run this script from the laruche/ directory.")
        return 1

    skills = skill_bodies()
    declared = declared_skill_less()

    documented, orphans = {}, []
    for tool in tools:
        pattern = re.compile(r"\b" + re.escape(tool) + r"\b")
        found_in = [name for name, body in skills.items() if pattern.search(body)]
        if found_in:
            documented[tool] = found_in
        elif tool not in declared:
            orphans.append(tool)

    print("registered tools    : %d" % len(tools))
    print("documented in skill : %d" % len(documented))
    print("declared skill-less : %d" % len(declared & set(tools)))
    print("skills              : %d" % len(skills))

    stale = sorted(declared - set(tools))
    if stale:
        print("\nListed in TOOL-COVERAGE.md but no longer registered:")
        for tool in stale:
            print("  ", tool)

    if orphans:
        print("\nFAIL: %d tool(s) with neither a skill nor a declaration:" % len(orphans))
        for tool in orphans:
            print("  ", tool)
        print("\nAdd them to a skill, or to the 'Deliberately without a skill' table")
        print("in skills/TOOL-COVERAGE.md with a reason.")
        return 1

    print("\nOK: every tool is covered or declared.")
    return 0


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.exit(main())
