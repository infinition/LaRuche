---
type: memory-node
title: "shell_exec"
description: "Sous-noeud de tools.abeilles"
id: "tools.abeilles.shell_exec"
timestamp: 2026-06-21T16:46:02.524853100+00:00
---

# shell_exec

- Abeille `shell_exec`: Execute a shell command and return its output (stdout + stderr). Use this for system tasks like checking disk space, listing processes, running git commands, etc. Dangerous commands are blocked. Schema: {"description":"Execute a shell command and return its output (stdout + stderr). Use this for system tasks like checking disk space, listing processes, running git commands, etc. Dangerous commands are blocked.","name":"shell_exec","parameters":{"properties":{"command":{"description":"The shell command to execute","type":"string"}},"required":["command"],"type":"object"}}  _(source: tool-registry)_
