---
type: memory-node
title: "file_write"
description: "Sous-noeud de tools.abeilles"
id: "tools.abeilles.file_write"
timestamp: 2026-06-21T16:46:02.524853100+00:00
---

# file_write

- Abeille `file_write`: Write text content to a file at the given path. Creates the file if it doesn't exist, overwrites if it does. Use with caution. Schema: {"description":"Write text content to a file at the given path. Creates the file if it doesn't exist, overwrites if it does. Use with caution.","name":"file_write","parameters":{"properties":{"content":{"description":"The text content to write","type":"string"},"path":{"description":"The file path to write to","type":"string"}},"required":["path","content"],"type":"object"}}  _(source: tool-registry)_
