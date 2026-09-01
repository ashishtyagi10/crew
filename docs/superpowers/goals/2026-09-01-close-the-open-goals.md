# Goal — close the open goals: the clock, the voice, the fortieth integration, and the sidecar

**Status (2026-09-01): Pillar 1 SHIPPED (v0.20.1–v0.20.2), Pillar 5 items 2 and 4 CLOSED
(v0.20.3).** Crew has a clock: standing intents that survive a restart, fire once, report what
they missed, and run with a trigger's authority rather than a person's — set from the terminal
with `crew daemon at`, or from a phone with "remind me tomorrow 9am to …". The document window's
last mile (Cmd+K on a URL, Tab through a table, the minimal-diff save asserted over `docs/CREW.md`)
landed with it, and the command-diet number was amended in the goal that owns it, with the reason.
**Pillar 3 SHIPPED v0.20.4–v0.20.5**: an HTTP API is one manifest file with no Rust
(`~/.config/crew/integrations/`, project-overridable, hot), secrets are env-named by construction,
a tool is irreversible unless its manifest says otherwise, and `/doctor` + `/reload` report what
is loaded. Retrieval landed with it: above 24 tools the task chooses which
are named, crew's own are never dropped, what was left out is counted, and `sys:find_tools`
searches the whole catalog so nothing is unreachable. **v0.20.6** closed Pillar 5 item 3 (`crew ask` reaches every window) and done-means 2 (the
morning briefing, which needed no code path of its own — the clock plus an integration is one).
Open: Pillar 2 (voice), Pillar 4 (the sidecar), and Pillar 5 items 1 and 5.

**Set:** 2026-09-01 by the user — *"write a goal to complete any pending plan"* — after an audit of
every goal, plan and spec under `docs/superpowers/` against the tree at v0.20.0.

**The audit's first finding is that far less is open than the corpus suggested.** There were
fourteen goals and sixty-two implementation plans on disk. Every plan describes work that shipped:
the plan-per-feature SDD workflow ended on 2026-08-01 (`2026-08-01-active-agents-footer` is the
last one written), and everything since has been goal-driven and loop-driven. The plans were
deleted in the same pass that wrote this document — git keeps them, `CHANGELOG.md` says what each
one became, and a directory of sixty-two stale instruction sets is a place to look for open work
that has none. **Nine of the fourteen goals are delivered in full.** What is genuinely open is five
threads, and four of them are one sentence:

**crew can reach the world when you ask it to, from one keyboard — and cannot yet do it on its own
clock, from your voice, with an integration it learned from a file, or through an engine it did not
compile in.**

That sentence is this goal. The fifth thread is debt: four small, specific, unglamorous items that
three goals are each carrying a copy of.

### The audit

| goal | state at v0.20.0 |
|---|---|
| `2026-07-28` file viewer + markdown editing | Phase 1 shipped 2026-07-31; Phase 2 taken up and shipped by the 2026-08-30 goal. **Closed.** |
| `2026-08-01` OAuth subscriptions | Shipped v0.12.0–0.12.1 (`/login`, `/logout`, grant outranks key). **Closed.** |
| `2026-08-01` smith autonomy / command diet | Shipped: keyword judges gone (`is_critic`/`is_writer`/`pick_by_role` have zero hits), `roundloop::MAX_ROUNDS` is a backstop, `broker/compact.rs` summarizes, `sched/replan.rs` re-plans mid-run. **One number unmet — Pillar 5.** |
| `2026-08-04` CRT holographic overhaul | Shipped v0.12.2–0.12.4; the luminous sheet retired v0.13.5. **Closed.** |
| `2026-08-04` CRT shows up by default | Shipped v0.12.6 (stale config pins healed). **Closed.** |
| `2026-08-06` theme follows the OS | Shipped v0.14.2–0.14.4. **Live verify outstanding — Pillar 5.** |
| `2026-08-09` todo pane | Shipped v0.15.0. **Live verify outstanding — Pillar 5.** |
| `2026-08-10` todo keyboard navigation | Shipped v0.16.4–0.16.6 (`PageUp`/`PageDown`/`Home`/`End` in `todopane/keys.rs`, composer cursor). **Closed.** |
| `2026-08-10` todo project colours | Shipped v0.16.3 (`crew-theme/src/tagcolor.rs`). **Closed.** |
| `2026-08-12` todo done history | Shipped v0.17.0. **Live verify outstanding — Pillar 5.** |
| `2026-08-22` one colour system | Phase 1 shipped (`ramp.rs`), Phase 2 shipped (`ansi.rs`), the open question answered by cutting 24 themes to 12. **Phase 3 (restraint, measured) open — Pillar 5.** |
| `2026-08-23` JARVIS assistant | Pillars 1–2 shipped: `crew daemon install` (launchd/systemd, `daemon/service.rs`), the action gate + append-only ledger + `/tools`, and Telegram as the second `Channel`. **Pillars 3–5 open — Pillars 1, 2 and 4 below.** |
| `2026-08-30` markdown editor in its own window | Shipped v0.19.84–0.19.95: second window, cursor in the render, typing, undo, selection, cut/paste, save. **Last mile open — Pillar 5.** |
| `2026-08-31` agents that reach the world | Pillar 1 shipped (the swarm calls tools, native tool-use, per-session gate). **Pillars 2–6 open — Pillars 3 and 4 below.** |

