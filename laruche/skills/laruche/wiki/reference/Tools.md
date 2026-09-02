# Tools

LaRuche registers 89 built-in tools in a default node build. `computer` and `camera`
are present when default features are enabled. Plugins and MCP servers can add more at
runtime. The Capabilities page shows the exact registry for the running node.

A tool is a callable capability. An abeille is an agent. Older Rust type names such as
`AbeilleRegistry` remain in the code for compatibility, but they do not define the
public vocabulary.

Tool arguments are checked against JSON Schema before execution. Disabled tools are
blocked at dispatch, sensitive operations use the approval policy, and every result
passes through secret masking.

## Files, execution and runtime

| Tool | Purpose |
|---|---|
| `file_read` | Read a file with bounded output. |
| `read_extract` | Extract the relevant content from a large text or document. |
| `file_list` | List files and directories. |
| `file_write` | Create or replace a file, subject to approval. |
| `file_edit` | Replace a precise block inside an existing file. |
| `file_search` | Search names and file content. |
| `shell_exec` | Run a shell command with live, bounded output. |
| `execute_code` | Run a Python snippet with a timeout. |
| `todo` | Maintain the current run's task list. |
| `math_eval` | Evaluate a mathematical expression. |
| `lsp` | Query a language server for symbols, definitions and diagnostics. |
| `task_complete` | Mark a long-running task as complete. |
| `system_info` | Read node, operating system and resource information. |

## Web, browser and media

| Tool | Purpose |
|---|---|
| `web_search` | Search through configured providers or free fallbacks. |
| `web_fetch` | Fetch pages and PDFs with pagination, structured data, `focus` and `probe`. |
| `web_deep_search` | Run bounded parallel research with scout agents. |
| `web_discover` | Enumerate hidden site surfaces through links, sitemaps and certificate logs. |
| `image_search` | Search for images. |
| `media_present` | Present image, audio or video output in the conversation. |
| `browser` | Control a persistent browser session through the DOM and Chrome DevTools Protocol. |
| `computer` | Control desktop applications through accessibility refs or screen coordinates. |
| `camera` | List cameras or capture one still frame. |
| `file_watch` | Check whether a file changed after a given timestamp. |

### `browser`

`browser` keeps one session open across calls. It has four connection modes:

| Mode | Behavior |
|---|---|
| `auto` | Use the Chrome extension when connected, otherwise launch a managed browser. |
| `extension` | Require the user's current Chrome, open tabs and signed-in sessions. |
| `launch` | Start a browser with LaRuche's persistent profile. |
| `attach` | Connect to an existing remote-debugging port. |

Its actions are:

`navigate`, `read`, `find`, `overlays`, `click`, `right_click`, `double_click`,
`middle_click`, `drag`, `fill`, `upload`, `key`, `hover`, `scroll`, `wait`, `eval`,
`screenshot`, `console`, `network`, `back`, `forward`, `cookies`, `download`, `tabs`,
`open_tab`, `select`, `resize`, `dialog` and `close`.

`read` returns page text and numbered element refs. `find` limits that map to matching
content. `overlays` identifies what covers the page, including consent banners and
modal walls. The tool crosses same-origin frames and shadow DOM, handles real mouse
events, waits on text or selectors, manages tabs and dialogs, inspects console and
network logs, and emulates mobile or tablet viewports with touch input.

Cookie names and sizes can be inspected, never values. Consent is never accepted
automatically. A visible amber frame, action panel and animated cursor can be disabled
for clean screenshots or faster runs.

### Chrome extension

The dedicated Manifest V3 extension is in `extension-chrome/`. See
[Chrome Extension](Chrome-Extension) for installation and the full security model. It connects to
`/ws/navigateur` on the local node and gives `browser` access to the Chrome instance the
user already has open. This preserves active tabs and authenticated sessions without
copying cookies into LaRuche.

The extension also supports explicit page or note capture into cognitive memory,
downloads, tab capture, desktop capture and showcase recording. It does not send data
to a third-party service. Keep it disabled when the node is not running because its
bridge is intentionally local and trusts the configured node port.

### `computer`

`computer` is the native desktop tool. Its actions are:

`windows`, `focus_window`, `read`, `find`, `focus`, `screens`, `screenshot`,
`cursor_position`, `mouse_move`, `left_click`, `right_click`, `middle_click`,
`double_click`, `triple_click`, `left_click_drag`, `mouse_down`, `mouse_up`, `scroll`,
`fill`, `type`, `key`, `key_down`, `key_up`, `release_all`, `wait`, `read_clipboard`,
`write_clipboard`, `move_window`, `minimize_window`, `maximize_window`,
`restore_window` and `close_window`.

On Windows, `read` uses UI Automation and returns numbered refs for controls. This is
the preferred path because it does not require vision or screen coordinates. Pixel
control remains available for canvas, games and applications without useful
accessibility metadata. Multi-monitor coordinates, mixed DPI, minimized windows and
elevated applications are handled explicitly.

