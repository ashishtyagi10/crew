# Swarm run timeline in the folded record

## Problem
The folded swarm record (chatswarm::record_text) lists per-task durations
(v0.5.69) but hides the run's *shape*: which tasks ran concurrently, where the
critical path was, how much wall-clock the fan-out actually saved. Once the
live block folds, that information is gone.

## Design
Append a Gantt-style timeline to the folded record as a fenced code block
(monospace-safe through the markdown preview, clipped not wrapped on narrow
panes):

```
- ✓ research — 12.4k tok · 3.2s
- ✓ merge — 2.1k tok · 9.4s

```
timeline · 12.4s
research  ██████··············
merge     ····████████████████
```
```

- One bar row per task that ever started, in plan order; never-started tasks
  (cancelled out of Pending) are omitted — the list above already shows ⊘.
- Bar maps [start_offset, end_offset] within the run span onto BAR_W=20 cells:
  start floors, end ceils, minimum 1 filled cell, '█' active / '·' idle.
- Header: `timeline · <fmt_elapsed(total)>` — total = max end offset.
- Emitted only when ≥2 tasks have timing AND total > 0 (a single bar or a
  zero-length run adds nothing).
- Titles clipped to 14 display columns (chatwidth::fit_end — CJK-safe) and
  padded to the widest clipped title.

## State
- `SwarmStatus.run_started: Instant` stamped in `new()` (plan arrival).
- Offsets computed at fold time: `start = started - run_started`,
  `end = start + elapsed_ms`; a task still Running at fold (error-path fold)
  gets `end = now - run_started` — honest partial bar.

## Modules (200-line cap)
- `chattimeline.rs` — pure renderer `timeline_block(&[(String, Option<(u64,u64)>)]) -> Option<String>`.
- `chatswarmrec.rs` — folded-record rendering moved out of chatswarm.rs
  (record_text, spans, fmt_tok, glyph); chatswarm.rs keeps state + fold and
  gains `run_started`.
