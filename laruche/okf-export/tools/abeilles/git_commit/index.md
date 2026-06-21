---
type: memory-node
title: "git_commit"
description: "Sous-noeud de tools.abeilles"
id: "tools.abeilles.git_commit"
timestamp: 2026-06-21T16:46:02.524853100+00:00
---

# git_commit

- Abeille `git_commit`: Stage all changes and create a git commit with the given message. Schema: {"description":"Stage all changes and create a git commit with the given message.","name":"git_commit","parameters":{"properties":{"add_all":{"description":"Stage all changes before committing (default true)","type":"boolean"},"message":{"description":"Commit message","type":"string"}},"required":["message"],"type":"object"}}  _(source: tool-registry)_
