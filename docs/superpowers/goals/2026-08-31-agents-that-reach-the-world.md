# Goal — agents that reach the world: a real tool loop, and LangGraph behind the bridge

**Set:** 2026-08-31 by the user. "Can we add support for langgraph in crew if not already done, we
are going to need good framework, so that we can build amazing agents/tools to work end to end,
accessing API's etc."

**Answer to the literal question first: no, LangGraph is not in crew, and the reason it never
landed is more interesting than the absence.** The bridge for it was DESIGNED and BUILT — `wire/`
(the `RemoteTask`/`RemoteReply` JSON line protocol + the `Transport` trait), `worker/`
(`serve_stdio`, `LoopbackTransport`) and `remoteagent/` (`RemoteAgent`, `RemoteFactory`) exist
today, and the plan that produced them says in as many words that the point was "external
orchestration engines (LangGraph / custom Python)"
(`docs/superpowers/plans/2026-06-27-crew-hive-remote.md:5`). It is DEAD CODE IN PRODUCTION.
`RemoteFactory` is constructed in exactly three places and every one of them is a test
(`crates/crew-hive/tests/engine.rs:260`, `remoteagent/tests.rs:59`, and its own doc example).
Nothing spawns a sidecar, no config key selects one, no `/crew` path can reach one. The seam is
real; the road to it was never paved.

**And a concern, stated once, because building on a wrong premise is expensive.** LangGraph is a
Python library. crew is a single Rust binary that installs to `~/.local/bin` (and `%LOCALAPPDATA%`)
with no runtime dependency on anything — that zero-dependency install is a feature people notice.
More to the point: a LangGraph sidecar, wired to today's protocol, would be STRICTLY WORSE than
what crew already runs, because `RemoteTask` carries `{agent, task, prompt, model, deps}` and
nothing else — no tools, no credentials, no stream, no state. A graph engine that can only return a
string is a slower `ApiAgent`. So LangGraph is adopted here as AN ENGINE BEHIND THE BRIDGE, opt-in
and probed, never as crew's spine — and it is adopted only after the thing that actually blocks
"agents that access APIs end to end" is fixed, which is not the graph engine at all.

**THE REAL BLOCKER, and it is a one-line fact: crew's parallel swarm cannot call a tool.** Plain
`/crew` messages execute as a crew-hive swarm (`broker/swarm.rs`) whose agents come from
`ApiFactory` (`broker/swarmconf.rs:41,58`) — and `ApiAgent` has no tool loop, no `ToolRunner`, no
mention of the word "tool" anywhere in `apiagent/mod.rs`. It calls `Provider::complete` once and
returns text. The MCP host, the `@tool` relay, the credential store — all of it hangs off
`broker/engine.rs:83`, the SINGLE-AGENT `@agent` relay path, which `swarm.rs` never touches. crew
today has a parallel engine that cannot reach the world, and a world-reaching engine that cannot
run in parallel. THAT is the framework gap. LangGraph does not close it; only crew can, because the
tools are on crew's side of the wire.

THE GOAL, in one line: any agent crew runs — native or sidecar, alone or forty-wide — can call a
real tool with real credentials, in a loop, with the result flowing back into its reasoning; and
when a job wants cycles, checkpoints and human-in-the-loop that crew's DAG will not express, a
LangGraph graph runs it over a wire rich enough to be worth crossing.

THE SHAPE, which is a stronger requirement than the goal: this is built ONCE, and after it every
new capability is a FILE, not a commit. Adding the fortieth integration must cost a manifest and a
`/reload` — and the agent must still pick the right tool out of forty without the prompt growing to
match. Those two sentences are Pillars 2 and 3; the rest of this document exists to make them
affordable.

