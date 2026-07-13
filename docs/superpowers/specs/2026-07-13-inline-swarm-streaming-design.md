# Inline swarm streaming + universal pane close — design

Date: 2026-07-13
Status: approved

## Problem

Two related UX gaps in the default `/crew` swarm engine (see
`2026-07-12-crew-hive-default-engine-design.md`):

1. **Swarm feedback opens a separate pane and looks frozen.** A plain `/crew`
   message runs a broker-side crew-hive swarm; the app auto-opens a companion
   "hive" SwarmPane on `HivePlan`, splitting the user's attention across two
   panes. Worse, the broker buffers all `HiveEvent` telemetry in an mpsc
   channel and emits it only after the scheduler completes
   (`crew-plugin/src/broker/swarm.rs`), so nothing streams live — real runs
   look frozen until they suddenly dump everything. The desired experience is
   claude/codex-style: progress and results stream inline in the same chat
   pane the user typed into.

2. **Panes can't be closed with the mouse — and the swarm pane can't be
   closed at all.** The only close affordances are per-pane-type keys
   (Esc on Far/Markdown/Chat, shell exit on Terminal). SwarmPane ignores key
   input entirely, so once opened it can only be minimized, never closed.

## Decisions (user-approved)

- The companion hive pane is **removed entirely** — the chat pane is the
  single surface for swarm runs. (The standalone `/swarm` batch pane is
  unaffected except for gaining closability.)
- Swarm progress renders as a **compact live status block** in the chat
  transcript, not as one chat message per state change.
- The **streaming drain is fixed in this work** — it is a prerequisite for
  the inline feedback feeling live.
- Every pane gets a **`[x]` close button** on the top border next to the
  existing `[-]` minimize button, and Swarm panes gain **Esc → close**.

## 1. Broker: live event drain (`crew-plugin/src/broker/swarm.rs`)

`run_with` currently does `rt.block_on(join!(sched.run(), drain, governor))`
where `drain` forwards bus events into an mpsc channel, then translates and
emits the channel's contents *after* `block_on` returns.

Restructure so the drain future calls `emit` directly while the run executes:

```rust
let outcome = rt.block_on(async {
    let drain = async {
        loop {
            match sub.recv().await {
                Ok(ev) => { /* sum tokens; emit(Hive) + emit(translate(ev)) */ }
                Err(Lagged(_)) => continue,
                Err(Closed) => break,
            }
        }
    };
    match governor {
        Some(g) => tokio::join!(sched.run(), drain, g).0,
        None => tokio::join!(sched.run(), drain).0,
    }
});
```

- Only the drain closure borrows `emit`; the scheduler and governor don't.
  On the current-thread runtime `join!` interleaves them, so events reach
  stdout as they happen. The blocking `emit` write on this worker thread is
  acceptable (it is exactly what the post-hoc loop did).
- Token totals for the aggregate `Stats` accumulate inside the drain.
- `emit` failures: record the first error, stop translating/forwarding, let
  the drain keep consuming until `Closed` so the scheduler can finish;
  return the recorded error after `block_on`.
- The drain terminates when every bus sender drops (`RecvError::Closed`) —
  the same condition that ends today's forwarding loop.
- Task outputs already arrive as **one `OutputChunk` per completed task
  reply** (`crew-hive/src/apiagent/mod.rs`), so the existing
  chunk-to-`Message` translation gives per-task result drops the moment each
  task finishes. No protocol change and no per-token deltas.
- Ordering guarantees kept: nothing is emitted before `HivePlan`; the
  aggregate `Stats` still lands before the final summary message.

## 2. App: swarm progress renders inside the chat pane

### Event routing (`crew-app/src/chatevents.rs`, `poll.rs`)

`PluginEvent::HivePlan` / `Hive` stop being `HostAction`s (no pane
creation). They become pane-local chat state, absorbed by the ChatPane like
`Activity`/`StatsTick` are today. The wire protocol is unchanged — stdio/CLI
clients still receive the same events.

### Chat state (`crew-app/src/chat.rs` + new module)

ChatPane gains `swarm: Option<SwarmStatus>`:

