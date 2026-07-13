# Esc interrupts a running crew turn — design

Date: 2026-07-13
Status: autonomous-loop iteration 4 (inspired by Codex CLI / Claude Code Esc-to-interrupt)

## Problem

While a `/crew` turn or swarm is streaming, Esc in the chat pane closes the
whole pane (losing the live view); the only way to cancel the run is typing
`/stop` into the composer mid-stream. Codex and Claude Code both bind Esc to
"interrupt the running work" — closing is what Esc means only when idle.

## Design (app-side; broker's existing /stop path is reused)

- `ChatPane::on_input`, `ChatInput::Close` arm: when `is_busy()` AND
  `connected`, do NOT return `ChatAction::Close`. Instead send the broker the
  literal `/stop` text (same `PluginCommand::Send` the composer would issue —
  the broker's existing cancel path handles it mid-run) and locally note the
  action in the transcript as a muted "crew" message: `⎋ interrupting — sent
  /stop`. Return `None`.
- Esc when NOT busy (or disconnected): unchanged — `ChatAction::Close`.
- Popup precedence unchanged: an open palette/mention popup still consumes
  Esc first (existing order in `on_input` already guarantees this).
- Repeat Esc while still busy: resend `/stop` (broker cancel is an idempotent
  AtomicBool) but do NOT duplicate the transcript note if the last message is
  already the interrupt note.
- The queued-messages feature (v0.5.64) interacts: Esc-interrupt does NOT
  clear the queue — the run cancels, the turn ends, and the queue then
  flushes normally (deliberate: typed work is never discarded).
- Hint: while `is_busy()`, the header's status segment (chathdr) appends a
  muted ` · esc interrupts`; when idle it shows nothing new. Width-permitting
  only — on narrow panes the existing header truncation rules win.

## Testing

- Unit (`on_input` level, no winit): busy+connected Esc → no Close action,
  /stop written (observable via the queue-bypass send path — assert on
  awaiting staying latched and, where the tests can observe sends, the send);
  idle Esc → Close; disconnected-but-busy Esc → Close (no dead-pipe write);
  popup-open Esc still goes to the popup; double-Esc doesn't duplicate the
  note.
- Header: busy → hint text present on wide panes, absent when idle, absent on
  narrow panes (truncation).
- Existing Esc/Close tests must be updated deliberately (idle behavior
  unchanged; only busy behavior differs) — list every changed assertion.

## Out of scope

Esc-Esc-to-close-while-busy chords; interrupting `@agent` relay turns
differently from swarm turns (both go through the same /stop); toast/overlay
animations.