### Pillar 1 — the tool loop reaches the swarm, and it stops being a text convention
**SHIPPED 2026-08-31** (branch `swarm-tools`, two commits). The `Tools` trait and the `@tool`
parser moved down into crew-hive so both engines share one; `ApiAgent` runs the loop; the approval
gate moved from per-broker to per-session so four concurrent agents ask once, not four times; and
the provider layer grew native tool-use — `CompletionRequest.tools`/`.turns`, `Completion.calls`,
`Provider::supports_tools`, mapped to Anthropic's `tool_use`/`tool_result` blocks and the OpenAI
shape's `tool_calls`/`role:"tool"`. `ToolCatalog` owns the `server:tool` ↔ wire-name encoding in one
place. MCP schemas are no longer discarded, and the four `sys` tools ship hand-written ones. The
text convention remains the fallback for providers without tool support.

VERIFIED LIVE 2026-08-31 against qwen-max (DashScope, the OpenAI shape): "find the weather in Oslo
and in Tokyo via wttr.in" planned TWO tasks with no dependency between them, and both agents called
`sys:run` — the second call went out while the first was still in flight, which is the
`spawn_blocking` decision earning its keep — then each fed the real API response back into its own
answer. The native path is identifiable in the event log: arguments arrive as compact
`Value::to_string()` output, and the run publishes NO `OutputDelta` at all, because the OpenAI-shape
provider drops to non-streaming whenever `tools` is on the wire.

NOT DONE: the `@agent` relay still uses the text path (its adapters include CLI agents that are not
`Provider`s), and the GUI itself has not been driven — macOS assistive access is denied to
`osascript` on this machine, so the verification ran against the real broker binary over its real
stdio protocol rather than through the window.

`CompletionRequest` is `{model, system, prompt, max_tokens}` (`provider/mod.rs:53`) — a single-shot
string in, a string out. It CANNOT EXPRESS A TOOL-USE TURN, which is why the existing tool path had
to be invented in prose: the relay appends a TOOLS section and asks the model to make the final
line of its reply `@tool <server>:<tool> {"arg": …}` (`broker/toolcall.rs:26`). That convention is
why tool use in crew is capped at `MAX_TOOL_ROUNDS = 4` per hop, why the model sees a name and a
100-character description CLIP instead of a JSON schema, and why an argument typo returns a model
apology rather than a validation error. Providers have had native tool-use for years; crew is
hand-rolling a worse one over the top.

So the provider abstraction grows a conversation and tools: a `messages` list (assistant turns,
tool-use blocks, tool-result blocks) and a `tools: Vec<ToolDef>` carrying the MCP server's ACTUAL
`inputSchema` — `tools/list` already returns it and `McpTool` (`mcp/mod.rs:22`) throws it away,
keeping only a `String` description. `AnthropicProvider` and `OpenRouterProvider` map to their
native shapes; `MockProvider` scripts tool turns so this is testable without a key. Then `ApiAgent`
takes an `Option<Arc<dyn ToolRunner>>` and runs the loop, and `swarmconf::backend()` hands it the
session's `SessionTools` — the same host, same servers, same credentials the relay uses. The `@tool`
text path stays as the fallback for providers with no native tool API, and for nothing else.

The measurable outcome, and it is the whole pillar: `/crew fetch the last 20 issues from the repo,
group them by area, and post a summary` runs FOUR agents that each hit the API, in parallel, today
impossible.

### Pillar 2 — THE CONTRACT: adding an integration is one file and zero Rust
This is the shape the whole goal is bent around, so it is written as a contract rather than a
description. **After the orchestration lands, adding `weather_data`, `google_api`, `meta_api` or the
fortieth integration is: drop one manifest into `~/.config/crew/integrations/` (or the project's
`.crew/integrations/`), `/reload`, done.** No recompile, no release, no core file touched. If a new
integration ever requires editing a Rust file, that is a BUG IN THIS PILLAR and the fix is to move
whatever was hard-coded into the manifest schema.

crew already proves this pattern works twice over. Agent adapters: "to add a fourth agent, write one
more constructor here and push it into `known_adapters` — the broker is untouched"
(`broker/agents.rs:5`). Manifest plugin agents load from `~/.config/crew/agents/` and
`./.crew/agents/`, hot-reload, and BEAT the built-ins on a name collision (`broker/registry.rs:17`).
Skills and `mcp.json` do the same. Tools are the one extension surface that never got this
treatment, and the goal is simply to finish the set.

