# Goal — crew becomes JARVIS: a resident assistant you talk to, not an app you open

**Status: PILLARS 1–3 SHIPPED, 4–5 OPEN.** `crewd` exists and is opt-in
(`crew daemon install` writes a launchd agent or a systemd user unit — `crew-app/src/daemon/`),
every tool call passes one gate and appends to an append-only ledger that `/tools` reads, and
Telegram is the second `Channel` beside the pane. The clock shipped v0.20.1–v0.20.2 (standing
intents, `crew daemon at`, `/watching`) and the morning briefing is one of them (v0.20.6). The
voice channel and the integration catalog are carried in `2026-09-01-close-the-open-goals.md`.

**Set:** 2026-08-23 by the user. "I would like crew to be the real personal assistant — you give it
a task and it handles it without any issue, should have integration channel with everything that is
needed in everyday life." Asked to clarify: **it should work like JARVIS.**

Crew already has a brain. `crates/crew-plugin/src/broker/` is a genuine agent runtime — skills,
agent manifests, an MCP host that hot-reloads (`mcp/mod.rs`), a tool relay, standing memory
(`broker/memory.rs`), and crew-hive's parallel DAG scheduler behind it. What it does not have is
any of the three properties that separate an assistant from an application:

1. **It does not outlive the window.** The broker is spawned as a CHILD OF THE GUI — `crew_broker_cmd()`
   (`crew-app/src/chatspawn.rs:166`) resolves `current_exe()` and the pane runs it with
   `--broker-plugin`. Close crew and the assistant ceases to exist. JARVIS is resident; he is not
   launched.
2. **Nothing can wake it.** There is no cron, no timer, no watch, no webhook anywhere in the tree.
   `broker/tick.rs` is token-rate pacing, not a clock. Crew acts only in the instant you type, which
   means it can never be the one who noticed.
3. **There is exactly one way in: your keyboard, at that machine.** No inbound channel, no voice.
   The most capable agent in the world is useless when you are in the car.

THE GOAL, in one line: crew stops being a window you open and becomes a RESIDENT you address — from
a pane, from your phone, or by speaking into the room — that holds tasks over time, acts on the
real world through everyday integrations, and is trusted to do so because every action passes one
gate and lands in one ledger.

**Decisions taken at goal-set time** (they shape every pillar): first channel is **Telegram**;
autonomy is **tiered** (read + reversible run free, irreversible confirm first); the daemon runs on
**this Mac now but is written location-agnostic** so a VPS is later config, not a rewrite; and
**voice is early — it IS the JARVIS feel**, not a phase-5 garnish.

### Pillar 1 — `crewd`, the resident (and the one gate every action passes)
The broker is promoted from GUI child to a user-level service — launchd on macOS, systemd user unit
on Linux, a Windows service — owning sessions, memory, swarms, and credentials. crew-app demotes to
*a* client that attaches and detaches over the existing IPC (the named-pipe/Unix-socket transport
built for the Windows release is already the right seam). Closing the window must become a
non-event: reattach and the swarm is still running, the todo still pending, the conversation intact.
The daemon addresses nothing by hostname and holds no assumption that a display exists.

Because the daemon is the only thing that EXECUTES, it is also the only place a consequence model
can be honest, so the gate lives here rather than in any channel: every tool call is classified
**read** / **reversible** / **irreversible**, and the third tier asks — on whatever channel the
request arrived from — before it fires. Sending mail is not `cargo test`. Every attempt, its tier,
its approver and its outcome append to a durable ledger; `activity.log` is the seed but NOT the
thing (`crew-app/src/activitylog.rs` truncates on every process start and is skipped under test —
correct for a session log, disqualifying for an audit trail).

### Pillar 2 — channels: one trait, three faces
A `Channel` is anything through which a request arrives and a reply leaves. Written once, it makes
the GUI pane, the phone and the microphone the same kind of thing — which is why voice-early costs
little: the mic is not a special subsystem, it is the third implementation of an interface the
Telegram work already forced into existence.

