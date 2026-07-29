#!/usr/bin/env python3
"""Reference MCP server. Copy this file to start your own.

A LaRuche node can BE an MCP server (it exposes its own tools on /mcp) and it can
also USE other MCP servers. This file is the second case: a minimal server LaRuche
launches as a child process and talks to over stdin/stdout.

Register it in Settings > Capabilities > MCP, or by hand in `mcp_servers.json`:

    {
      "mcpServers": {
        "example": { "command": "python", "args": ["mcp/example_mcp.py"] }
      }
    }

Then the `compter_mots` tool below shows up alongside the built-in tools, and the
agent can call it like any other.

The protocol is JSON-RPC 2.0, one message per line. No dependency, no framework:
everything a server strictly needs is in this file, so you can read it end to end
before trusting it. Anything printed on stdout that is NOT a JSON-RPC message
breaks the transport - use stderr for your own traces.
"""

import json
import sys

PROTOCOLE = "2024-11-05"

# Chaque entree devient un outil visible par l'agent. Le schema sert a la fois de
# validation et de documentation: c'est ce que le modele lit pour decider quoi
# passer, donc une description vague donne des appels approximatifs.
OUTILS = [
    {
        "name": "compter_mots",
        "description": (
            "Count the words, lines and characters of a text. "
            "Reference tool: it does something trivial so that what you observe "
            "is the plumbing, not the logic."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "texte": {"type": "string", "description": "The text to measure."}
            },
            "required": ["texte"],
        },
    }
]


def executer(nom, arguments):
    """Runs a tool and returns its textual result."""
    if nom == "compter_mots":
        texte = arguments.get("texte", "")
        return (
            f"{len(texte.split())} mots, "
            f"{len(texte.splitlines())} lignes, "
            f"{len(texte)} caracteres"
        )
    raise ValueError(f"outil inconnu: {nom}")


def repondre(ident, resultat=None, erreur=None):
    """Writes one JSON-RPC response line.

    A notification (no `id`) expects no answer: replying to one is a protocol
    error that some clients treat as fatal.
    """
    if ident is None:
        return
    message = {"jsonrpc": "2.0", "id": ident}
    if erreur is not None:
        message["error"] = erreur
    else:
        message["result"] = resultat
    sys.stdout.write(json.dumps(message) + "\n")
    sys.stdout.flush()


def main():
    for ligne in sys.stdin:
        ligne = ligne.strip()
        if not ligne:
            continue
        try:
            requete = json.loads(ligne)
        except json.JSONDecodeError:
            # Impossible de repondre: sans `id` valide on ne sait pas a qui.
            print("ligne illisible ignoree", file=sys.stderr)
            continue

        ident = requete.get("id")
        methode = requete.get("method")
        params = requete.get("params") or {}

        if methode == "initialize":
            repondre(
                ident,
                {
                    "protocolVersion": PROTOCOLE,
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "example", "version": "1.0.0"},
                },
            )
        elif methode == "tools/list":
            repondre(ident, {"tools": OUTILS})
        elif methode == "tools/call":
            nom = params.get("name", "")
            try:
                texte = executer(nom, params.get("arguments") or {})
                repondre(ident, {"content": [{"type": "text", "text": texte}]})
            except Exception as e:
                # Une erreur d'OUTIL se rend dans le resultat, avec isError: l'agent
                # doit pouvoir la lire et reessayer. Une erreur de PROTOCOLE, elle,
                # passe par le champ `error` - les deux ne se confondent pas.
                repondre(
                    ident,
                    {"content": [{"type": "text", "text": str(e)}], "isError": True},
                )
        elif methode in ("notifications/initialized", "initialized"):
            pass  # notification: rien a repondre
        else:
            repondre(ident, erreur={"code": -32601, "message": f"methode inconnue: {methode}"})


if __name__ == "__main__":
    main()
