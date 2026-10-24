# Goal — one agent harness for crew: the relay and the swarm run the same turn loop

**Status: PHASES 1–2 SHIPPED (v0.28.99, v0.29.0).** Phases 3–4 open.

**Set:** 2026-10-24 by the user. "Do you know how to build harness ?" → "Yes I am talking about
Agent Harness." → offered a standalone harness, one harness for crew, or a teaching one: "Option 2,
one harness for crew" → "yes, write the doc and start phase 1" → "yes, start phase 2".

## Where it stands today

crew runs an agent turn (a model call, and while the reply asks for a tool, the tool run and the
model called again with the result) in TWO places, written twice:

- **The swarm** (`/crew`, the intent router's `swarm` shape, `/plan` once approved): crew-hive's
  `ApiAgent` (`crates/crew-hive/src/apiagent/mod.rs`), which since phase 1 runs each task as one
  `harness::run` (`crates/crew-hive/src/harness/mod.rs`). It picks a loop once: native tool-use
  (`harness/native.rs`) when the tool surface has schemas and the provider speaks tools, else the
  text `@tool` loop (`harness/text.rs`). Before phase 1 both lived in `apiagent/`, the text loop
  inline in `ApiAgent::attempt`.
- **The relay** (`@agent`, the `reply` / `loop` / `goal` shapes, keyless starters): the broker's
  `Broker::run` (`crates/crew-plugin/src/broker/engine.rs`) routes hops between agents with
  `@next`/`@done`. Since phase 2 an API agent's hop with tools on runs whole on the harness
  (`broker/apiturn.rs`), native tool-use included; any other agent's hop is a dial, and
  `run_tools` (`broker/toolcall.rs`) runs the text `@tool` loop over its reply. Before phase 2
  every hop went through `run_tools`, and API agents never went native: `ApiAdapter` built every
  request with `..Default::default()`, so `tools` and `turns` were always empty.

Already shared, from the 2026-08-31 goal (`2026-08-31-agents-that-reach-the-world.md`, Pillar 1):
the `Tools` trait and its parser, `Seen`, `Exchanges`, the todo checklist, `MAX_TOOL_ROUNDS`, and
the approval gate, which sits inside `SessionTools::call` (`broker/session.rs:497`), so a relay
call and a swarm call are asked about the same way.

Built twice, and drifted (the table as it stood before phase 2; since then the relay's column is
the CLI agents' alone):

| piece | swarm (crew-hive) | relay (broker) |
|---|---|---|
| the loop | `harness/text.rs`, `harness/native.rs` | `toolcall.rs:113` `run_tools` |
| one reply's batch | `pending.rs` (`spawn_blocking`), at most 8 (`native.rs:32`) | `toolbatch.rs` (`thread::scope`), at most 4 (`toolbatch.rs:21`) |
| the follow-up prompt | `toolloop.rs` | `toolfull.rs`, `toolround.rs` |
| a long result | `toolloop.rs` `RESULT_CAP`, kept the head (both ends since phase 2) | `toolclip.rs`, keeps the last line |
| context overflow | `overflow.rs` (cut, retry once) | `toolfull.rs` (follow-ups only) |
| budget spent | refused the batch, then a last-word call (`lastword.rs`); since phase 2 runs it, then the last word | runs the batch, nothing reads it (`toolcall.rs`, `budget_answer`) |
| what a round is | text: one per CALL (one per reply since phase 2); native: one per turn | one per ROUND TRIP |
| steering | `steered.rs` | `steer.rs` |
| stop | `TurnContext::cancel` | `cancelscope` — the same `Arc<AtomicBool>` type |

Only the swarm has: native tool-use, bad-argument refusals (`badargs.rs`), the last word. Only the
relay has: the advisor that reads a failed check (`advise.rs`), the cut-off continuation
(`cutoff.rs`), `never_ran` for a call a reply narrated but never made (`toolround.rs`), and
cross-hop summary compaction (`compact.rs` — that one belongs to hop routing and stays there).

## The shape

The relay is two layers, and only the second is a copy: **hop routing** (`Broker::run`: who speaks
next, peers, the hop cap, the repair re-ask, compaction between hops) and **the tool loop inside a
hop** (`run_tools`). So one harness means ONE TURN LOOP in crew-hive that both call; hop routing
stays in the broker on top of it.

```
today                                            after
relay hop  → Adapter::call → run_tools (text)    relay hop  ─┐
swarm task → ApiAgent::attempt (own loop)        swarm task ─┴→ crew_hive::harness::run
```

The loop's three seams, which are exactly where the two engines differ:

- **where its events go** — the swarm's `EventBus`, or the pane's `HopStream`: a trait, `Events`;
- **the stop it honours** — both are already an `Arc<AtomicBool>`;
- **the rounds it may draw** — both fit a `ToolBudget` (the swarm's pool over the run; the relay's
  rounds per hop is a pool of one turn).

## Phases — one release each

1. **Extract, no behaviour change.** `crew_hive::harness` takes the loop: `TurnContext` (agent id,
   events, budget, stop), `TurnSpec` (what the tools are picked for, the prompt, system, model,
   tier, token ceiling), `run`, and `Ended` (answered / failed / stopped). The loop's modules move
   from `apiagent/` to `harness/`; `ApiAgent` keeps what is the swarm's own — the dependency
   results in the prompt (`context.rs`), the planner's system prompt and preamble, a checklist per
   task — and maps `Ended` to the scheduler's `Attempt`. The tests that drive `ApiAgent` end to end
   are the proof: they pass with no change to what they assert.
2. **The relay's API agents run on it.** A Provider-backed hop calls `harness::run` (blocking on
   it as `ApiAdapter` already blocks on its provider), with an `Events` that feeds the hop's
   `HopStream` and `RunStats`. The relay gets native tool-use, schemas, bad-argument refusals and
   the last word — the "NOT DONE" of the 2026-08-31 goal. Hop routing is untouched. **Done
   (v0.29.0)** — and the advisor came with it, through a hook (`harness::Advice`), since it was
   built for exactly these agents; phase 4 moves its body down.
3. **The CLI agents run on it.** `claude`, `codex`, `opencode` and manifest agents are wrapped as a
   text-only model (a `Provider` that says it has no tools), so the harness's text loop runs any
   `@tool` line they print, as `run_tools` does today; their own internal tools stay theirs. Then
   `run_tools`, `toolround`, `toolbatch`, `toolfull`, `toolclip` and `toolhive` go (~800 lines),
   their tests ported or retired with the behaviour.
4. **The relay's extras move down**, so the swarm has them too: the advisor, the cut-off
   continuation, `never_ran`.

## Decisions (where the two disagree)

All four in the harness since phase 2, so the swarm and the relay's API agents share them; the
CLI agents' `run_tools` keeps its own until phase 3 removes it.

- **A round is a round trip**, as the relay counts it: counted per call, a rename's six edits
  spent a solo turn's budget before its last file (live, 2026-10-20). Changes the swarm's text
  path. Phase 2 or later, never phase 1.
- **Budget spent: run the batch, then the last word.** The relay runs the last batch (22 of 22
  budget stops asked for one, 2026-10-23) but nothing reads it; the swarm reads but refuses the
  batch. Both halves.
- **At most 8 calls per reply** (native already), not the relay's 4. Every call still meets the
  approval gate.
- **A long result keeps its head and its end** (`clip_middle`, already in crew-hive), not two
  6,000-char rules that keep opposite ends; the whole output is still spilled to `.crew/out/`.
- **Out of scope:** the app's own `/goal` swarm (`crew-app/src/swarm/*`, `swarmpane.rs`), a third
  host with no tools. Routing it through the broker is a small follow-up once the harness exists.

## Risks

- **Prompt-byte tests.** Many relay tests pin prompt wording; phase 2 changes what API agents are
  sent. Port them; retire only what tests behaviour that is gone.
- **The relay is synchronous.** The harness runs on the hop's current-thread runtime; tool calls go
  to the blocking pool (`pending.rs`), so an approval prompt waiting on the person never freezes
  the runtime.
- **Live verification.** `osascript` keystrokes are denied on this Mac, so live checks drive the
  broker binary over its stdio protocol (`crew --broker-plugin < in.jsonl`, `zsh -lc` for the
  keys), as the 2026-08-31 work was verified. `CREW_INTENT=0` forces the swarm; the `reply` shape
  reaches the relay.

## Log

- 2026-10-24 — goal written; phase 1 started on branch `one-harness`.
- 2026-10-24 — **phase 1 shipped (v0.28.99).** `crew_hive::harness` holds the loop: `TurnContext`
  (agent, `Events`, `ToolBudget`, stop), `TurnSpec`, `Ended`, `run`, the text loop in
  `harness/text.rs`. Fifteen loop modules and their tests moved out of `apiagent/` with `git mv`;
  `ApiAgent` builds the spec and maps `Ended` to `Attempt` (`attempt_of`). The draw, stop and
  race moved from `AgentContext` to `TurnContext`; `AgentContext::turn` makes one. All 99 of the
  old `apiagent` tests compile and run (61 under `harness::`, 38 under `apiagent::`); their edits
  are import lines only. Gate: crew-hive 589, crew-plugin 1,573 + integration, crew-app bin 4,522.
  LIVE (broker over stdio, `CREW_PROVIDER=dashscope CREW_INTENT=0`, qwen-max): a two-task swarm,
  one worker called `sys:run {"cmd":"uname -sr"}` natively and answered from `Darwin 27.2.0`.
  The `@tool` text path is unit tests only: on this Mac every plain message is led by the
  signed-in Claude Code (auth resolves `Delegated`, `auth/mod.rs` `starter`), which comes before
  the intent router, and `/plan` is retired, so no live route reaches a swarm on a provider
  without tool-use.
- Finding for phases 2–3: because of that delegation, the person's everyday plain message runs on
  the relay's `claude` CLI adapter, whose real tools are Claude Code's own. Phase 3 changes only
  the `@tool` lines it prints; phase 2 (API agents) matters for specialists and keyless setups.
- 2026-10-24 — phase 2 started on branch `harness-relay` ("yes, start phase 2").
- 2026-10-24 — **phase 2 shipped (v0.29.0).** `Adapter::engine` exposes an API agent's provider,
  model, persona and runtime; `Broker::run` hands such a hop, with tools on, to `api_turn`
  (`broker/apiturn.rs`), which runs `harness::run` on the adapter's runtime. The broker's parts go
  in as seams: `Pane` (the harness's events into the hop's `HopStream`: text, reasoning, the tool
  lane `toolhive` used, a checklist as the plan once accepted), `Bounded` (the agent's provider
  with the broker's timeout, the `cutoff` continuation — now skipped for a reply that made tool
  calls — and its usage into `RunStats`), `Advised` (the advisor, read each round through the new
  `harness::Advice`), `Steers` with the relay's closing line (`Steers::with_tail`) and
  `ToolBudget::turn(tool_rounds)`. `TurnSpec::hinted` keeps the `@tool` hint where the frame puts
  it, in TASK above HOW TO REPLY. Ended maps to the hop as `run_tools` ended: an error card when
  nothing ran, `failed_answer` / `stopped_answer` after a tool. The four decisions land in the
  harness, so the swarm changes too: one round per reply on the text loop, at most 8 calls a
  reply, the batch past the budget runs before the last word, results cut from the middle; the
  swarm's "pool ran dry" note counts rounds and rides the empty `ToolBudget` reading alone.
  `todo::finished` adds its line once, since a relay answer is finished twice. CLI agents, which
  have no engine, still dial and go through `run_tools`. Gate: crew-hive 593, crew-plugin 1,584 +
  integration. LIVE (broker over stdio, `CREW_PROVIDER=dashscope`, `@system-administrator`, a
  stored specialist): native `sys:run {"cmd":"uname -r"}` answered from `27.2.0`, 2 exchanges with
  real usage; a two-round turn ran `sys:glob` then `sys:read_file` with schema arguments
  (`line`, `lines`) and quoted the line. The text loop live: `CREW_PROVIDER=claude-code` puts
  specialists on `claude -p`, which has no tool-use, but Claude answered through the person's own
  Node REPL MCP server instead of an `@tool` line — `ClaudeCliProvider` passes `--tools ""`,
  which leaves MCP servers on, so those calls bypass crew's approval gate. The first prompt was
  byte for byte the old relay's, so this predates phase 2; reported, not fixed here.
