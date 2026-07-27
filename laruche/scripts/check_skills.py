#!/usr/bin/env python3
"""Lint the skill library against the rules in skills/AUTHORING.md.

Four checks, each of which caught a real defect in the shipped set:

1. FRONTMATTER   `type: skill` present, `name` equal to the folder name, and no
                 field the runtime does not read.
2. DESCRIPTION   survives `resumer_description` untouched. That function (in
                 laruche-essaim/src/contexte.rs) drops everything after the first
                 " - ", then everything after the first ". ", then cuts at 80
                 characters. A description longer than that reaches the model
                 amputated, and nothing reports it.
3. TOOL NAMES    every `tool_name(` call and every backticked `tool_name` that
                 looks like a registered tool actually exists in the registry.
                 `read_extract(urls=[...])` shipped for months against a tool
                 whose only argument is `path`.
4. CROSS-REFS    a skill named in the prose exists on disk, and a bundled file
                 referenced by a relative path exists in the skill folder.

Usage: python scripts/check_skills.py   (run from the laruche/ directory)
Exit code 0 when the library is clean, 1 otherwise.
"""

import io
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SKILLS = os.path.join(ROOT, "skills")
SOURCES = [
    os.path.join(ROOT, "laruche-essaim", "src", "abeilles"),
    os.path.join(ROOT, "laruche-node", "src"),
]

TOOL_NAME = re.compile(r'fn nom\(&self\) -> &str \{\s*"([a-z][a-z0-9_]+)"')
SCHEMA_FN = re.compile(r"fn schema\(&self\)[^{]*\{(.*?)\n    \}", re.S)
READ_FIELDS = {"type", "name", "description", "prerequisites", "enabled", "tools", "scripts"}
DESCRIPTION_BUDGET = 80

# Arguments a tool reads out of `args` without declaring them in its schema. Documented
# here so the linter does not flag a skill for describing something that really works.
UNDECLARED_ARGS = {
    "cron_create": {"skills", "channel"},
}


def read(path):
    with io.open(path, encoding="utf-8", errors="replace") as handle:
        return handle.read()


def tool_schemas():
    """name -> set of accepted argument names, sliced per `fn nom` block."""
    schemas = {}
    for base in SOURCES:
        if not os.path.isdir(base):
            continue
        for folder, _, files in os.walk(base):
            for filename in sorted(files):
                if not filename.endswith(".rs"):
                    continue
                text = read(os.path.join(folder, filename))
                marks = [(m.start(), m.group(1)) for m in TOOL_NAME.finditer(text)]
                for index, (start, name) in enumerate(marks):
                    stop = marks[index + 1][0] if index + 1 < len(marks) else len(text)
                    found = SCHEMA_FN.search(text[start:stop])
                    if not found:
                        continue
                    schemas[name] = _properties(found.group(1)) | UNDECLARED_ARGS.get(name, set())
    return schemas


def _properties(schema_source):
    opening = re.search(r'"properties"\s*:\s*\{(.*)', schema_source, re.S)
    if not opening:
        return set(re.findall(r'"([a-z_][a-z0-9_]*)"\s*:\s*\{\s*"type"', schema_source))
    depth, block = 0, []
    for char in opening.group(1):
        if char == "{":
            depth += 1
        elif char == "}":
            if depth == 0:
                break
            depth -= 1
        block.append(char)
    return set(re.findall(r'"([a-z_][a-z0-9_]*)"\s*:\s*\{', "".join(block)))


def resumer_description(desc):
    """Port of resumer_description in laruche-essaim/src/contexte.rs."""
    collapsed = " ".join(desc.split())
    cut_at = collapsed.find(" - ")
    if cut_at >= 0:
        base = collapsed[:cut_at]
    else:
        cut_at = collapsed.find(". ")
        base = collapsed[:cut_at] if cut_at >= 0 else collapsed
    base = base.strip()
    if len(base) <= DESCRIPTION_BUDGET:
        return base
    kept = ""
    for word in base.split(" "):
        if len(kept) + len(word) + 1 > DESCRIPTION_BUDGET - 2:
            break
        kept = (kept + " " + word).strip()
    return kept + "…"


def frontmatter(text):
    match = re.match(r"---\n(.*?)\n---\n", text, re.S)
    return match.group(1) if match else ""


def field(block, key):
    """Read a scalar or a folded (>-) block field out of the frontmatter."""
    folded = re.search(r"^%s: >-\n((?:  .*\n)+)" % key, block + "\n", re.M)
    if folded:
        return " ".join(line.strip() for line in folded.group(1).splitlines()).strip()
    plain = re.search(r"^%s:[ \t]*(.+)$" % key, block, re.M)
    return plain.group(1).strip() if plain else ""