**Today the contract is broken in three specific places** — each a Rust edit that a new integration
would require:
- `tier::tier_of` is a `match server { "sys" => …, _ => Irreversible }`. A weather tool cannot be
  declared Read without editing Rust, so it prompts for approval on every forecast.
- `credentials::VARS` is a fixed six-entry const of LLM provider keys. A Google client secret has
  nowhere to live without editing Rust.
- `mcp/client.rs` launches `Command::new(cfg.command)` — stdio only. A hosted HTTP/SSE server
  (which is what Google and Meta publish) cannot be declared at all.

**So the manifest carries exactly those three things plus identity**, and the loader is the only
code that ever grows: `name`; `transport` (`stdio` command/args, or `http`/`sse` url — the second
transport is built ONCE, behind the existing `McpClient` seam, and every hosted integration after it
is free); `auth` (`none` | `api_key` naming a namespaced credential slot | `oauth2` with authorize
and token URLs, scopes and a client id); and `tools`, each with its TIER. Tiers ship WITH the
integration and are covered by the same exhaustiveness test that already stops a fifth `sys` tool
shipping unclassified — `weather:current` is Read, `calendar:create_event` is Reversible (we can
delete it), `gmail:send` is Irreversible. Anything a manifest fails to classify stays Irreversible.

**The one piece that is genuinely build-once-then-config: OAuth.** Weather is an API key in a header
— an afternoon. Google and Meta are OAuth 2 with user consent, SCOPES, refresh rotation and
per-account storage, and `crew-hive/src/oauth.rs` is OpenRouter SIGN-IN that does not generalise to
them. Treating those two as the same size of work is the single likeliest way this goal is
mis-estimated. Build the generic flow once — authorize, callback, refresh, revoke, multi-account —
so that `google_api` and `meta_api` and everything after them are a manifest row and a browser
consent, not a code change.

**And what NOT to build: not a native Rust tool per service.** Each would be crew's own HTTP client,
own schema, own maintenance burden the next time Meta moves an endpoint. Integrations are MCP
servers — the ecosystem writes and maintains them, `mcp.json` already takes other tools' configs
verbatim, and the manifest is the thin declarative layer above that which teaches crew the auth and
the tiers it cannot infer. The only native tools crew owns are the ones no server provides: the
`sys` surface, and an HTTP tool if and only if `sys:run curl` proves insufficient in practice.

**One correction to the record while here:** agents CAN already execute commands. `sys:run` is a
real `/bin/sh -c` with a 120 s deadline and drained, capped pipes (`broker/sysrun.rs`), classified
Irreversible because "a shell command is a blank cheque", and an API call is reachable today as
`@tool sys:run {"cmd": "curl …"}`. It is UNIX-ONLY — Windows gets an error string and keeps the
other three sys tools — which is its own gap now that crew ships a Windows binary.

### Pillar 3 — the main agent chooses, and at forty tools that is a RETRIEVAL problem
"Keep adding tools and the agent decides" has a scaling wall in it, and it is worth naming with
numbers before it is hit rather than after. `SessionTools::hint` (`broker/session.rs:206`) builds the
tool list by concatenating the `sys` tools with EVERY tool on EVERY connected MCP server, and
`toolcall::augment` prepends that whole list to the task body ON EVERY HOP. It is O(all tools) per
hop, per agent, forever.

With four `sys` tools that is free. Google Workspace's MCP server alone exposes on the order of
FIFTY tools; add Meta, a weather server, GitHub and a calendar and the flat list is several hundred
lines and many thousands of tokens injected into every single hop of every agent in a four-wide
swarm. The token bill is the lesser problem. The real one is that selection ACCURACY COLLAPSES first
— a model shown two hundred similarly-worded one-line descriptions (clipped to 100 characters by
`hint_for`, so the disambiguating half of the sentence is gone) picks the wrong one, and the failure
looks like a bad model rather than a bad prompt.

