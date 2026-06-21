---
type: memory-node
title: "file_search"
description: "Sous-noeud de tools.abeilles"
id: "tools.abeilles.file_search"
timestamp: 2026-06-21T16:46:02.524853100+00:00
---

# file_search

- Abeille `file_search`: Search for files matching a pattern in a directory tree. Returns matching file paths. Useful for finding files by name or extension. Schema: {"description":"Search for files matching a pattern in a directory tree. Returns matching file paths. Useful for finding files by name or extension.","name":"file_search","parameters":{"properties":{"max_depth":{"description":"Maximum directory depth (default: 5)","type":"integer"},"path":{"description":"Root directory to search in","type":"string"},"pattern":{"description":"Search pattern (case-insensitive substring match on filename)","type":"string"}},"required":["path","pattern"],"type":"object"}}  _(source: tool-registry)_
