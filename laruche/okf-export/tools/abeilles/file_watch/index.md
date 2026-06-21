---
type: memory-node
title: "file_watch"
description: "Sous-noeud de tools.abeilles"
id: "tools.abeilles.file_watch"
timestamp: 2026-06-21T16:46:02.524853100+00:00
---

# file_watch

- Abeille `file_watch`: Check if a file has been modified since a given timestamp. Returns whether the file was modified and its last modification time. Schema: {"description":"Check if a file has been modified since a given timestamp. Returns whether the file was modified and its last modification time.","name":"file_watch","parameters":{"properties":{"path":{"description":"Absolute or relative path to the file to watch","type":"string"},"since":{"description":"ISO 8601 timestamp to compare against (e.g., '2026-04-05T12:00:00Z')","type":"string"}},"required":["path","since"],"type":"object"}}  _(source: tool-registry)_