So tool selection becomes a real stage instead of a string concatenation:
- **Two-tier advertisement.** The agent sees INTEGRATIONS, not tools: one line per manifest
  ("`google` — gmail, calendar, drive: read and send mail, manage events, search files"). It names
  the integration it needs and only THAT server's tools expand into the next turn. Constant cost in
  the common case; the manifest already has the natural place to write that summary.
- **Descriptions stop being clipped.** Once only one server's tools are in the prompt, they arrive
  with their full `inputSchema` — which `McpTool` (`mcp/mod.rs:22`) currently throws away — so the
  model chooses on the argument shape, not on half a sentence.
- **Retrieval when even one server is too big.** For a fifty-tool server, rank its tools against the
  task text (lexical first — it is honest, debuggable and needs no embedding model; embeddings only
  if measurement demands) and show the top handful plus an explicit "ask for the full list".
- **The PLANNER learns that tools exist.** `PLANNER_SYSTEM` (`planner/mod.rs:138`) decomposes a goal
  into specialists and has NO notion of capability — it cannot know a task is reachable, so it
  cannot route a task to the agent holding the right integration. It gets the integration summary
  list too, and may name the integrations a task will need. This is what makes the swarm, not just
  the relay, able to pick.
- **A wrong choice must be cheap.** `MAX_TOOL_ROUNDS = 4` per hop was sized for a world with four
  tools; a wrong first pick now costs a quarter of the budget. The cap becomes a budget over the
  RUN, and a tool error returns a structured, actionable message (unknown tool → the near matches),
  not prose.

The test that proves this pillar, and it should be written before the catalogue grows: with forty
integrations installed, the prompt an agent sees does not grow with the fortieth, and a fixed suite
of task→tool expectations still resolves correctly.

### Pillar 4 — the bridge stops being a test fixture, and the wire grows what a graph engine needs
`RemoteFactory` gets wired into the running app: a `CrewConfig` key and a `/crew engine
native|sidecar` switch; a spawner (`childproc::no_console_window` is already the cross-platform
seam) that starts the sidecar with the session's cwd and environment; a HANDSHAKE frame where the
worker declares its protocol version and capabilities; per-task timeouts; and a health model where
a sidecar that dies mid-graph FAILS ITS TASKS AND FALLS BACK TO NATIVE rather than hanging the
scheduler. The doctrine from the file-viewer goal is the doctrine here: a missing tool DEGRADES A
RUNG, it never errors. No Python on the machine must read as "sidecar unavailable" in `/doctor`,
not as a broken `/crew`.

**The wire itself is the hard half.** Today's protocol is one request and one reply. Four additions, each of which is a way this ships
half-done if skipped:

**Tool calls flow BACKWARD.** The sidecar must be able to emit a `tool_call` frame and receive a
`tool_result` — because the MCP host, the OAuth grants and the API keys live in the broker and MUST
NOT be copied into a Python process. A LangGraph node calling crew's tools over the wire is the
entire value; a sidecar with its own duplicated tool stack is a second application wearing crew's
name.

**Chunks flow backward too.** `RemoteReply` arrives whole, so a sidecar task shows nothing until it
finishes, while native agents stream (`Provider::complete_streaming`, `HiveEvent::OutputChunk`).
The pane would visibly regress the moment you switched engines.

**State and checkpoints.** The reason to want LangGraph at all is durable graph state and resume;
the wire needs a thread/checkpoint id so a crashed run resumes rather than restarts.

**Interrupts.** LangGraph's human-in-the-loop interrupt maps ONTO CREW'S APPROVAL GATE
(`broker/approval.rs`) — not onto a second confirmation UI. One gate, one ledger, whichever engine
asked.

