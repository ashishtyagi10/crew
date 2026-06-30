# Crew Paper-Reader Themes — Design

**Date:** 2026-06-30
**Status:** Approved (design); ready for implementation plan

## Summary

Give crew two themes — `paper-light` and `paper-dark` — styled to feel like an
e-ink reader: warm "paper" backgrounds, soft "ink" text, no pure black or white,
and a muted, ink-toned ANSI palette so terminal pane output stays readable on
paper. Themes switch live via a `/theme` command and a hotkey, with the choice
persisted to config.

The paper treatment reaches **all the way into the terminal panes** (agent/shell
output), not just crew's own chrome — that is what sells the "reader page"
illusion. Paper *texture* (grain/vignette) is explicitly deferred to a later
phase behind a config flag.

## Motivation / context

Today crew ships a single, hardcoded dark theme. The only themeable color is the
accent, stored in an atomic global in `crew-app/src/palette.rs` and pushed into
the renderer per-frame via cell colors. Every other color (backgrounds, borders,
legends, status glyphs, terminal default fg/bg, the 16 ANSI colors, the wgpu
clear color) is a hardcoded constant scattered across ~8 sites in three crates.

Crew's "one canvas / fieldset panels" aesthetic is already close to an e-reader
page, so a paper look is a natural fit. The work is mostly (a) centralizing the
scattered colors and (b) remapping the terminal ANSI palette so pane content
reads as ink on paper rather than neon on a screen.

## Architecture

### New crate: `crew-theme`

A small, dependency-light crate at `crates/crew-theme` that all three other
crates depend on (`crew-app`, `crew-render`, `crew-term`). It is the single
source of truth for every UI color.

Contents:

