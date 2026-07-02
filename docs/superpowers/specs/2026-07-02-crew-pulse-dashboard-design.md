# Crew Pulse — agent activity dashboard in the crew pane

**Date:** 2026-07-02
**Goal:** the most intuitive crew-panel UI: add charts/bars/timers that show
meaningful, live information about the AI agents, inspired by the best
agent-observability tools.

## Inspiration (what the best tools converge on)

- **LangSmith / AgentOps**: an execution timeline (waterfall) is the single
  most legible view of a multi-agent turn — who ran, in what order, for how
  long.
- **btop / lazygit / k9s**: spatial consistency — fixed lanes in fixed places;
  the eye learns where an agent lives and goes there automatically. Block-glyph
  charts (`▁▂▃▄▅▆▇█`) read surprisingly well at cell resolution.
- **Codeman / agent monitors**: live elapsed timers and token/cost meters are
  the "is it stuck?" signal users check first.
- **dataviz rules applied**: color = identity (one stable hue per agent,
  reused everywhere), magnitude = bars on a shared scale, sequence = waterfall;
  direct labels; text stays in ink/muted tokens, marks carry the color.

## Approaches considered

- **A. Status strip of per-agent stat tiles** — always-visible cards (state,
  timer, totals). Glanceable but no history, no sequence.
- **B. Turn waterfall** — LangSmith-style hop timeline. Best single view of a
  relay turn, but alone it lacks live per-agent state.
- **C. Separate analytics pane** — a whole dashboard pane. Most room but
  breaks the "one canvas" vision and splits attention; the data (a handful of
  hops/turn) doesn't warrant a pane.

**Chosen: A+B fused inline** — a "pulse" block inside the crew pane: one lane
per agent (identity + live state + duration sparkline + time-share bar) plus a
turn waterfall row. C rejected (YAGNI, one-canvas vision).

## Design

### Placement

Inside the crew pane, directly under the status header, replacing the roster
row and the activity row when the pane is tall enough (they carry a subset of
the pulse's information). Height gating in `ChatPane::top_rows`:

- `rows >= 14` and agents known → header (1) + one lane per agent (≤6) +
  waterfall (1).
- Shorter panes keep today's header/roster/activity rows unchanged.

### Lane row — one per agent, roster order (spatial consistency)

```
▸ planner qwen-max ⠹ 4s   ▂▃▅▂▁▄▂▃▅▆▂▁▃▄▂▃  ██████░░░░ 62%
▪ coder   qwen-max ·3× 2.1s ▁▂▁▃▂▁▂▁▁▂▁▁▂▁▁▂  ███░░░░░░░ 28%
```

- **Marker + name** in the agent's stable color (`chatroster::agent_color`),
  `▸`+bold while thinking, `▪` idle — same convention as the roster row.
  Names padded to the longest so lanes align. Dimmed `short_model` badge when
  width allows.
- **State**: thinking → braille spinner + live elapsed `Ns` (accent, ticks on
  the busy-animation frames that already flow); idle → the roster's dimmed
  `·n× avg` stat.
- **Duration sparkline**: that agent's recent hop durations (`spark::History`,
  cap 32) drawn with `spark::line_cells` in the agent color. **All lanes share
  one scale** (max across visible windows) so bar heights compare across
  agents.
- **Time-share bar + %**: agent's share of total reply milliseconds
  (`agent_stats`), gauge-style `█`/`░` fill in the agent color over the
  `border_normal` track, percent in muted ink.

Progressive disclosure by width: name+state always; sparkline ≥ ~46 cols;
share bar ≥ ~62 cols. Lanes cap at 6 agents (default crew is 3).

### Waterfall row — the turn timeline

```
turn ▶ ██████████ ████ ██▌ 12.4s
```

- Label `turn` in muted ink; one segment per hop of the current (or last
  completed) turn, width proportional to hop duration, colored by
  agent, separated by 1-cell page-bg gaps (surface-gap rule; nonzero hops get
  ≥1 cell).
- **Live**: while a turn is in flight the thinking agent's segment grows each
  frame from `ActiveAgent.since`; total elapsed at the right ticks live. On
  turn end the row freezes as the settled record until the next turn starts.

### Data — app-side hop timing, no protocol change

The broker only emits per-hop `Stats {agent, ms}` at turn end, but `Activity`
events fire live at every hop boundary. A new `Pulse` struct (in
`chatpulse.rs`, owned by `ChatPane`) records:

- `hist: HashMap<String, spark::History>` — per-agent hop durations (ms),
  pushed when an agent transitions thinking→idle (`since.elapsed()`).
- `hops: Vec<(String, u64)>` — the in-flight turn's completed hops, in order;
  cleared when a new turn starts (first `thinking` after a turn ended).
- `turn_done: bool` — flips on the turn-over signal (empty-agent idle); a
  clear-all also flushes still-running agents' elapsed into `hops` so
  cancelled turns still render truthfully.

Existing stores are untouched: `agent_stats` (broker-timed totals) feeds the
share bars and idle stats; `tokens`/`turns` keep feeding the header meter.

### Rendering modules

New file `crates/crew-app/src/chatpulse.rs` (≤200 lines, split if it grows):
pure functions returning `Vec<CellView>`, matching the codebase idiom:

- `lane_cells(cols, row, agent, active, hist, share, scale, name_w)`
- `waterfall_cells(cols, row, hops, live)`
- assembly hook in `chatview::cells` + gating in `top_rows`.

No GPU/renderer changes; everything is cell glyphs + theme colors
(sRGB triples; the renderer handles the linear conversion at its boundary).

### Animation

`ChatPane::is_busy()` is already true while any agent thinks, which keeps
~15 fps redraw frames flowing — the spinner, elapsed labels, and growing
waterfall segment ride those frames for free. Idle panes stay static (no new
wakeups, nothing on the winit thread).

### Testing

Cell-assertion unit tests (the `gauges.rs`/`chatflow.rs` idiom):

- Pulse state: thinking→idle records a hop + history sample; clear-all
  flushes and marks the turn done; the next thinking clears `hops`.
- Lanes: name + spinner + elapsed while active; `·n× avg` when idle; sparkline
  cells bounded and in the agent color; share bar fill ∝ share; graceful
  clipping at narrow widths.
- Waterfall: segment widths proportional with gaps; live segment present while
  in flight; empty without hops.
- `top_rows`: pulse rows on tall panes, legacy rows on short ones.

Live verification via the GUI harness (osascript + screencapture +
`CREW_BROKER_MOCK_REPLY`) across themes.

## Out of scope

- Per-agent **token** attribution (broker protocol change) — durations are the
  honest per-agent series today; tokens stay at the turn/session level in the
  header meter. A follow-up could add `tokens` to reply-level `Stats`.
- A separate analytics pane; historical persistence across sessions.