### Pillar 5 — `crew-langgraph`, the reference sidecar (small, probed, never required)
A Python package, versioned with crew, that implements the wire as a LangGraph app: crew's tools
projected as LangChain `BaseTool`s that round-trip over the `tool_call` frames, crew's model
selection honoured, crew's telemetry emitted as events the swarm pane already knows how to draw.
Shipped as an OPTIONAL extra (`pipx install crew-langgraph` or a `uv` one-liner), probed the way
`pdftotext` is probed, absent by default. `cargo test` must never need a virtualenv; the Rust side
is tested entirely against `LoopbackTransport` and a recorded frame corpus. If the sidecar cannot
be developed without CI growing a Python job, that is the signal it is too big.

### Pillar 6 — cycles, or the honest reason to leave the DAG
`TaskGraph` rejects cycles outright (`GraphError::Cycle`, `graph/mod.rs:35`), which is correct for a
plan-then-execute scheduler and fatal for the shapes people actually want: reflect-and-retry,
critic loops, "keep calling the API until the cursor is exhausted", ReAct. THIS is what LangGraph is
for, and it is the one capability crew cannot reach by adding a field. Decide by measurement, not
taste: after Pillars 1–4, write the three loop-shaped jobs both ways — bounded cycles in the native
scheduler versus a LangGraph graph over the bridge — and let the cost, the latency and the code
size pick. Recording the losing branch is part of the deliverable.

### Non-negotiables
- **The zero-dependency install survives.** Default crew stays one binary. A user who never installs
  Python must not be able to tell this goal happened.
- **A sidecar is an executor, not a peer.** The blackboard, the budget governor
  (`govern::budget_governor`), the fleet telemetry and the event bus stay crew's. Two orchestrators
  is how this rots into a fork.
- **The `is_pty` doctrine holds across the wire.** `AgentKind::is_pty` documents that a graph
  derived from untrusted input must never carry a `Pty` agent, and `planner::parse_plan` forces
  model-derived tasks to `Api`. A sidecar-produced plan is EXACTLY that untrusted input. A sidecar
  cannot spawn a process, cannot raise a tool's tier, and cannot approve its own interrupt.
- **Every tool call, either engine, passes one gate and lands in one ledger** — the JARVIS goal's
  read/reversible/irreversible tiers (`docs/superpowers/goals/2026-08-23-jarvis-personal-assistant.md`).
  Parallel agents calling real APIs is precisely when a consequence model stops being theoretical.
- **No integration may require a recompile.** The manifest loader is allowed to grow; `tier.rs`,
  `credentials.rs` and `mcp/client.rs` are not allowed to grow PER INTEGRATION. A pull request that
  adds a service by editing a match arm has missed the point of the goal.
- **The prompt does not grow with the catalogue.** Advertisement is per-integration, expansion is
  on demand. A change that puts every tool back into every hop is a regression even if it improves
  a benchmark.
- **A secret never leaves the broker.** Service credentials go into an MCP server's launch `env` or
  an outbound header and nowhere else — never into `CrewConfig`, never over the sidecar wire, never
  into a prompt, never into a hop the pane renders. The redacting `Debug` on `credentials::Store`
  exists because someone will `dbg!` it; the new section keeps it.
- **Lowering a tier is an explicit, reviewed act.** Anything not in the table stays Irreversible.
  A tool may never be classified by asking the model what it does.
- **The winit blocking rule.** Sidecar spawn, handshake and every frame stay off the main thread.
- **Pillar 1 ships and is used before Pillar 5 starts.** If the swarm cannot call a tool natively,
  a LangGraph sidecar has nothing to be better than.

### Done looks like
`/crew tomorrow's forecast for the three cities on my calendar, then draft the note` runs a
four-agent swarm where each agent authenticates against a real API — weather without a prompt,
calendar read without a prompt, the send asking once — calls it several times and feeds the results
into the next task, visible in the pane as it happens, billed against one budget, every call in the
ledger. Then the fortieth integration is added by someone who has never seen the source: one
manifest file, `/reload`, and the agent starts choosing it when it is the right tool — with no
prompt bigger than it was at ten. `/crew engine sidecar` runs the same goal through a LangGraph
graph with a reflection cycle, using crew's tools and crew's credentials, resumable after a crash —
and on a machine with no Python, `/doctor` says so in one line and `/crew` behaves exactly as it
does today.
