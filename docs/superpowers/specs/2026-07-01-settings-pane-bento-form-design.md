# Settings pane: two-column bento form

**Date:** 2026-07-01
**Status:** Approved (bento grouping, boxed inputs, and implementation approach chosen by user; Cmd+S conflict resolution per recommendation)

## Goal

Replace the settings pane's flat one-row-per-field list with a professional
form: two columns of bento cards, boxed text inputs, a multi-line text area,
and keyboard save/cancel (Cmd+S / Alt+S save, Esc cancel).

## Current state

- `crates/crew-app/src/settingspane/` renders a scrollable label/value list
  via ratatui into a `Buffer`, converted by `tui::to_cells` for GPU drawing.
  Crew draws the outer fieldset pane border separately (`panecard.rs`).
- Fields (`mod.rs` `FIELDS`): font family/size, nav width, show nav, theme,
  accent, paper texture/grain, maximized, five notify settings, Save, Cancel.
- Esc already cancels (`commit.rs::escape`); Save requires tabbing to the
  `[ Save ]` button. There is no Cmd+S; **Cmd+S is taken globally** by
  broadcast-toggle (`chords.rs` `"s"` branch).
- Notify patterns are a comma-separated single-line buffer committed to
  `Vec<String>`.

## Design

### Layout

Two-column bento inside the existing pane border:

- **Left column — APPEARANCE card:** font family (boxed input + dropdown),
  font size + paper grain (two short boxes side-by-side), theme (boxed
  `‹ name ›` selector), accent (boxed hex input), paper texture (checkbox).
- **Right column — WINDOW card:** nav width (boxed input), show nav and
  launch-maximized (checkboxes).
- **Right column — NOTIFICATIONS card:** notify / agent-done / bell / exit
  (checkboxes), min secs (short boxed input), patterns (multi-line text
  area, one pattern per line, ~4 content rows).
- Bottom row, right-aligned: `[ Save ⌘S ]   [ Cancel esc ]`.

Cards are ratatui rounded `Block`s with the legend embedded in the top
border — same fieldset-legend language as `boxdraw::titled_card`; dim border,
accent legend on the card containing focus.

Below ~64 columns the cards stack single-column. The existing scroll
machinery keeps the focused field visible; a pure layout function maps each
field to its `Rect` for a given size and is shared with tests.

### Form controls (new `settingspane/form.rs`)

- `input box`: label row above a 3-row `┌─┐` box; focused → accent border and
  block cursor. Two short fields may share one row.
- `checkbox`: single-row `[x] Label`; Space/←/→ toggles (existing behavior).
- `selector`: boxed `‹ value ›`; ←/→ cycles themes (existing behavior).
- `text area`: multi-line boxed input for notify patterns. Enter inserts a
  newline; Tab/Shift-Tab leaves the field; Backspace deletes (including
  newlines). Commit splits the buffer on newlines (was commas); trims and
  drops empties. On-disk config format (`Vec<String>`) is unchanged.
- Font-family dropdown popup is unchanged, anchored under its input box.

### Keyboard

- **Esc** — unchanged: closes the font dropdown if open, else Cancel.
- **Cmd+S** — `chords.rs` `"s"` branch: when the focused pane is the settings
  pane, commit the focused field and emit `SettingsAction::Apply` (save and
  close); otherwise keep broadcast-toggle. Broadcast is meaningless while a
  settings pane is focused, so nothing is lost.
- **Alt+S** — handled in global `keys.rs` before pane routing, matched on the
  **physical** `KeyS` with Alt held (macOS Option+S produces `ß` as the
  logical key), settings-focused only; same save action.
- Tab/Shift-Tab focus order, wheel scroll, and per-field editing semantics
  are unchanged.

### Error handling

Unchanged: parse/clamp on commit (`commit.rs`), invalid input falls back to
the previous draft value, `build_config` clamps the whole config on save.

### Testing

- Layout function: field→Rect geometry at two-column and stacked widths;
  focused-field-visible scrolling.
- Text area: Enter inserts newline, commit splits on newlines, round-trip
  through `refresh_bufs`.
- Chords: Cmd+S saves when settings focused, still toggles broadcast
  otherwise; Alt+S saves via physical-key match; Esc still cancels.
- Update existing `render_tests.rs` / `mod_tests.rs` for the new geometry.

## Out of scope

- No reusable crate-level widget library (single consumer today).
- No mouse hit-testing changes beyond what existing wheel scroll provides.
- No new settings fields.
