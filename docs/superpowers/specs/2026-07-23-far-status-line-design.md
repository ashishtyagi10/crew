# /far mini-status line — design

**Date:** 2026-07-23
**Problem:** Long file names are truncated in the /far panel columns, and the
existing fallback — the selected name right-aligned inside the command bar —
is *dropped entirely* when it doesn't fit, which is exactly the long-name
case. The user can never read a long name in full.

## Decision

Add an always-visible one-row **status line** between the panels and the
command line (Midnight Commander's mini-status), carrying the active panel's
selected entry in full.

## Layout (`farpane/render.rs`)

Vertical split becomes four rows:

| Row | Constraint | Content |
|---|---|---|
| panels | `Min(3)` | dual directory panels |
| status | `Length(1)` | selected entry, full name |
| command | `Length(1)` | `<cwd> $ <typed>▏` |
| fkeys | `Length(1)` | function-key bar / prompt |

The tiny-size guard bumps from `rows < 5` to `rows < 6`.

## Status line (`farpane/bars.rs`, new `status_bar`)

- Content: the active panel's `selected_label()` — `name/` for directories,
  `name · size` for files (helper already exists).
- Style: ink on page background, left-aligned at column 0, matching the
  command line's left edge.
- Overflow: if the label exceeds the row, truncate the *name* with `…` but
  keep the ` · size` suffix intact (same rule the listing rows use).
- Empty listing: blank row — the layout never jumps.

## Cleanup

`command_bar` loses its `selected` parameter and the right-aligned name
rendering; `render.rs` stops computing/passing it. The name lives in exactly
one place and the command line regains its full width for typing, the ghost
suggestion, and the running-command note.

## Testing (`farpane/render_tests.rs`)

1. A long-named file selected in a narrow pane: the panel row shows the
   truncated form, the status row shows the full name.
2. A name longer than the whole row: status row ends with `… · <size>` —
   suffix intact.
3. The command bar line no longer contains the selected name.
4. Empty directory: status row is blank, command line still on the row above
   the F-key bar.

Out of scope: per-panel status lines (only the active panel is shown),
free-space/percentage info, and any change to the F-key bar.
