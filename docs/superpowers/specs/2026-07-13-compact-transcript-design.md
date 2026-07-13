# Ctrl+O compact transcript toggle — design

Date: 2026-07-13
Status: autonomous-loop iteration 6 (inspired by Claude Code's Ctrl+O collapsed/expanded transcript)

## Problem

A single swarm run adds a plan line, several per-task output messages, a
folded record, and a status line. After a few runs the conversation itself
(user asks, final answers) is buried. Claude Code solves this with Ctrl+O:
collapse detail, expand on demand.

## Design (pane-local, purely presentational — mirrors `show_source`)

- `ChatPane` gains `compact_view: bool` (not persisted, default false),
  toggled by Ctrl+O. Decode in `chatkeys` as a new `ChatInput::ToggleCompact`
  (follow how the existing Ctrl+Shift+M / source toggle is decoded and
  routed); popups still get keys first.
- Rendering: in compact mode each message renders its header line plus the
  FIRST body line only; when body lines were hidden, the shown line gets a
  muted suffix ` … +N` (N = hidden line count). Messages whose body is a
  single line render unchanged. The swarm status block, queued indicator,
  composer, and header are NOT affected — this is transcript-only.
- Threading: `card_lines`/`card_line_count`/`message_cells`/`placed_lines`
  already take `source: bool`; widen that to a small `View { source, compact }`
  copy-struct (or a second bool if the codebase style prefers — implementer's
  call, consistently applied) so scroll math, scrollbar, link hit-tests and
  the unread pill all agree automatically.
- Header indicator: while compact, the header status segment shows a muted
  `compact` chip (same segment family as the esc-interrupts hint; dropped
  first on narrow panes, after the esc hint).
- Interaction with `show_source`: orthogonal — source mode shows raw text,
  compact clamps lines; both can be on (raw text, one line). No special case.
- `/export` is NOT affected (exports full transcript regardless).

## Testing

- Toggle: Ctrl+O flips the flag; popup-open Ctrl+O goes to the popup first
  iff the popups consume it today (match existing popup key contract);
  second Ctrl+O restores.
- Render: multi-line message → header + first line + ` … +N` suffix in
  compact; single-line message identical in both modes; line counts
  (card_line_count) shrink accordingly and scrollbar/unread math follows;
  link on a hidden line no longer hit-tests, link on the visible line still
  does.
- Header chip present when compact (wide), dropped on narrow.
- Existing render tests unchanged when compact_view=false (default-off
  regression guard = full suite staying green).

## Out of scope

Per-message expand (click a message to expand just it); persistence; auto-
compact heuristics; touching the swarm block or queue indicator.
