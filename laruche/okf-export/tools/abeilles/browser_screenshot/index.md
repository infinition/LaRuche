---
type: memory-node
title: "browser_screenshot"
description: "Sous-noeud de tools.abeilles"
id: "tools.abeilles.browser_screenshot"
timestamp: 2026-06-21T16:46:02.524853100+00:00
---

# browser_screenshot

- Abeille `browser_screenshot`: Take a screenshot of a web page. Returns the path to the saved screenshot file. Useful for visual inspection or sharing with users. Schema: {"description":"Take a screenshot of a web page. Returns the path to the saved screenshot file. Useful for visual inspection or sharing with users.","name":"browser_screenshot","parameters":{"properties":{"output_path":{"description":"Path to save the screenshot (default: screenshot.png)","type":"string"},"url":{"description":"The URL to screenshot","type":"string"}},"required":["url"],"type":"object"}}  _(source: tool-registry)_
