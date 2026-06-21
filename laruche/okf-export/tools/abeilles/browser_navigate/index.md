---
type: memory-node
title: "browser_navigate"
description: "Sous-noeud de tools.abeilles"
id: "tools.abeilles.browser_navigate"
timestamp: 2026-06-21T16:46:02.524853100+00:00
---

# browser_navigate

- Abeille `browser_navigate`: Navigate to a URL using a headless browser and return the page content as text. This is more powerful than web_fetch as it executes JavaScript and renders the page like a real browser. Use for pages that require JS rendering. Schema: {"description":"Navigate to a URL using a headless browser and return the page content as text. This is more powerful than web_fetch as it executes JavaScript and renders the page like a real browser. Use for pages that require JS rendering.","name":"browser_navigate","parameters":{"properties":{"url":{"description":"The URL to navigate to","type":"string"},"wait_seconds":{"description":"Seconds to wait for page load (default: 3)","type":"integer"}},"required":["url"],"type":"object"}}  _(source: tool-registry)_