LaRuche refuses to control its own windows. A halo shows active control, human pointer
movement interrupts an automated sequence, held inputs are released after a timeout,
and `Ctrl+Alt+Shift+H` is the global stop shortcut.

### `camera`

`camera` has two actions: `list` and `capture`. A capture returns one still frame and
immediately releases the device. There is no continuous camera mode and no video
recording. Each capture requires approval.

## Git and workspaces

| Tool | Purpose |
|---|---|
| `git_status` | Read repository status. |
| `git_diff` | Read a bounded diff and summary. |
| `git_log` | Read commit history. |
| `git_commit` | Stage and commit selected work. |
| `git_worktree_enter` | Create or enter an isolated git worktree. |
| `git_worktree_exit` | Leave and clean up the active task worktree. |

## Cognitive memory and skills

| Tool | Purpose |
|---|---|
| `memory_search` | Run hybrid semantic and full-text recall. |
| `memory_write` | Add a fact to cognitive memory. |
| `memory_update_item` | Update a memory item. |
| `memory_delete` | Delete a memory item. |
| `memory_move_item` | Move an item to another node. |
| `memory_review` | Review pending memory changes. |
| `memory_list_proposed` | List proposed items. |
| `memory_stats` | Read memory statistics. |
| `memory_mutations` | Read the mutation history. |
| `memory_tree` | Browse the node tree. |
| `memory_read_node` | Read one node and its items. |
| `memory_grep` | Search raw memory content. |
| `memory_doctor` | Diagnose memory structure and indexes. |
| `memory_delete_node` | Delete a node. |
| `memory_create_node` | Create a node. |
| `memory_update_node` | Rename or update a node. |
| `memory_suggest_nodes` | Suggest destinations for uncategorized items. |
| `memory_consolidate` | Merge related items with model-assisted review. |
| `skill_list` | List skills stored in cognitive memory. |
| `skill_view` | Read one skill. |
| `skill_create` | Propose or create a skill. |
| `skill_patch` | Update a skill. |
| `skill_delete` | Delete a skill. |

## Automation, channels and history

| Tool | Purpose |
|---|---|
| `cron_create` | Create or update a scheduled prompt. |
| `cron_list` | List scheduled prompts and mission schedules. |
| `cron_delete` | Delete a cron. |
| `mission_create` | Create a persistent long-running goal. |
| `mission_list` | List missions. |
| `mission_delete` | Delete a mission. |
| `run_now` | Force an eligible scheduled item to run now. |
| `watcher_create` | Create a watcher with a validated compiled rule. |
| `watcher_list` | List watchers and their state. |
| `watcher_delete` | Delete a watcher. |
| `kanban_create` | Add a kanban task. |
| `kanban_list` | List kanban tasks. |
| `calendar_add` | Add a calendar event. |
| `calendar_list` | List calendar events. |
| `mesh_send` | Send a message to another hive. |
| `session_search` | Search saved conversations. |

## Run control and agent coordination

| Tool | Purpose |
|---|---|
| `run_script` | Execute an approved sequence of registered tool calls. |
| `tool_search` | Search the live registry for a tool. |
| `tool_call` | Call a tool by name through the normal dispatcher. |
| `delegate` | Send a bounded subtask to a scout agent. |
| `mixture_of_agents` | Ask several model profiles and combine their answers. |
| `spawn_specialist` | Start a named specialist agent. |
| `clarify` | Ask the user a question during a run. |
| `research_mode` | Declare deep-research mode to the engine. |
| `plan_mode` | Create or update long-run planning state. |
| `finding` | Record a verified finding in the run ledger. |

The [Table Ronde](Table-Ronde) uses the same model profiles and a mission-specific
whitelist of registered tools through dedicated API routes. It adds a constitution,
specialist pool, live rounds, disagreement tracking, arbitration and debate history. It
is an interface-level workflow, not one extra tool name in the default registry.

## Skills, plugins and MCP management

| Tool | Purpose |
|---|---|
| `skill_file_write` | Write a supporting file inside a skill. |
| `skill_file_read` | Read a supporting skill file. |
| `skill_file_delete` | Delete a supporting skill file. |
| `skill_file_list` | List files belonging to a skill. |
| `plugin_create` | Create a JSON tool plugin. |
| `plugin_list` | List installed plugins. |
| `plugin_delete` | Delete a plugin and remove its tools. |
| `reload_plugins` | Reload tool plugins from disk. |
| `mcp_add` | Add an external MCP server. |
| `mcp_remove` | Remove an MCP server. |
| `mcp_list` | List configured MCP servers. |

Tools provided by connected MCP servers join the registry dynamically and therefore do
not appear in the fixed list above. MCP resources are available through dynamically
registered resource tools when a server exposes them.
