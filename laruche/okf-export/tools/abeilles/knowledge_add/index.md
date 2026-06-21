---
type: memory-node
title: "knowledge_add"
description: "Sous-noeud de tools.abeilles"
id: "tools.abeilles.knowledge_add"
timestamp: 2026-06-21T16:46:02.524853100+00:00
---

# knowledge_add

- Abeille `knowledge_add`: Add information to the persistent knowledge base. The information will be stored with an embedding and can be retrieved later via semantic search. Use this to remember important facts, user preferences, or any information that should persist across conversations. Schema: {"description":"Add information to the persistent knowledge base. The information will be stored with an embedding and can be retrieved later via semantic search. Use this to remember important facts, user preferences, or any information that should persist across conversations.","name":"knowledge_add","parameters":{"properties":{"source":{"description":"Optional source (e.g., 'user said', 'web search', 'file: x.txt')","type":"string"},"text":{"description":"The information to remember","type":"string"}},"required":["text"],"type":"object"}}  _(source: tool-registry)_