- Built from `HivePlan`: ordered task list (id, title), all pending.
- Updated by `AgentSpawned` (task → running), `TaskStateChanged`
  (done/failed/cancelled), `TokenDelta` (per-task token count via the
  agent→task map, mirroring the broker's `translate` bookkeeping).
- A new `HivePlan` while a block exists resets it (same reuse semantics the
  hive pane had).

### Rendering (`crew-app/src/chatbody.rs` / sibling)

A compact status block pinned after the last transcript message while the
run is live (visually akin to the existing activity row):

```
  ⠋ research APIs      12.4k tok
  ✓ draft outline       3.1k tok
  ⠋ merge results
```

Spinner for running, `✓`/`✗`/`⊘` for done/failed/cancelled, right-aligned
token counts. When the run ends the block stays in the transcript as a
static record of the run. Width-clamped like other chat rows; on narrow
panes token counts drop first.

### Deletions

- `crew-app/src/hivepane.rs` (auto-open companion pane) and its tests.
- The companion-close special case in `app.rs::close_pane`.
- `HostAction::HivePlan` / `HostAction::Hive` and their `poll.rs` arms.

The broker's one-line "planned N task(s): …" chat message stays — it also
serves stdio clients and the session log/export.

## 3. Universal pane close: `[x]` button + Esc on Swarm panes

### Border button (`crew-app/src/panecard.rs`)

The top-right corner slot becomes `[-][x]`: `[x]` takes the corner
(card columns `cols-5 ..= cols-3`), `[-]` shifts left to `cols-8 ..= cols-6`;
status glyphs step past both. A `close_btn_rect` mirrors `min_btn_rect` and
shares the column constants so draw and hit-test agree. The width floor
(`MIN_BTN_COLS`) rises so both buttons appear only when legible; on cards too
narrow for both, neither is drawn (same all-or-nothing rule as today).
`Bar.min_btn` gating (full tiles + zoomed tile, not strip thumbnails) applies
to both buttons.

### Hit-test + click (`crew-app/src/hit.rs`, `events.rs`)

`close_btn_at_cursor` mirrors `min_btn_at_cursor`; a click routes to
`close_pane(i)`, which already handles grid reconciliation, zoom exit, and
focus fallback. No confirmation dialog — consistent with every existing
close path (Esc, shell exit).

### Keyboard (`crew-app/src/keys.rs`)

Swarm panes currently swallow all keys. Give `PaneContent::Swarm` Esc →
close, consistent with Far/Markdown/Chat.

### Teardown check

`close_pane` removes the `Pane`, dropping its content. Implementation must
verify Terminal drop kills the PTY child and Chat drop tears down the broker
child (the historical "plugin leaks broker child" bug path) — `[x]` makes
closing far more reachable than Esc was.

## Error handling

- Planning failure: unchanged degrade-to-single-task path; the status block
  simply shows one task.
- Task failure: unchanged — chat-visible `✗ failed:` message (never
  `PluginEvent::Error`), plus the block marks the task `✗`.
- Budget/`/stop` cancellation: `TaskStateChanged(Cancelled)` marks tasks `⊘`;
  the existing "swarm cancelled" summary closes the run.
- Broker death mid-run: existing `PluginEvent::Error` → disconnected banner;
  the block stays frozen at its last state (acceptable — the banner explains).

## Testing

- **Broker** (`swarm.rs` tests): existing ordering tests stay green
  (plan-first, stats-before-summary, failure-as-message, resume-fold). Add a
  liveness test: a two-task graph whose second task's factory blocks until
  the test observes the first task's emitted events — proves emission happens
  during the run, not after.
- **Chat state**: HivePlan builds the block; state/token events update it;
  a second HivePlan resets it; events without a block are ignored (no panic).
- **Render**: block appears after the last message, glyphs match task
  states, static after run end; narrow-width clamp.
- **Panecard**: `[-][x]` drawn on wide cards, neither on narrow; rect/draw
  agreement tests mirror the existing `min_btn` pair.
- **App**: `[x]` click closes exactly that pane; Esc closes a Swarm pane;
  `hivepane.rs` tests deleted with the module.
- **GUI harness** (`.claude/skills/verify`): mock-provider `/crew` run shows
  the block updating live in the chat pane and no second pane appearing.

## Out of scope

- Token-by-token streaming inside a message bubble (needs a new
  `MessageDelta` protocol event; per-task drops already read as streaming).
- Any `/swarm` batch-pane change beyond closability.
- Close-confirmation prompts.
