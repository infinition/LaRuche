---
type: memory-node
title: "memory_search"
description: "Sous-noeud de tools.abeilles"
id: "tools.abeilles.memory_search"
timestamp: 2026-06-21T16:46:02.524853100+00:00
---

# memory_search

- Abeille `memory_search`: Recherche dans la mémoire cognitive durable (carte de nœuds + items, activation + retrieval hybride). Renvoie les faits/décisions/préférences pertinents stockés lors de conversations précédentes. À appeler avant un travail de fond pour s'orienter. Schema: {"description":"Recherche dans la mémoire cognitive durable (carte de nœuds + items, activation + retrieval hybride). Renvoie les faits/décisions/préférences pertinents stockés lors de conversations précédentes. À appeler avant un travail de fond pour s'orienter.","name":"memory_search","parameters":{"properties":{"limit":{"description":"Nombre max d'items (défaut 8)","type":"integer"},"query":{"description":"Termes de recherche (intention de l'utilisateur)","type":"string"}},"required":["query"],"type":"object"}}  _(source: tool-registry)_
