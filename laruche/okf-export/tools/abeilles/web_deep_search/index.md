---
type: memory-node
title: "web_deep_search"
description: "Sous-noeud de tools.abeilles"
id: "tools.abeilles.web_deep_search"
timestamp: 2026-06-21T16:46:02.524853100+00:00
---

# web_deep_search

- Abeille `web_deep_search`: Perform a deep web search: first searches the web, then automatically fetches and extracts content from the top 3 results. Returns both search snippets AND full page content. Use this for thorough research when you need detailed information, not just snippets. Schema: {"description":"Perform a deep web search: first searches the web, then automatically fetches and extracts content from the top 3 results. Returns both search snippets AND full page content. Use this for thorough research when you need detailed information, not just snippets.","name":"web_deep_search","parameters":{"properties":{"num_results":{"description":"Number of results to fetch in detail (default: 3, max: 5)","type":"integer"},"query":{"description":"The search query","type":"string"}},"required":["query"],"type":"object"}}  _(source: tool-registry)_
