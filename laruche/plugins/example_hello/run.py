"""Reference plugin body. Copy this folder to start your own.

It shows the four things a plugin script has to get right, and each of them is a
real failure someone has already hit:

1. READ THE ARGUMENTS FROM argv, in the order the manifest's `command` passes
   them. The manifest is the contract; this file only honours it.
2. WRITE THE RESULT TO stdout. Whatever lands there is the observation handed
   back to the agent, so print what it needs and nothing else. Progress chatter
   belongs on stderr.
3. EXIT NON-ZERO ON FAILURE. A script that prints an error and exits 0 reports
   success, and the agent then builds on a result that does not exist.
4. SAY WHAT WENT WRONG. "Unsupported language 'de'" tells the agent how to fix
   its call; "error" sends it guessing.

Run it by hand exactly as the node would, from the laruche/ directory:

    python plugins/example_hello/run.py Fabien fr
"""

import sys

SALUTATIONS = {
    "fr": "bonjour",
    "en": "hello",
}


def main(argv):
    if not argv:
        # Usage goes to stderr: stdout is reserved for the result.
        print("usage: run.py <name> [language]", file=sys.stderr)
        return 2

    nom = argv[0]
    langue = argv[1] if len(argv) > 1 else "fr"

    salutation = SALUTATIONS.get(langue)
    if salutation is None:
        connues = ", ".join(sorted(SALUTATIONS))
        print(
            "Unsupported language '%s'. Known: %s." % (langue, connues),
            file=sys.stderr,
        )
        return 1

    print("%s %s" % (salutation, nom))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
