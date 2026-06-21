---
type: memory-node
title: "delegate"
description: "Sous-noeud de tools.abeilles"
id: "tools.abeilles.delegate"
timestamp: 2026-06-21T16:46:02.524853100+00:00
---

# delegate

- Abeille `delegate`: Delegate a sub-task to a fresh agent. The sub-agent will execute the task independently using all available tools and return the result. Use this for complex tasks that can be broken into independent sub-tasks, or when you need to run something in a separate context. Schema: {"description":"Delegate a sub-task to a fresh agent. The sub-agent will execute the task independently using all available tools and return the result. Use this for complex tasks that can be broken into independent sub-tasks, or when you need to run something in a separate context.","name":"delegate","parameters":{"properties":{"context":{"description":"Optional context or instructions for the sub-agent","type":"string"},"task":{"description":"The task description for the sub-agent to execute","type":"string"}},"required":["task"],"type":"object"}}  _(source: tool-registry)_