def main():
    if not os.path.isdir(SKILLS):
        print("skills/ not found; run from the laruche/ directory", file=sys.stderr)
        return 1

    schemas = tool_schemas()
    tools = set(schemas)
    folders = sorted(
        entry
        for entry in os.listdir(SKILLS)
        if os.path.isfile(os.path.join(SKILLS, entry, "SKILL.md"))
    )
    problems = []

    # Tool-ish mentions: `name(` in a call, or a bare snake_case identifier followed by
    # "(". Only flag names that look like our registry (a leading verb-ish segment and an
    # underscore) to keep example function names out of the report.
    call = re.compile(r"`?\b([a-z][a-z0-9]*(?:_[a-z0-9]+)+)\s*\(")
    known_prefixes = tuple(sorted({t.split("_")[0] for t in tools}))

    for folder in folders:
        path = os.path.join(SKILLS, folder, "SKILL.md")
        text = read(path)
        block = frontmatter(text)
        where = "skills/%s/SKILL.md" % folder

        if "type: skill" not in block:
            problems.append("%s: missing `type: skill`, the disk sync ignores it" % where)

        name = field(block, "name")
        if name != folder:
            problems.append(
                "%s: `name: %s` disagrees with the folder name `%s`" % (where, name, folder)
            )

        for key in re.findall(r"^([a-z_-]+):", block, re.M):
            if key not in READ_FIELDS:
                problems.append("%s: frontmatter field `%s` is read by nothing" % (where, key))

        desc = field(block, "description")
        if not desc:
            problems.append("%s: no description" % where)
        else:
            shown = resumer_description(desc)
            if shown != desc:
                problems.append(
                    "%s: description is truncated in the catalog (%d chars)\n"
                    "      written: %s\n      model sees: %s" % (where, len(desc), desc, shown)
                )
            if not re.match(r"^[A-Z][a-z]+[,ns]?[ ,]", desc):
                problems.append("%s: description should start with a verb: %s" % (where, desc))

        for candidate in set(call.findall(text)):
            if candidate in tools:
                continue
            if candidate.startswith(known_prefixes) and candidate.split("_")[0] in {
                t.split("_")[0] for t in tools
            }:
                problems.append(
                    "%s: `%s(` is not a registered tool (closest family: %s_*)"
                    % (where, candidate, candidate.split("_")[0])
                )

        # Every argument a skill attributes to a tool must exist in that tool's schema.
        # Two forms are checked: `tool(arg=...)` in an example, and the prose form
        # "`tool` with `arg`", which is how the LaRuche-written skills phrase it.
        for tool, arglist in re.findall(r"\b([a-z][a-z0-9_]*_[a-z0-9_]+)\(([^)]*)\)", text):
            if tool not in schemas:
                continue
            # Anchor on "(" or a comma, so a query string such as "?format=3" inside a
            # URL argument is not mistaken for a named parameter.
            for arg in re.findall(r"(?:^|,)\s*([a-z_][a-z0-9_]*)\s*=", arglist):
                if arg not in schemas[tool]:
                    problems.append(
                        "%s: `%s(%s=...)` is not an argument of %s, which takes %s"
                        % (where, tool, arg, tool, sorted(schemas[tool]) or "nothing")
                    )
        prose = re.compile(
            r"`([a-z][a-z0-9_]*_[a-z0-9_]+)`\s+(?:with|takes)\s+"
            r"((?:`[a-z_][a-z0-9_]*`(?:\s*(?:,|and|or|plus)\s*)?)+)"
        )
        for tool, args in prose.findall(text):
            if tool not in schemas:
                continue
            for arg in re.findall(r"`([a-z_][a-z0-9_]*)`", args):
                if arg not in schemas[tool]:
                    problems.append(
                        "%s: text says `%s` with `%s`, but %s takes %s"
                        % (where, tool, arg, tool, sorted(schemas[tool]) or "no arguments")
                    )

        for ref in set(re.findall(r"`([a-z][a-z0-9_-]{3,})`\s+skill", text)):
            if ref not in folders:
                problems.append("%s: refers to skill `%s`, which does not exist" % (where, ref))

        for rel in set(re.findall(r"(?<![\w/])((?:\.\./)?(?:scripts|templates|references)/[\w./-]+)", text)):
            # A path is valid if it resolves inside the skill folder (a bundled file) or
            # from the repository root (a shared script such as scripts/check_skills.py).
            candidates = [
                os.path.normpath(os.path.join(SKILLS, folder, rel)),
                os.path.normpath(os.path.join(ROOT, rel)),
            ]
            if not any(os.path.exists(candidate) for candidate in candidates):
                problems.append("%s: references `%s`, which is not on disk" % (where, rel))

    # An em dash, or the name of another agent, anywhere in the library, bundled scripts
    # and templates included. A borrowed name in a review template ends up published in
    # someone's pull request; a borrowed runtime path fails where nobody can read why.
    # The ban is on another AGENT's identity: its name, its runtime, its signature. It is
    # not on vendor names as such. `openai` is deliberately absent: it is the name of a
    # Python package and of the wire protocol llama.cpp's server speaks, so a llama-cpp
    # page that says "OpenAI-compatible API" is being accurate, not borrowing an identity.
    foreign = re.compile(
        r"third-party|nous\s*research|third-party|open\s*claw|claude|anthropic|"
        r"chatgpt|copilot|cursor\.(?:so|com)|codeium",
        re.I,
    )
    for folder, _, files in os.walk(SKILLS):
        for name in files:
            target = os.path.join(folder, name)
            shown = os.path.relpath(target, ROOT).replace("\\", "/")
            for number, line in enumerate(read(target).splitlines(), 1):
                if "—" in line:
                    problems.append("%s:%d: em dash" % (shown, number))
                hit = foreign.search(line)
                if hit:
                    problems.append(
                        "%s:%d: names another agent or vendor (%r)" % (shown, number, hit.group(0))
                    )

    print("skills checked      : %d" % len(folders))
    print("registered tools    : %d" % len(tools))
    if problems:
        print("\n%d problem(s):\n" % len(problems))
        for problem in problems:
            print("  - %s" % problem)
        return 1
    print("\nOK: frontmatter, descriptions, tool names and cross-references are clean.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
