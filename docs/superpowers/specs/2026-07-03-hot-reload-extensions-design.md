# Hot-reload of plugins, skills, and MCP — design

**Date:** 2026-07-03
**Goal:** editing a skill (`.md`), a plugin-agent manifest (`.json`), or `mcp.json`
takes effect while crew is running — no app or pane restart.

## Current behavior (audit)

| Surface | Loaded | Stale? |
|---|---|---|
| Skills (`~/.config/crew/skills/`, `./.crew/skills/`) | `skills::load()` per `/skill` / `/skills` call | **already fresh** per use |
| Plugin agents (`~/.config/crew/agents/`, `./.crew/agents/`) | `Session::registry()` → `discover::roster_with()` → `plugins::append()` per message | **already fresh** per message, but the pane's Roster (agent badges) is only pushed on `hello` and `/model` — a new manifest works yet doesn't show |
| MCP (`~/.config/crew/mcp.json`, `./.crew/mcp.json`) | `McpHost::from_config()` once in `Session::default()`; per-server tool cache never invalidated; clients live until a call errors | **stale until restart** — the real gap |

## Design

Sync-on-use plus an explicit `/reload` construct. No file-watcher thread:
extension state only matters at the moment a message uses it, and skills and
plugin manifests already re-read from disk at that moment. MCP gets the same
property; `/reload` adds forced refresh and user-visible feedback. All file
I/O happens in the broker subprocess (never the winit thread) and is bounded
(two small JSON files, a couple of `read_dir`s).

### 1. `McpHost` config sync (`crates/crew-plugin/src/mcp/mod.rs`)

- `McpHost` gains `auto: bool` — `true` only when built by `from_config()`,
  so test hosts built with `new(map)` keep their explicit maps.
- `sync()`: when `auto`, re-load the merged `mcp.json` config. If it differs
  from `self.servers`: servers that were **removed or changed** get their
  client dropped (drop kills the child) and their tool-cache entry cleared;
  **new/changed** configs are inserted; **unchanged** servers keep their live
  connection and cache. Called at the top of `tools()`, `call()`, `report()`.
- `reload()`: forced variant for `/reload` — `sync()` then drop *all* clients
  and the whole tool cache, so the next use reconnects and re-lists tools
  (catches a running server that gained tools).

### 2. `/reload` construct (`crates/crew-plugin/src/broker/commands.rs`)

Quick (inline) command:
1. `McpHost::reload()` on the session's shared host.
2. Re-emits the `Roster` event from a fresh `session.registry()` so the
   pane's agent badges pick up manifest adds/removes/edits live.
3. Reports one line each: skills loaded, plugin agents installed, MCP report.

Added to `HELP`, `CONSTRUCTS` (did-you-mean), and quick-command routing.
`/agents` also re-emits the `Roster` event (it already rebuilds the registry).

### 3. What deliberately doesn't change

- No watcher thread (`notify`): updates can't matter while idle; sync-on-use
  is strictly simpler and covers the same user story.
- Skills/plugin loading paths: already hot; behavior locked by tests instead.

## Error handling

- Unreadable/malformed `mcp.json` already parses to an empty map — sync then
  drops all servers; a later fix re-adds them (same as today's startup rule).
- A changed server that fails to reconnect reports its error via the existing
  `report()` path; unchanged servers are untouched.

## Testing

- `mcp/mod.rs` unit tests: config change swaps/drops/keeps the right clients
  and cache entries; `new(map)` hosts never auto-sync; `reload()` clears the
  cache. (Config source injected via the existing merged-load function plus a
  test seam that swaps the loader.)
- `commands` tests: `/reload` is quick, emits a `Roster` event and a report
  under `CREW_BROKER_MOCK_REPLY`; `/help` lists it; typo suggests it.
- Skills/plugins freshness: tests asserting a file written *after* load is
  picked up by the next `load()` call (locks the already-hot behavior).
