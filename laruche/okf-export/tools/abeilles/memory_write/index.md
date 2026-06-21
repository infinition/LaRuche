---
type: memory-node
title: "memory_write"
description: "Sous-noeud de tools.abeilles"
id: "tools.abeilles.memory_write"
timestamp: 2026-06-21T16:46:02.524853100+00:00
---

# memory_write

- Abeille `memory_write`: Mémorise un fait, une décision ou une préférence durable dans la carte cognitive. Utiliser un node_id pointé : `projects.<nom>`, `decisions.<sujet>`, `people.<nom>`. À appeler après une décision ou un fait qui doit survivre aux conversations. Schema: {"description":"Mémorise un fait, une décision ou une préférence durable dans la carte cognitive. Utiliser un node_id pointé : `projects.<nom>`, `decisions.<sujet>`, `people.<nom>`. À appeler après une décision ou un fait qui doit survivre aux conversations.","name":"memory_write","parameters":{"properties":{"content":{"description":"Le fait à mémoriser","type":"string"},"node_id":{"description":"Nœud pointé, ex. decisions.archi","type":"string"},"source":{"description":"Provenance optionnelle","type":"string"}},"required":["node_id","content"],"type":"object"}}  _(source: tool-registry)_