- **Pane** — the existing chat pane, retrofitted onto the trait so it stops being the privileged path.
- **Telegram** — Bot API: no number scraping, no fragile bridge, and crew becomes reachable from
  your pocket. This is the step where crew stops being a terminal app.
- **Voice** — wake word, STT in, TTS out, barge-in. Speaking to the room, with the daemon answering
  aloud, is the moment it reads as JARVIS rather than as a very good CLI.

The addressing model is already designed: the sentinel envelope and resolver
(`docs/vision/sentinel-network.md`) exist precisely to let a question find its answer across a
boundary. A Telegram thread and a microphone are two more addresses a question can arrive from and
a verdict can return to — the resolver widens, the engine does not change.

### Pillar 3 — the clock: crew gets to be the one who noticed
Triggers wake the daemon without you: wall-clock and recurring ("every weekday at 7"), natural-
language dues (todopane already parses these — `todopane/duedate.rs`), file and folder watches,
inbound channel messages, and webhooks. A trigger builds a task graph, runs it on crew-hive, and
reports through the channel you are actually on. **Standing intents** are the durable form: "tell me
if the deploy breaks", "watch that flight", "if the invoice hasn't landed by Friday, chase it" —
registered once, evaluated forever, with a visible list of what crew is currently watching on your
behalf and a one-word way to cancel any of them.

### Pillar 4 — the hands: everyday-life integrations, mostly not written by us
"Integration with everything needed in everyday life" is calendar, mail, contacts, messages, files,
money, home, travel, shopping, health. Nearly none of that should be a hand-rolled API client in
this repo: `McpHost` already discovers, connects and hot-reloads servers declared in `mcp.json`, so
the integration surface is a CURATED CATALOG plus first-run auth, not a mountain of Rust. What crew
owns is the part MCP cannot give it — the catalog itself, one-command connect with credentials held
by the daemon, per-integration tiering of which calls are irreversible, and a health view that says
what is connected, what expired, and what needs re-auth before you find out mid-task.

### Pillar 5 — it speaks first
Everything above still waits to be addressed. The JARVIS signature is INITIATIVE: a morning
briefing assembled from calendar, mail and the day's todos; interruption when a standing intent
fires; a nudge when something you asked for two weeks ago just became possible. This pillar is last
not because it matters least but because a proactive assistant built on an unreliable one is a
machine for generating false alarms.

### Done means
1. `crewd` runs as a launchd user service; `crew` launched with no daemon starts one and attaches,
   and quitting the GUI leaves a running swarm running — proven by a test that kills the app
   mid-swarm, relaunches, and asserts the same swarm id still streaming.
2. No tool call reaches the outside world except through the daemon's gate. A test enumerates every
   registered tool and fails on any that is unclassified; irreversible calls block on an approval
   whose grant is recorded with tier, channel, approver and outcome in an append-only ledger that
   survives process restart.
3. `Channel` has at least three implementations (pane, Telegram, voice) and one transcript-level
   test runs the SAME task through all three, asserting identical tool calls and equivalent replies.
4. From a phone, with the crew window closed: a message to the Telegram bot schedules a task, and
   its result arrives back in that thread without the GUI ever being opened.
5. Spoken aloud with the app in the background: wake word, a task, and a spoken answer — with
   barge-in interrupting mid-sentence.
6. At least six everyday integrations connect from the catalog in one command each (calendar, mail,
   contacts, files/drive, home, money), each with its irreversible calls declared, and `/doctor`
   reports connected / expired / needs-auth per integration.
7. A standing intent registered on Monday fires on Thursday, on the right channel, with the app
   having been closed in between — and `crew` can list and cancel what it is watching.
8. The morning briefing arrives unbidden, on the channel of your choosing, assembled from at least
   three integrations.

### Explicitly not this goal
Multi-user or hosted-for-others operation; a mobile app of our own (Telegram IS the mobile app);
scraping any messaging network that forbids it; and any autonomy tier above the one chosen here —
"act then report" on irreversible actions stays off until the ledger and approvals have earned it.
