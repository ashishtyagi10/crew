# Per-task timings in the swarm block and record — design

Date: 2026-07-13
Status: autonomous-loop iteration 8 (OpenCode/Claude Code-flavored run timeline)

## Problem

A finished swarm run's record shows what ran and token counts, but not where
the time went; the live block shows a spinner but not how long a task has
been running. Codex/Claude Code both surface elapsed time continuously.

## Design (app-side; no protocol change — times are stamped on event arrival)

- `SwarmTask` gains `started: Option<std::time::Instant>` and
  `elapsed_ms: Option<u64>` (duration captured at terminal state).
  `SwarmStatus::apply` stamps: `AgentSpawned`/`TaskStateChanged(Running)`
  (whichever arrives first) sets `started` once; a terminal
  `TaskStateChanged` (Done/Failed/Cancelled) sets
  `elapsed_ms = started.elapsed()` (None → stays None, e.g. cancelled
  before start).
- Live block (`chatswarmview::block_cells`): running tasks append a muted
  right-side ` 12s` elapsed (before the token count column; dropped with the
  same width rule as tokens — tokens drop first, then elapsed). Elapsed
  derives from `started` at render time — the existing per-frame redraw
  while busy animates it for free. For testability, thread `now` in the same
  way the spinner already threads `now_ms` (0 = skip elapsed rendering in
  tests that don't care).
- Folded record (`SwarmStatus::record_text`): tasks with a captured duration
  append ` · 3.2s` after the token part: `- ✓ research — 12.4k tok · 3.2s`.
  Format: `<1s` → `0.Xs`, else one decimal up to 99s, else `MmSSs` (reuse an
  existing duration formatter if one exists — check `chattime`/`chatchips`
  for the roster's `4.2s` style and share it rather than writing a third).
- No bars/Gantt in v1 (YAGNI): durations only.

## Testing

- apply(): started set once (AgentSpawned then Running doesn't reset);
  terminal state captures elapsed_ms; cancelled-before-start leaves None.
  (Instant can't be mocked cheaply — assert `elapsed_ms.is_some()` and
  monotonic sanity rather than exact values, and unit-test the FORMATTER
  exactly instead.)
- record_text: with elapsed_ms Some(3200) → " · 3.2s" suffix; None → no
  suffix; formatter edge cases (900ms, 3.2s, 61s, 125s).
- block_cells: running task with started + non-zero now shows elapsed; width
  rules (tokens drop before elapsed; both drop on very narrow panes);
  finished tasks in the live block show no elapsed (their spot shows the
  glyph state as today).
- Existing swarm/record tests stay green (suffix only appears when timing
  data exists — old tests construct tasks without timestamps).

## Out of scope

Gantt bars; total-run wall-clock in the summary (broker already reports
Stats); persisting timings; per-agent (vs per-task) attribution.
