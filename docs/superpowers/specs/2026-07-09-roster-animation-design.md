# Roster Animation & Live Feedback (Phase A) — Design

**Date:** 2026-07-09
**Status:** Approved
**Scope:** app-side only (crew-app). Phase B (streaming stats protocol) is a
separate spec: `2026-07-09-streaming-stats-design.md`.

## Goal

Make the crew pane's agent roster read as *live*: bars ease instead of jump,
the working agent visibly breathes, handoffs flash, and token counts roll —
all in the paper/ink aesthetic (restrained motion, no idle repaints).

## Constraints

- All time from `anim::now_ms`; render fns take `now` as a parameter so tests
  inject it. Only the frame loop reads the clock.
- Never animate while idle: the pane exposes `anim_active()` and the existing
  busy/poll machinery drives redraws only until eases settle. An idle crew
  never repaints.
- Restrained motion: ease-out ≤ 400ms, pulse amplitude ≤ 25% blend toward the
  theme accent, no continuous motion on idle rows.
- Roster grid alignment invariants are untouchable: every glyph used stays
  width-1 (the v0.5.57 fallback-glyph correction protects partial blocks in
  any font).

## 1. Animation state — `chatanim.rs` (new)

```rust
pub(crate) struct Eased { shown: f32, target: f32, since_ms: u64 }
```

- `set_target(now, v)` — retargets, restarting the ease from current `shown`.
- `value(now) -> f32` — ease-out interpolation toward target; clamps and
  settles exactly on target at the end of the window.
- `settled(now) -> bool`.

`ChatPane` gains an `anim: RosterAnim` store: `Eased` per (agent, metric)
for ctx%, shr% (250ms window) and tok (400ms window), plus
`flash: HashMap<String, u64>` (agent → handoff start ms).
`RosterAnim::active(now)` is true while any ease is unsettled or a flash is
younger than its 400ms window; `ChatPane::anim_active()` forwards it and the
poll loop keeps requesting redraws while true.

## 2. Eased bars with sub-cell fill — `chatchips.rs`

- `push_segment` receives the eased fraction (0.0..=1.0) instead of a
  rounded pct. Fill = whole `█` cells plus one partial cell from the
  left-eighth blocks `▉▊▋▌▍▎▏` (U+2589..U+258F), remainder `░` track.
- The `NNN%` text shows the *target* percentage — digits never flicker
  mid-ease; only the bar sweeps.
- `AgentView` carries both eased fractions and target pcts (computed by
  `agent_views` from the anim store).

## 3. Active-agent pulse

While an agent's hop is in flight (same condition that drives the spinner),
its marker + name color blends toward the theme accent by
`0.25 * tri(now, 1600)` — a slow breath, not a blink. Idle rows render
exactly as today.

## 4. Handoff flash

`absorb` of an `Activity { state: "thinking", agent, .. }` event records
`flash[agent] = now`. For 400ms the row's foreground blends from accent back
to normal (linear fade). One-shot; entry removed when expired so the map
stays small.

## 5. Token count-up

`tok_text` renders the eased tok value through the existing `fmt_tokens`;
when a reply-level `Stats` lands, `set_target` rolls the number old→new over
400ms. Phase B later retargets the same `Eased` mid-hop from streaming
ticks — no renderer change.

## Testing

- `chatanim` unit tests: ease converges/clamps/settles, retarget mid-ease
  starts from current shown value, `active()` false after settle + flash
  expiry (all with injected `now`).
- `chatchips` tests: partial-block selection per eighth, pct text shows
  target during ease, pulse blend bounded, flash fade bounds, and existing
  alignment tests still pass (all glyphs width-1).
- Live: GUI harness (fonts symlinked into the isolated HOME) over a
  mock-broker turn; screenshot sequence shows bar sweep + flash.

## Out of scope (YAGNI)

Protocol changes (Phase B), sparkline changes, sounds, configurable
animation speeds/curves, animating any pane other than the crew roster.
