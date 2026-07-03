# Crew pane status: dense KPI chip grid

2026-07-02. Approved direction: KPI chip grid; drop the per-agent sparkline
and share-gauge visuals, keep the turn waterfall.

## Problem

The `/crew` chat pane's top spreads its status across three sparse rows: a
header line with right-anchored segments, a roster row (or, once engaged, one
full-width **pulse lane per agent** carrying a hop-duration sparkline + a
share-of-time gauge bar + a context meter). Each agent claims a whole row with
a wide whitespace gap in the middle. The goal is to pack the same information
as densely as possible and leave room to keep adding items.

## Design

One chip system replaces the header-segments + roster + pulse-lanes, in three
top-down zones drawn by `chatview::cells` above the message body:

### 1. Session line (row 0)
`crew · <channel>` on the left, then dim bracketed chips packed left-to-right:
`[N turns] [~X tok] [M msgs] [●]` (the `●` is the connection dot, green
connected / red not). A live spinner + active-agent chip appears here while a
turn runs (`⠙ planner · 3.2s`). Session-level metrics are chips here; a new one
is just another bracket.

### 2. Agent chips (rows 1..)
Each agent renders as a compact colored cluster, flowing left-to-right and
**wrapping** to fill the pane width (so 3 narrow agents share one row):

```
▸planner qwen-max ⠙3.2s 4.1k 38% 42%    ▪coder qwen-max ·2× 6.0k 61% …
```

Per agent, in order: marker (`▸` active / `▪` idle) + name in the agent's
stable color; then dim values — model (short), state (`⠙3.2s` thinking /
`·n×` idle-with-count / `idle`), tokens, ctx%, share%. A fixed 2-space gutter
separates clusters. Clusters are equal-width (padded to the widest visible
cluster) so they read as a grid, not ragged text.

### 3. Turn waterfall (last row)
Unchanged: `turn ▶ ████ ██ █ 12.4s`, its own full-width line — the single
retained chart (`chatpulse::waterfall_cells`).

## What is removed
- The per-agent **hop-duration sparkline** (`spark::line_cells`, 16 cells) and
  the **share-of-time gauge bar** (10 cells) in each pulse lane. Their info is
  preserved numerically: `share%` becomes a chip; the sparkline is dropped
  (the waterfall already shows the current turn's hop shape). `lane_cells` and
  its sparkline/bar helpers are deleted or reduced to the chip formatter.
- The separate legacy roster row + activity row: folded into the chip grid,
  which is shown always (idle or engaged), so the two old modes unify.

## Width behavior
Chips left-pack; trailing per-agent values drop in priority order as the pane
narrows — `share%` first, then `ctx%`, then `tok`, then `model` — leaving at
minimum `marker+name+state`. This mirrors the current progressive region-drop.
A pane too short for even one agent row falls back to just the session line.

## Rows consumed (`top_rows`)
Now variable: `1 (session) + ceil(visible_agents / chips_per_row) + 1
(waterfall when a turn has run)`. `chips_per_row = max(1, usable_cols /
cluster_width)`. `top_rows` and `pulse_lanes` are replaced by one function that
returns the grid's row count for the current width/height/agent-count, so the
message body sizes correctly.

## Components
- `chatchips.rs` (new): pure formatters + layout math — `session_chips(...) ->
  Vec<Chip>`, `agent_chip(agent, state, tok, ctx, share) -> Chip`,
  `pack(chips, cols) -> rows` (wrap + equal-width), `grid_rows(agents, cols) ->
  u16`. Chip = `{ text, color }` cell runs. Pure string/geometry in/out, unit-
  testable without a pane.
- `chatview.rs`: composes the three zones from `chatchips`, replacing the
  header/roster/pulse-lane calls.
- `chathdr.rs`: reduced to the session line (reuses `fmt_tokens`).
- `chatpulse.rs`: keeps `waterfall_cells` and the hop/timing bookkeeping;
  loses `lane_cells` + sparkline/bar plumbing. `chatroster.rs` keeps
  `agent_color`; loses the roster-row renderer if unused elsewhere.

## Testing
- `chatchips`: session chip text (pluralization, `~tok`, dot color); agent chip
  contents per state (thinking/idle/idle-with-count); `pack` wrap + equal-width
  + gutter; width-drop priority order; `grid_rows` math (1 agent, N agents,
  narrow → 1/row, too-short → session only).
- `chatview`: golden-ish assertions that the composed cells contain the session
  chips, one cluster per agent, and the waterfall row; body offset matches
  `grid_rows`.
- No sparkline/share-bar cells appear in the output.

## Out of scope
- No change to hop timing / `Pulse` bookkeeping or the waterfall itself.
- No new metrics added now (the grid just makes adding them trivial later).
- Sidebar panels and other panes are untouched.
