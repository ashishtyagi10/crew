# Queued messages while the crew is busy — design

Date: 2026-07-13
Status: autonomous-loop iteration 3 (user pre-authorized feature selection; inspired by Claude Code / Codex CLI type-ahead queueing)

## Problem

While a `/crew` swarm (or any turn) is in flight, pressing Enter sends the text
to the broker immediately. The broker worker is single-threaded per session, so
the message sits invisibly in stdin until the current run finishes — the user
gets no feedback that it was neither answered nor lost, and several sends
interleave confusingly with the streaming run.

## Design

App-side only (no broker/protocol change):

- `ChatPane` gains `queued: std::collections::VecDeque<String>`.
- In `on_input`'s send path: if `is_busy()` and the text is broker-bound,
  push to `queued` instead of `plugin.send`. Exceptions that BYPASS the queue:
  - `/stop` (must reach the broker mid-run — it's the cancel path);
  - pane-local intercepts (`/export`, `/theme`, `/compact`, `/exit`), which
    already return before the send path.
- Flush: at the end of `poll()`'s event drain, if `!is_busy()` and the queue is
  non-empty, pop the front and send it (set `awaiting = true` again). One
  message per turn — the next flush happens when that turn ends. Also transcribe
  the sent text as a user-side Message the same way a direct send does (match
  existing behavior — if direct sends don't locally echo, neither do flushed ones).
- Broker death (`Error` event): keep the queue (the pane shows disconnected;
  clearing would lose typed work silently).
- Render: when the queue is non-empty, a muted one-line indicator directly
  above the composer: `⧗ 2 queued — sends when the crew is idle` (singular/
  plural). It claims one row from the message budget like the swarm block does.
- No queue editing in v1 (YAGNI): no reorder/delete; Esc keeps its existing
  meanings.

## Testing

- Unit: busy send → queued (no plugin write), idle send → direct; /stop
  bypasses while busy; flush on the busy→idle transition sends exactly one and
  re-latches awaiting; queue survives Error; indicator row math (budget) and
  cells (count text) on wide/narrow panes.
- The existing chat tests must stay green (no behavior change when queue empty).

## Out of scope

Broker-side queue awareness, queue persistence across sessions, editing queued
entries, multi-message flush per idle gap.
