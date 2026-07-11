# Font Rotation (`/font random`) — Design

**Date:** 2026-07-11
**Status:** Approved

## Goal

`/font random` rotates the UI font every 10 minutes through the installed,
verified-monospace families — same cadence as theme rotation. A manual font
pick turns it off. Survives restart.

## Design

- **State on `CrewApp`** (font family is app/renderer state, not crew-theme):
  `font_random: bool` (mirrored to config) and `font_rotated_ms: u64`.
  Reuse `crew_theme::ROTATE_MS` for the interval — one shared "rotation
  minute-hand" constant.
- **Pool:** `renderer.monospace_families()` — already verified fixed-pitch.
  Scanned once on first need and cached on the app (`font_pool:
  Option<Vec<String>>`); the scan loads faces, so it runs at most once per
  session (same call the settings pane already makes). Pool excludes the
  current family at pick time; a pool of ≤1 makes `/font random` report
  "only one monospace font installed" and stay off.
- **Pick:** deterministic-from-timestamp hash, same recipe as
  `crew_theme::random_pick` (no rand dependency).
- **Tick:** in `poll_panes` beside `tick_random`: when `font_random` and
  `ROTATE_MS` elapsed → pick, `renderer.set_font_family(...)`, and track the
  pick in an app-side `rotated_family: Option<String>` — `config.font_family`
  is NEVER touched by rotation (unrelated `config.save()` calls — e.g. the
  window-resize settle — must not persist a rotated pick; restart returns to
  the pinned family). Status flash `font → <family>`, redraw. Cell metrics
  are fixed per font size, so the grid never moves on swap; the celltext
  correction cache is keyed per family and handles narrow-glyph fallbacks
  per font. Cell metrics are fixed
  per font size, so the grid never moves on swap; the celltext correction
  cache is keyed per family and handles narrow-glyph fallbacks per font.
- **Commands** (`fontcmd.rs`): `/font random` enables (immediate first pick,
  so the effect is visible — mirroring `set_random`); `/font <n>` still sets
  size; `/font` no-arg reports size + rotation state + current family. Any
  explicit family selection (settings pane path that calls
  `set_font_family`) sets `font_random = false`.
- **Config:** `font_random: bool` (default false), saved with the config;
  startup with `font_random: true` starts rotating from the configured
  family (first rotation after ROTATE_MS — no jarring swap at launch).

## Testing

- Pick: never returns current; deterministic for a seed; ≤1-family pool
  reports and stays off.
- Tick gating: no rotation before ROTATE_MS; rotation updates family +
  timestamp + returns redraw.
- Command parse: `/font random` toggles on, manual family selection toggles
  off, `/font` no-arg mentions rotation state.
- Config round-trip for `font_random`.

## Out of scope (YAGNI)

Per-pane fonts, user-curated rotation lists, rotation intervals other than
the shared 10 minutes, font previews.
