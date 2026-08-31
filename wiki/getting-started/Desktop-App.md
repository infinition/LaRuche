# Desktop App

LaRuche ships as a desktop application, a server node and a terminal client. They share
the same Rust workspace and the same responsive web interface.

## The three executables

| Executable | Use |
|---|---|
| `laruche` | Normal desktop entry point. Opens the LaRuche window and starts a local node when needed. |
| `laruche-node` | Headless server, web UI, API, channels, MCP and background jobs. |
| `laruche-cli` | Terminal TUI for operating a node. |

The desktop application does not contain a second frontend. It displays the exact SPA
served by the node, so the browser, desktop and phone layouts do not drift apart.

## Full application

Windows and Linux releases include full installers. The node travels with the desktop
application. On launch, the application checks for a node already answering on
`127.0.0.1:8419`. If none exists, it starts the bundled node, waits for a real HTTP
response and then opens the window. It only stops a node that it started itself.

Portable archives include all three executables. macOS is distributed this way because
the project does not currently publish a signed DMG.

## Lightweight LAN client

The client variant contains the desktop shell without a node. It listens for Miel mDNS
announcements and connects to a hive already running on the local network. If one hive
is found, it opens directly. If several are found, a local selection page is shown.

The remote node must be started with `LARUCHE_BIND_LAN=1`. Discovery and reachability
are separate: a hive can announce itself over mDNS while still listening only on
loopback. `decouvrir_ruches.bat` reports both states on Windows.

## Selecting a target

`LARUCHE_URL` gives the desktop application an explicit target:

```text
LARUCHE_URL=http://192.168.1.20:8419
```

`LARUCHE_SANS_NOEUD=1` forces client-only mode and prevents the application from finding
or starting a local node.

## Data directory

Every launcher resolves to the same hive home unless `LARUCHE_DATA_DIR` says otherwise:

| Platform | Default |
|---|---|
| Windows | `%APPDATA%\LaRuche` |
| macOS | `~/Library/Application Support/LaRuche` |
| Linux | `$XDG_DATA_HOME/laruche`, normally `~/.local/share/laruche` |

This directory contains memory, sessions, skills, plugins, secrets, configuration and
journals. A source checkout that already contains hive state remains a valid home for
backward compatibility.
