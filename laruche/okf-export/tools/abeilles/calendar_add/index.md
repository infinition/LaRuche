---
type: memory-node
title: "calendar_add"
description: "Sous-noeud de tools.abeilles"
id: "tools.abeilles.calendar_add"
timestamp: 2026-06-21T16:46:02.524853100+00:00
---

# calendar_add

- Abeille `calendar_add`: Add an event or reminder to the calendar. Specify title, date (YYYY-MM-DD), optional time (HH:MM), and optional description. Schema: {"description":"Add an event or reminder to the calendar. Specify title, date (YYYY-MM-DD), optional time (HH:MM), and optional description.","name":"calendar_add","parameters":{"properties":{"date":{"description":"Date in YYYY-MM-DD format","type":"string"},"description":{"description":"Optional description","type":"string"},"time":{"description":"Optional time in HH:MM format","type":"string"},"title":{"description":"Event title","type":"string"}},"required":["title","date"],"type":"object"}}  _(source: tool-registry)_