- **`Theme` struct** — one field per color the UI uses today (see "Theme
  fields" below), including a 16-entry ANSI palette plus terminal default
  fg/bg.
- **Two `&'static` presets**: `PAPER_LIGHT` and `PAPER_DARK`.
- **Lock-free current-theme selector**: an `AtomicU8` holding the active preset
  index, with `current() -> &'static Theme` resolving the index to a preset.
  Reads MUST be lock-free / non-blocking — `current()` is called every frame on
  the winit thread, and per the project's main-thread-blocking rule nothing on
  that path may block. `set_theme(idx)` stores the index.
- **Default accent per theme**: each preset carries a default accent. The
  existing user-overridable accent atomic in `crew-app` stays; when the user has
  NOT set `accent =` in config, the theme's default accent is used; when they
  have, the user value wins (preserves today's behavior).

Rationale for a shared crate (vs. a global in `crew-app`): `crew-render` and
`crew-term` do not depend on `crew-app`. The renderer chooses its own
border/clear colors in `scene.rs`, and the terminal owns its ANSI palette in
`crew-term/color.rs`. A global in `crew-app` is unreachable from those crates.
A tiny shared crate lets each crate read `crew_theme::current()` directly,
mirroring how the project already uses a process-global for accent — just
promoted to a place all crates can see.

### Theme fields (centralizes today's scattered constants)

The `Theme` struct must cover every current hardcoded site:

| Field (illustrative) | Replaces (today) |
|---|---|
| `page_bg` | `scene.rs` clear color (#000), `cellgrid.rs` `DEFAULT_BG`, overlay backdrop black |
| `ink` (default fg) | `crew-term/color.rs` `DEFAULT_FG`, `chatlayout.rs` `TEXT_FG`, `inputbar.rs` `TEXT_FG` |
| `term_default_bg` / `term_default_fg` | `crew-term/color.rs` `DEFAULT_BG` / `DEFAULT_FG` |
| `ansi[16]` | `crew-term/color.rs` 16-color ANSI palette |
| `border_normal` / `border_focused` | `scene.rs` `BORDER_NORMAL` / `BORDER_FOCUSED`; `panecard.rs`/`inputbar.rs` `BORDER_ON`/`BORDER_OFF` |
| `legend_off` | `panecard.rs` `LEGEND_OFF` |
| `accent_default` | `palette.rs` `DEFAULT_ACCENT` (per-theme default) |
| `status_fg`, `activity`, `bell`, `broadcast` | `inputbar.rs` `STATUS_FG`, `panecard.rs` status glyph colors |
| `find_hl_bg` | `findhl.rs` `HL_BG` |
| `input_fg`, `hint_fg` | `chatlayout.rs` `INPUT_FG`, `HINT_FG`; `inputbar.rs` |

(Exact field set finalized during implementation; the rule is: every hardcoded
color constant in the listed files becomes a `Theme` field read via
`current()`.)

### Refactor: read colors from `crew_theme::current()`

Replace the hardcoded constants at every site below with reads from the active
theme:

- `crew-render/src/scene.rs` — clear color, `BORDER_NORMAL`, `BORDER_FOCUSED`,
  overlay black backdrop, `DEFAULT_BG` comparison.
- `crew-render/src/cellgrid.rs` — `DEFAULT_BG`.
- `crew-term/src/color.rs` — `DEFAULT_FG`, `DEFAULT_BG`, the 16-color ANSI
  palette resolution.
- `crew-app/src/palette.rs` — accent default becomes theme-derived.
- `crew-app/src/{inputbar,panecard,boxdraw,chatlayout,findhl}.rs` — chrome,
  legends, status glyphs, find highlight.

## The two palettes

Values are illustrative starting points; final hues tuned visually during
implementation. Hard rule: **no pure `#000000` or `#ffffff` anywhere**, and pane
background equals page background in both themes (panes read as the same sheet).

### `paper-light` — warm page

- page bg `#f4f1ea`, ink `#2b2825`
- border `#c9c2b2` → focused `#8c8475`, legend-off `#a89f8d`
- default accent `#9c6b3f` (burnt sienna)
- find highlight `#e8dca8`
- ANSI remap (muted ink): red→brick `#9c3b2e`, green→sage `#5d6b3a`,
  yellow→ochre `#9a7b2e`, blue→faded indigo `#3f5a78`, magenta→mauve `#7d4b6e`,
  cyan→teal `#3f6f6b`, white→warm gray `#5c564b`; bright variants slightly
  lighter but still muted.

### `paper-dark` — warm e-ink night (charcoal-brown, not blue-black)

- page bg `#20201c`, ink `#cfc7b8` (dim parchment)
- border `#4a463d` → focused `#8a8474`, legend-off `#6b6557`
- default accent `#c79a5e` (warm tan)
- find highlight `#4a431f`
- ANSI remap (softened hues on warm charcoal): red `#c06a5a`, green `#9aa76a`,
  yellow `#ccaa6a`, blue `#7d9ab8`, magenta `#b58aa8`, cyan `#7fb0aa`,
  white `#cfc7b8`; brights a touch lighter.

## Switching: `/theme` command + hotkey

- **Command**: `/theme paper-light` / `/theme paper-dark`, parsed alongside the
  existing `/reload` command path (command handling in `crew-app`, near
  `spawn.rs` `apply_config()` / the command dispatcher). Unknown name
  (`/theme foo`) → error shown in the input-bar status line; no state change.
- **Hotkey**: `Ctrl-Shift-L` toggles between the two themes, wired into the
  winit key handler.
- **Live apply**: on any theme change, set the theme index, then mark **all**
  panes dirty and force a full repaint. The terminal default bg/fg changed, so
  every pane must rebuild its cell views — chrome-only repaint is insufficient.

## Config & persistence

- New config field `theme = "paper-dark"` in `~/.config/crew/config.toml`,
  read at startup. Default `paper-dark`. This replaces today's neon-on-black
  default.
- An unknown/invalid `theme` value at startup → fall back to `paper-dark` and
  log a warning (consistent with how bad config values are tolerated today).
- `/theme` and the hotkey write the new choice back to `config.toml` so it
  survives restart.

## Error handling

- `/theme <unknown>` → input-bar status error, no change.
- Invalid `theme` in config → fallback to default + warning.
- Hex parsing for any future per-theme overrides reuses the existing
  `parse_hex()` in `palette.rs`.

## Testing

- **Unit (`crew-theme`)**:
  - No preset contains pure black or pure white (any field).
  - `term_default_fg` vs `term_default_bg` contrast is above a sane floor in
    both presets.
  - `set_theme` / `current()` index round-trip.
  - ANSI palette has exactly 16 entries per preset.
- **Manual / visual**:
  - Toggle live with the hotkey and `/theme`; confirm all panes (including
    running terminal output) repaint immediately.
  - Confirm a user `accent =` override still wins over the theme default.
  - Confirm `/reload` still works and `theme` persists across restart.

## Out of scope (Phase 2 — deferred)

- **Paper texture**: a `paper_texture = false` config flag that, when enabled,
  adds faint procedural grain + edge vignette via a wgpu fragment-shader pass
  behind the text layer. Default off. Requires a shader change and a render-path
  perf check; intentionally NOT part of this plan.
- Any third theme or full user-authored theme files.
- Following the macOS system light/dark appearance automatically.

## Affected files (reference)

New: `crates/crew-theme/` (crate).
Modified: `crates/crew-render/src/{scene,cellgrid}.rs`;
`crates/crew-term/src/color.rs`;
`crates/crew-app/src/{palette,inputbar,panecard,boxdraw,chatlayout,findhl,config}.rs`
plus the command dispatcher and winit key handler in `crew-app`;
workspace `Cargo.toml` (add `crew-theme` member + deps).
