# Built-in system tools (`sys`) for crew agents

2026-07-02. Approved approach: native broker-side tools (Approach A).

## Problem

Crew's inbuilt agents have no tools of their own — the only tool surface is MCP
servers from `mcp.json`, advertised through the `@tool` relay loop
(`broker/toolcall.rs`). With no `mcp.json` an agent asked to "summarize
README.md" can only ask the user to paste the file. Even with the filesystem
MCP server configured, agents still cannot run terminal commands (zip, git,
builds). Crew should be useful out of the box: read/write files and run shell
commands with zero configuration and no node/npx dependency.

## Design

A built-in tool provider under the reserved server name `sys`, implemented in
`crew-plugin` (the broker subprocess — off the winit thread, so bounded
blocking is fine) and merged into the existing `@tool` surface. Agents call it
exactly like MCP tools: `@tool sys:run {"cmd": "zip -r docs.zip docs/"}`. The
existing engine loop already logs every call and result as hops (visible in
the pane), clips results, and caps rounds at `MAX_TOOL_ROUNDS` per hop —
none of that changes.

### Components

- **`broker/systools.rs`** (new, ≤200 lines + `systools_tests.rs`): the `sys`
  tool implementations and their `McpTool` descriptors (reusing the existing
  descriptor struct so `hint_for` renders one uniform TOOLS section).
  - `sys:run {"cmd"}` — runs `/bin/sh -c <cmd>` in the broker's working
    directory, stdin null. Returns exit status + stdout + stderr, capture
    capped at 64 KB. Timeout `CREW_SYS_TIMEOUT_MS` (default 30 000 ms); on
    expiry the child is killed and the result says so.
  - `sys:read_file {"path"}` — UTF-8 read, capped like `run` output.
  - `sys:write_file {"path", "content"}` — create/overwrite; parent dirs must
    exist (agents can `sys:run mkdir -p`).
  - `sys:list_dir {"path"?}` — names + kind + size, defaults to `.`.
  - Relative paths resolve against the broker cwd; absolute paths are allowed
    (the shell can reach them anyway — the cwd scoping is a convention, not a
    sandbox, and the spec does not pretend otherwise).
- **`broker/session.rs`**: `McpTools` grows into `SessionTools`, owning the
  MCP host *and* the sys provider. `hint()` merges both tool lists into one
  `hint_for` section; `call()` dispatches `server == "sys"` locally and
  everything else to the MCP host. The "no MCP servers → no tools" gate at
  `attach` time changes to "attach whenever sys is enabled or MCP is
  non-empty".

### Safety model (approved: scoped + limits, no approval prompt)

- Non-interactive: stdin is null, child killed on timeout (kill-on-drop).
- Bounded: 30 s default timeout, 64 KB capture cap, existing 6 000-char clip
  into the agent prompt, `MAX_TOOL_ROUNDS = 4` per hop.
- Visible: every call/result is a logged hop in the crew pane.
- `CREW_SYS_TOOLS=0` disables the whole surface.
- `CREW_BROKER_MOCK_REPLY` (mock provider) skips sys tools exactly as it
  already skips MCP, keeping broker tests deterministic.

### Error handling

Tool errors return `Err(String)` and flow back to the agent as `ERROR: …`
through the existing loop (never crash the hop): unknown tool, bad JSON args,
missing file, non-UTF-8 content, spawn failure, timeout. `sys:run` treats a
non-zero exit as a *successful call* whose result reports the exit code —
agents need to see failing command output.

## Testing

- `systools_tests.rs`: echo round-trip, exit-code + stderr reporting, timeout
  kills a `sleep`, 64 KB cap, cwd resolution, each file tool's happy path and
  error path, `CREW_SYS_TOOLS=0`.
- `session.rs` tests: hint merges sys + MCP; dispatch routes `sys:` locally;
  disabled sys + empty MCP attaches no runner.
- Relay test in the `toolcall_tests.rs` style: scripted agent replies
  `@tool sys:run {"cmd":"echo hi"}`, hop log shows the call and `hi`, final
  reply routes normally.

## Out of scope

- Per-command user approval UI (Approach C) — can layer on later.
- A real sandbox (chroot/seatbelt), allowlists, or path jails.
- `sys:zip`/`sys:summarize` style specials: zip is just `sys:run`, and
  summarizing is the model's job once it can read.
