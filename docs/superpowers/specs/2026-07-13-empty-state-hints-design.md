# /crew empty-state quick-start hints — design

Date: 2026-07-13
Status: autonomous-loop iteration 9 (OpenCode/Claude Code-style welcome)

## Problem

A fresh `/crew` pane's empty state announces the roster but none of the
interactions — and the last four releases added interactions nobody is told
about (type-ahead queueing, Esc-interrupt, Ctrl+O compact, live timings).
OpenCode/Claude Code welcome screens earn their keep by teaching 3-5 keybinds
at the exact moment the user is looking at an empty pane.

## Design (all in `chatempty.rs`; render-only)

Below the existing connected/agents content, when the pane is TALL enough
(existing empty-state layout rules decide; hints are the first casualty on
short panes), render a muted hint block:

    ─ quick start ─────────────────
    Enter    send · type while busy to queue
    @agent   address one agent · plain text runs a swarm
    Esc      interrupt a running turn (idle: close pane)
    Ctrl+O   compact transcript · Ctrl+Shift+M raw text
    /        command palette

- Column-aligned key/description pairs, key in the accent color, text muted;
  the `─ quick start ─` rule uses the same legend styling family as the
  fieldset cards (but inside the pane body, not a border).
- Width rules: below the width needed for the longest line, drop the ` · …`
  second clauses; below that, drop the whole block (never wrap mid-hint).
- Disconnected state: hints render only when `connected` (the connecting
  hint keeps the pane clean).
- Static content — no per-frame cost beyond the cells (empty panes don't
  redraw continuously anyway).

## Testing

- Tall+wide connected pane: block present, keys accent-colored, aligned
  starts (assert by column positions, not substrings — lesson from iter 8).
- Short pane: block absent, existing empty-state content unchanged.
- Narrow: second clauses dropped; very narrow: block absent.
- Disconnected: absent.
- Existing chatempty tests unmodified.

## Out of scope

Persisted "don't show again"; dynamic hint rotation; localization; touching
the welcome (non-chat) app screen.
