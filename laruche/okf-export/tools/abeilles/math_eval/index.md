---
type: memory-node
title: "math_eval"
description: "Sous-noeud de tools.abeilles"
id: "tools.abeilles.math_eval"
timestamp: 2026-06-21T16:46:02.524853100+00:00
---

# math_eval

- Abeille `math_eval`: Evaluate a mathematical expression. Supports: +, -, *, /, %, ** (power), parentheses, and common functions like sqrt, abs, sin, cos, pi, e. Use this for precise calculations instead of computing in your head. Schema: {"description":"Evaluate a mathematical expression. Supports: +, -, *, /, %, ** (power), parentheses, and common functions like sqrt, abs, sin, cos, pi, e. Use this for precise calculations instead of computing in your head.","name":"math_eval","parameters":{"properties":{"expression":{"description":"The mathematical expression to evaluate (e.g., '(42 * 3.14) + sqrt(16)')","type":"string"}},"required":["expression"],"type":"object"}}  _(source: tool-registry)_