---

### Pillar 1 — the clock: crew has no notion of "later"

The daemon can hold a conversation and cannot hold an appointment. `crates/crew-app/src/daemon/`
owns sessions, routes a message from a channel to a broker session and reads the reply back
(`daemon/task.rs`) — and nothing anywhere in it fires on time. The `--after` flag in `daemon/cli.rs`
is a poll cursor, not a schedule. The only natural-language time in the whole tree is
`todopane/duedate.rs`, which already parses *"tomorrow 5pm"* into an instant, is already tested, and
already drives a toast; the daemon is simply the only process alive to honour one when the window is
shut.

A standing intent is one record: what to run, when it fires, which channel it answers on, and
whether it repeats. It goes beside the ledger — same directory, same append-only discipline, same
survives-a-restart requirement — because a thing that acts while you are asleep must be auditable
for exactly the reason it is useful. Three rules the implementation does not get to skip: a fire
missed because the machine was asleep **says so** rather than silently running four hours late or
silently vanishing; every scheduled run passes the same gate as a typed one, with no tier promoted
for being unattended; and `crew daemon` can **list and cancel** what is watching, or the feature is a
haunting rather than an assistant.

The morning briefing (`2026-08-23`, done-means 8) is then the first standing intent and not a
special case — if it needs its own code path, the clock was built wrong.

### Pillar 2 — the third channel: voice

`crates/crew-app/src/channel/mod.rs` was written for this and says so in its first line: *"a pane, a
phone and a microphone are the same kind of thing"*. It has two implementations — `loopback` and
`telegram`. The JARVIS goal's done-means 3 asks for three, and for one transcript-level test that
runs the SAME task through all three asserting identical tool calls. Nothing in the tree speaks
audio.

Decisions to take before the first commit, because they are the whole cost of this pillar:
**push-to-talk first, wake word second** (a hotkey is a channel; a wake word is a wake word plus an
always-on microphone plus a consent story, and shipping them together means shipping neither);
**API-backed STT/TTS before on-device**, since the zero-dependency install is a feature people
notice and a model runtime is the largest thing crew would ever ask a user to install — the network
call is gated and ledgered like every other reach outside; **barge-in from the start**, because a
spoken answer you cannot interrupt is worse than a printed one.

### Pillar 3 — the fortieth integration is a file, and the agent can still find it

**SHIPPED v0.20.4 (retrieval) and v0.20.5 (the manifest contract).** `broker/integration/` loads
JSON manifests from `~/.config/crew/integrations/` and `.crew/integrations/`, turns each into a
server of tools on the existing `@tool` surface, fills `{arg}` placeholders from the call, reads
every credential from an environment variable the manifest NAMES rather than holds, and defaults
every tool to irreversible. `broker/toolpick.rs` does the choosing. What follows is the goal as
it was set.

