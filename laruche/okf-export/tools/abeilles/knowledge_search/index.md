---
type: memory-node
title: "knowledge_search"
description: "Sous-noeud de tools.abeilles"
id: "tools.abeilles.knowledge_search"
timestamp: 2026-06-21T16:46:02.524853100+00:00
---

# knowledge_search

- Abeille `knowledge_search`: Search the knowledge base for relevant information using semantic search. Returns the most relevant stored entries. Use this to recall previously stored information, user preferences, or facts from earlier conversations. Schema: {"description":"Search the knowledge base for relevant information using semantic search. Returns the most relevant stored entries. Use this to recall previously stored information, user preferences, or facts from earlier conversations.","name":"knowledge_search","parameters":{"properties":{"query":{"description":"The search query","type":"string"},"top_k":{"description":"Number of results (default: 5)","type":"integer"}},"required":["query"],"type":"object"}}  _(source: tool-registry)_
