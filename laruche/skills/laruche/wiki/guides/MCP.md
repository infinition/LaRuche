# MCP

LaRuche sits on both sides of the Model Context Protocol: it consumes external MCP
servers as extra tools, and it exposes itself as an MCP server so other agents can use
the hive.

## LaRuche as an MCP client

Add servers in **Settings > MCP**. Their tools join the tool registry and become
available to the agent alongside the built-ins, subject to the same approval gates and
the same secrets masking on outputs.

Dynamic tool selection applies too: MCP tools are matched against the conversation's
intent like any other tool, so connecting a large server does not flood every context.

## LaRuche as an MCP server

The node exposes an MCP endpoint at `POST /mcp`. Any MCP-capable client (Claude Code,
an IDE, another agent framework) can connect and use the hive's capabilities:

- query the cognitive memory,
- trigger agentic runs,
- reach the tools you choose to expose.

This turns LaRuche into infrastructure: a persistent memory and automation node that
your other AI tooling talks to.

## A concrete pattern

Run LaRuche 24/7 on a homelab box. Connect your laptop's coding agent to it over MCP.
The coding agent now has a durable, searchable memory that survives its own sessions,
plus scheduled jobs and watchers on a machine that never sleeps, without giving up its
own workflow.
