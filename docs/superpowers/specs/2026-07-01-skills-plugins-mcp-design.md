# Skills, Plugin Agents, and MCP — design

2026-07-01. Adds the three extension surfaces modern coding tools ship
(Claude Code, Codex, opencode) to Crew's `/crew` multi-agent pane: **skills**
(reusable prompt playbooks), **plugin agents** (bring your own agent CLI via a
manifest, no recompile), and **MCP** (Model Context Protocol tool servers the
inbuilt agents can call). All three live in `crew-plugin` — the broker is the
AI surface — and respect every guardrail (≤200-line files, no overlays, no new
external dependencies; `dirs` is already a workspace dependency).

## 1. Skills — reusable prompt playbooks

Markdown files with an optional frontmatter header:

```markdown
---
name: review-checklist
description: A strict review checklist for Rust changes
---
When reviewing, check: unsafe blocks, unwraps on foreign input, …
```

- **Discovery:** `~/.config/crew/skills/*.md` (user) and `./.crew/skills/*.md`
  (project, relative to the broker's cwd). A project skill overrides a user
  skill with the same name. Missing frontmatter → name = file stem,
  description = first non-empty body line (clipped).
- **Surface:** two broker constructs in the `/crew` pane:
  - `/skills` — list loaded skills (name — description — origin).
  - `/skill <name> <task>` — run the normal relay with the skill body
    prepended to the task (`SKILL "<name>":\n<body>\n\nTASK:\n<task>`), so
    every agent in the thread sees the playbook.
- **Module:** `broker/skills.rs` (pure load/parse + unit tests); command
  wiring in `broker/commands.rs`; `/skills`+`/skill` added to `HELP` and the
  composer's `CONSTRUCTS` completion in `crew-app`.

## 2. Plugin agents — bring your own agent CLI

A JSON manifest turns any headless CLI into a roster agent (the existing
`CliAdapter` already abstracts claude/codex/opencode this way):

```json
{
  "name": "aider",
  "command": "aider",
  "args": ["--message", "{}", "--yes-always"],
  "role": "repo-wide edits"
}
```

- **Discovery:** every `*.json` in `~/.config/crew/agents/` and
  `./.crew/agents/` (project wins on name collision). `{}` in `args` is the
  message placeholder; appended as a final arg when absent. Names are
  lowercased; manifests whose `command` is not on `$PATH` are dropped by the
  existing probe, and manifests that shadow an inbuilt agent name are ignored.
- **Roster:** `Registry::discover` appends probed plugin agents after the
  inbuilt API agents — and now returns a non-empty roster even with **no API
  key** when plugin agents exist, so `/crew` works with CLI-only rosters.
- **Roles:** the `Adapter` trait gains `fn role(&self) -> &str` (default
  delegates to the static `role_for`); `CliAdapter`/`ApiAdapter` carry their
  role so manifest roles show in the roster and peer lists.
- **Module:** `broker/plugins.rs` (manifest parse + load + tests).

## 3. MCP — Model Context Protocol tool servers

A minimal MCP **client** (stdio transport, line-delimited JSON-RPC 2.0):
`initialize` → `notifications/initialized` → `tools/list` → `tools/call`.
No new dependencies — `serde_json` + `std::process` only.

- **Config:** Claude Code's familiar schema, merged from
  `~/.config/crew/mcp.json` and `./.crew/mcp.json` (project wins):

```json
{ "mcpServers": { "fs": { "command": "mcp-server-fs", "args": ["--root", "."], "env": {} } } }
```

- **Client:** `mcp/client.rs` spawns the server, a reader thread feeds parsed
  JSON lines into an mpsc channel; each request waits with a deadline
  (default 30 s) and matches on `id`, skipping notifications. Kill-on-drop
  like the plugin host. `mcp/mod.rs` holds `McpHost`: lazy connect per
  server, `tools() -> Vec<McpTool {server, name, description}>`,
  `call(server, tool, args_json) -> Result<String, String>` (text content
  concatenated; `isError` → `Err`).
- **Surface:**
  - `/mcp` construct — connects and lists each configured server's tools
    (quick command; lazy servers spawn on first use).
  - **Tool calls in the relay:** when tools exist, `frame()` appends a TOOLS
    section (one line per tool) and the directive: end the reply with
    `@tool <server>:<tool> {"arg": …}` to call one. The engine detects the
    directive (new `broker/toolcall.rs`), executes it through a
    `tools: Option<Arc<dyn ToolRunner>>` handle on `Broker`, and re-dials the
    same agent with the tool result appended — bounded at 4 tool rounds per
    hop, every call logged as a hop (`[tool] server:tool → …`).
- **Session:** the broker `Session` owns one lazy `McpHost` shared with
  worker snapshots (`Arc<Mutex<…>>`), so `/mcp` and relays reuse connections.

## Error handling

Everything degrades to a readable message in the pane: missing dirs → empty
lists (`/skills` explains where to put files), bad JSON manifests/config →
skipped with the file named once, MCP spawn/timeout errors → the tool call
fails into the transcript and the agent continues (it sees the error text).
Nothing here can block the winit thread: all of it runs in the broker
subprocess; MCP servers get kill-on-drop and hard deadlines.

## Testing

Unit tests per module (frontmatter parse, manifest parse, config merge,
directive parse, tool-loop with a fake `ToolRunner`, registry append rules).
MCP client gets a unix-gated e2e test against a canned `sh` JSON-RPC server.