Adding `weather_data`, `google_api` or the fortieth integration must be: drop one manifest into
`~/.config/crew/integrations/` (or a project's `.crew/integrations/`), `/reload`, done. Zero Rust. If
an integration ever needs a source edit, that is a bug in the contract and the fix is to move what
was hard-coded into the manifest schema. crew already proves the pattern three times — agent
manifests, skills, `mcp.json` — and tools are the one extension surface that never got it.

**The selection half SHIPPED v0.20.4** (`broker/toolpick.rs`): a budget of 24, scoring against
the task's words, `sys` never dropped, the count of what was left out in the prompt, and
`sys:find_tools` as the door back to everything. The manifest contract above is what remains.

And the selection problem that arrives with the tenth integration, not the fortieth, as it stood:
`SessionTools::hint` (`crates/crew-plugin/src/broker/session.rs:255`) concatenated the `sys` tools
with every tool on every connected MCP server, and that string is prepended to the task body on
EVERY hop of EVERY agent. Free at four tools. At two hundred — one Google Workspace server is fifty
— it is thousands of tokens per hop per agent in a four-wide swarm, and the token bill is the lesser
half: selection accuracy collapses first when a model is shown two hundred similarly-worded
one-line descriptions. This is retrieval, not concatenation, and the fix has to hold the invariant
that a tool the agent needs is never the one that got filtered out.

### Pillar 4 — the bridge stops being a test fixture

`crew-hive` already contains `wire/` (the `RemoteTask`/`RemoteReply` line protocol and the
`Transport` trait), `worker/` (`serve_stdio`, `LoopbackTransport`) and `remoteagent/`
(`RemoteAgent`, `RemoteFactory`) — built for exactly this and **unreachable from production**:
`RemoteFactory` is constructed in three places and all three are tests. Nothing spawns a sidecar and
no config key selects one.

The wire is also too thin to be worth crossing: `RemoteTask` carries `{agent, task, prompt, model,
deps}` and nothing else — no tools, no credentials, no stream, no state — so a graph engine behind
it can only return a string, which makes it a slower `ApiAgent`. Grow the wire to carry the tool
surface, a gated credential handle, streamed events and resumable state; then `crew-langgraph` as
the reference sidecar: opt-in, probed, never required. On a machine with no Python, `/doctor` says
so in one line and `/crew` behaves exactly as it does today.

### Pillar 5 — the debt that is not a feature

Four items, each small, each currently carried by a goal doc instead of by a commit.

1. **Live verify, written off or run.** Three goals ship "live verify pending" against the same
   macOS-permissions debt (`2026-08-06`, `2026-08-09`, `2026-08-12`). Either the harness gets its
   permissions and the passes run, or the debt is written off in the docs. Carrying it in three
   places is the worst of both: it reads as work outstanding and nobody can tell if it is.
2. ~~**The document window's last mile.**~~ **CLOSED v0.20.3.** `2026-08-30`'s done-list names link editing and
   Tab-through-table-cells; `docwin/event.rs` maps Tab to two spaces and has neither. Its item 4 — a
   save whose diff touches only what was edited, asserted over a REAL repo document — has no test in
   `docwin_tests.rs`, and that assertion is the entire reason the byte-provenance model was chosen.
3. ~~**`crew ask` addresses one window.**~~ **CLOSED v0.20.6.** The ask socket and the federation relay are held by the
   launch canvas, so a second window's panes are unaddressable — stated as the honest cost of the
   smaller shape in `2026-08-30` and never taken back.
4. ~~**The command diet's own number.**~~ **CLOSED 2026-09-01 by amendment** — the reason is in
   `2026-08-01-smith-autonomy-command-diet.md`, done-means 1: what the diet was about is retired,
   and the nine that remain are mechanical verbs with no model in the path. `CONSTRUCTS` (`broker/commands.rs:115`) lists nine; the
   `2026-08-01` goal's done-means 1 says at most eight. Retire one — `/restore` and `/diff` are both
   checkpoint verbs — or amend the number in that goal with the reason. Not both, and not neither.
5. **Phase 3 of the colour system.** `2026-08-22` left one phase open: bloom radius and amplitude,
   the gradient light-ring, the dot lattice and grain each get a defensible number per pool rather
   than a per-theme feel, checked with the screenshot harness that already renders every theme.

### Done means

1. A standing intent registered on one day fires on another, on the channel it was registered from,
   with the app closed in between; `crew daemon` lists and cancels what is watching; a fire missed
   to sleep is reported as missed; every scheduled run appears in the ledger with the same tiering
   as a typed one.
2. ~~The morning briefing is a standing intent with no code path of its own~~ — **MET v0.20.6**,
   and it needed nothing but the clock and the integrations: `crew daemon at "tomorrow 7am brief
   me: …" --every daily` fires unbidden, as a trigger, on the channel it was set from.
3. `Channel` has three implementations — pane, Telegram, voice — and one transcript-level test runs
   the same task through all three, asserting identical tool calls and equivalent replies. Spoken:
   push-to-talk, a task, a spoken answer, and barge-in that stops it mid-sentence.
4. A new integration is one manifest file plus `/reload`, proven by a test that adds one from a
   fixture directory with no recompilation and no core file touched, and by a second that fails if
   any integration-shaped capability requires a Rust edit.
5. At two hundred registered tools, the per-hop prompt is no larger than it is at ten, and a
   retrieval test shows the right tool still selected for each of a fixed set of asks.
6. `RemoteFactory` is reachable from a real run: `/crew engine sidecar` executes a goal through an
   out-of-process engine using crew's tools and crew's credentials, streamed into the pane and
   resumable after a kill; on a machine with no Python `/doctor` says so in one line and nothing
   else changes.
7. Every Pillar 5 item is closed in the tree or explicitly written off in the goal doc that owns it,
   in the same merge that changes its state.
8. No goal document under `docs/superpowers/goals/` claims a status the tree contradicts —
   enforced by reading them at the end of each iteration, the way `changelog_covers_the_current_
   version` enforces the changelog.

### Non-negotiables

The consent rule stands: no release, update or app launch installs a background service — `crew
daemon install` is the whole consent (`daemon/service.rs:3`), and the same applies to a microphone
and to anything that reaches the network on a schedule. Every outward call keeps passing the gate
and appending to the ledger, with irreversible calls gated on approval, and no tier is relaxed
because the request arrived from a phone, a clock or a voice. The zero-dependency install stays: no
Python, no node, no model runtime is ever required for crew to work — every one of them is opt-in
and probed. The planner still can never select a process-executing agent, and token, cost and hop
ceilings stay as hard backstops. This lands as SMALL FILES ACROSS ITERATIONS, each shippable and
released on its own, never one god commit — and the mock provider path keeps working so the tests
need no key.

### Explicitly not this goal

A second orchestration spine (LangGraph is an engine behind a bridge, never crew's centre); a mobile
app of our own; multi-user or hosted-for-others operation; on-device speech models; and any autonomy
tier above the one the JARVIS goal chose — "act then report" on irreversible actions stays off until
the ledger and the approvals have earned it.
