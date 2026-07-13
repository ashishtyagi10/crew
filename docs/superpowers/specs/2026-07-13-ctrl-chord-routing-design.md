# Unified ctrl-chord routing (InputBar passthrough fix) — design

Date: 2026-07-13
Status: autonomous-loop iteration 7 (follow-up to the v0.5.67 review callout)

## Problem

Two inconsistencies in ctrl-chord handling:
1. With the docked command bar focused, unhandled ctrl-chords fall through to
   text insertion — Ctrl+O literally types "o" into the bar.
2. Ctrl+Shift+M (source toggle) is globally intercepted in keys.rs and works
   from anywhere, while Ctrl+O (compact toggle, v0.5.67) is decoded inside the
   chat pane's key path and dies at the input bar. Same family of toggle,
   different reach.

## Design

1. **InputBar stops inserting ctrl-chords** (`inputkeys.rs`): when the ctrl
   modifier is held and the key is a `Key::Character`, do not insert text.
   The bar's own editing chords (Ctrl+W delete-word, Ctrl+U clear-line — and
   any others already special-cased) keep working; everything else becomes a
   no-op instead of a literal character.
2. **Ctrl+O moves to the global-intercept family** (`keys.rs`): intercept
   exactly like Ctrl+Shift+M's existing block — when the chord matches and the
   focused pane is a Chat pane, toggle `compact_view` on it and return
   (regardless of whether the input bar has focus, matching Ctrl+Shift+M).
   The chatkeys `ChatInput::ToggleCompact` decode and its on_input arm are
   DELETED (single handler — no double-toggle path). Popup interaction is
   unaffected in practice: the v0.5.67 review verified popups only consume
   Up/Down/Complete/Enter/Close, so the popup-precedence test for ToggleCompact
   is deleted with the decode (its guarantee is now vacuous by construction).
   Terminal panes are untouched: the intercept fires only when the focused
   pane is Chat, so terminals keep receiving raw Ctrl+O bytes.

## Testing

- Input bar focused: Ctrl+O → focused chat pane toggles compact, bar text
  unchanged; Ctrl+<random letter> (e.g. Ctrl+K) → bar text unchanged, no-op;
  Ctrl+W/Ctrl+U editing chords still work; plain "o" still inserts.
- Chat pane focused: Ctrl+O toggles exactly once per press.
- Terminal pane focused: Ctrl+O does NOT toggle any chat pane and the byte
  (0x0f) still reaches the PTY (existing key_to_bytes path).
- Ctrl+Shift+M behavior byte-identical before/after (regression guard).
- v0.5.67's compact rendering tests unaffected (only the toggle's routing
  moves, not its effect).

## Out of scope

A general keymap/config system; rebindable chords; any new chords beyond
moving Ctrl+O; touching Ctrl+Shift+M's implementation.
