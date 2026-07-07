# Minimize pane into the left nav — design

**Date:** 2026-07-07
**Goal:** a button on each pane that minimizes it out of the grid and into the
left-nav PANES list, from which a click restores it.

## Context

Crew already has two adjacent concepts, neither of which is this feature:

- **LRU demotion** (`grid/state.rs`, `minstrip.rs`): panes beyond
  `MAX_FULL_TILES` are automatically demoted to a bottom thumbnail strip.
  Automatic, capacity-driven, unchanged by this feature.
- **The left-nav PANES list** (`panelist.rs`, `navcard.rs`): every pane is
  already listed; clicking a row focuses that pane (`hit.rs::pane_at_sidebar`).

This feature adds an *explicit, user-triggered* minimize: the pane leaves the
grid (and the bottom strip) entirely, keeps running, and lives only as its
PANES row until restored.

## Design

### State: `Pane.hidden: bool`

One new field on `Pane` — `hidden` — meaning "user-minimized into the left
nav". Named `hidden` (not `minimized`) because `GridLayout::minimized()`
already means the LRU bottom strip.

- `reconcile_grid` (app.rs) keeps hidden panes out of the `GridLayout` order:
  it skips `add` for hidden panes and removes any already-tracked hidden index
  via a new `GridLayout::retain(keep)` (plain `Vec::retain` — no index
  shifting, unlike `on_close`).
- Because `compose_grid`, `pane_hit_rects`, the min strip, and scene building
  all derive from `GridLayout`, hidden panes vanish from the canvas with no
  other layout changes.
- Hidden panes stay in `self.panes`: PTYs keep running, `poll` keeps setting
  their `activity` dot, and the PANES list keeps showing them (indices stable,
  so `Cmd+N` still addresses them).

### Restore invariant: focusing a hidden pane un-hides it

In `reconcile_grid`: when the keyboard focus is on a pane (`!input.focused`)
and that pane is hidden, clear `hidden`. The freshly-visible pane re-enters
the LRU at MRU position → becomes a full tile. This single rule makes every
existing focus path a restore path: clicking the pane's PANES row, `Cmd+N`
pane switching, spawn-focus. No changes to `hit.rs` click routing needed.

### The button: `[-]` on the top border

`panecard.rs::Bar` gains `min_btn: bool`. When set (and the card is ≥ 10
cells wide), `pane_card` draws `[-]` at card columns `cols-5 ..= cols-3`,
row 0 — where the status glyphs start today — in the legend color; the
status glyphs (`⇡N » ● !`) start further left. Only full grid tiles set
`min_btn` (threaded through `full_scenes` → `push_pane_scenes`); the zoomed
view and minimized-strip thumbnails do not show it.

Hit-testing: a new pure helper `min_btn_rect(rect, cw, ch) -> Option<Rect>`
mirrors `relayout_one`'s cols math and returns the button's three row-0
cells, `None` when the card is too narrow. In `events.rs`, the left-press
handler checks the button (via `compose_grid`'s full tiles) after Cmd+click
and before `selection_press`, so a button click never focuses or arms a drag.

### Minimize action (`minimize_pane(idx)`, panemanage.rs)

1. Set `panes[idx].hidden = true`; drop `zoomed`.
2. If the nav is hidden, enable `show_nav` (and save config) — the pane
   minimizes *into* the nav, so the nav must be visible.
3. If `idx` was focused, focus the nearest visible pane (by index distance);
   if none remain visible, focus the input bar.
4. Status message: `minimized to nav — click it in PANES to restore`.

### Left-nav marker

`PaneRow` gains `minimized: bool` (sourced from `pane.hidden` in
`navcard.rs`). `panelist::pane_cells` draws a right-aligned accent `[+]`
restore button on minimized rows (ending a cell left of the activity-dot
slot; the title clamps short of it). Title stays `text_muted` as for any
unfocused pane.

## What doesn't change

- LRU demotion + bottom strip behavior and visuals.
- Welcome screen condition (`panes.is_empty()`): all-panes-hidden shows an
  empty canvas beside the nav list, which reads as "everything tucked away".
- No new keybinding, no slash command (button-only, per the goal). Easy to add
  later.
- No persistence: `hidden` resets on restart (panes aren't persisted anyway).

## Error handling / edge cases

- Minimize the focused pane → focus moves to nearest visible pane.
- Minimize the last visible pane → input bar takes focus; canvas is empty.
- Narrow pane (< 10 card cells): no button drawn, no hit region (helper and
  draw share the same threshold).
- Stale grid between event and next frame: same one-frame staleness class as
  existing close/focus paths; every action calls `redraw()`.

## Testing

Cell-level unit tests (the codebase's dominant pattern):

- `grid/tests.rs`: `retain` drops indices without shifting the rest.
- `app_tests.rs`: reconcile skips hidden panes; focusing a hidden pane
  restores it; input-bar focus does *not* restore; `minimize_pane` moves
  focus / falls back to the input bar / enables `show_nav`.
- `paneview_tests.rs` (panecard): `min_btn` draws `[-]` ending at `cols-3`,
  shifts status glyphs left; absent when `min_btn` is false or the card is
  narrow.
- `min_btn_rect`: pixel rect matches the button cells; `None` when narrow.
- `panelist.rs`: minimized row shows a right-aligned `[+]`.

Manual verify: GUI harness (osascript + screenshot) — spawn panes, click the
button, confirm the pane leaves the grid and its nav row restores it.
