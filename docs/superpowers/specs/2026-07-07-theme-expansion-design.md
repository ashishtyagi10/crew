# Theme Expansion: Four Dark Themes, Dark-Only Random, Light-Theme Ink Weight & Newsprint Grain

**Date:** 2026-07-07
**Status:** Approved for planning

## Summary

Four additions to crew's theme system, all driven by new per-theme data fields:

1. Four new dark themes — `sepia-dark`, `midnight-ink`, `graphite` (paper family) and `crt-violet` (CRT family).
2. Random-rotation mode picks **dark themes only** — it never rotates into a light theme.
3. Regular text renders at **Medium (500)** weight when a light theme is active (dark themes keep Normal 400; explicitly bold cells stay Bold 700 everywhere).
4. Light themes get a **~3× newsprint grain** from the existing paper-texture pass; dark themes keep today's subtle grain.

## Mechanism (decision)

**Explicit per-theme fields, not luminance heuristics.** The `Theme` struct in
`crates/crew-theme/src/lib.rs` gains:

```rust
/// Whether this is a dark theme (dark page, light ink). Drives the random-
/// rotation pool, the light-theme text weight, and the grain multiplier.
pub dark: bool,
/// Grain amplitude multiplier for the paper-texture pass, relative to the
/// user's configured `paper_grain`. 1.0 for dark themes; 3.0 for light
/// themes (visible newsprint).
pub grain: f32,
```

Both are static design data set on each preset. A unit test asserts `dark`
agrees with `page_bg`'s WCAG relative luminance (< 0.5 ⇒ dark), so the flag
cannot drift from the palette. `ThemeId` gains a convenience
`pub fn is_dark(self) -> bool` that reads `self.theme().dark`.

Rationale: themes are design data; intent should be literal and greppable. A
derived-luminance approach misclassifies future mid-gray themes silently and
gives no place to hang per-theme tuning (e.g. a dark theme that wants more
grain later).

## 1. Four new dark themes

New `ThemeId` variants and `&'static Theme` presets, registered in
`ALL_THEMES` (cycle order below), `as_str`/`from_name`, `describe`, `theme`,
and the `as_u8`/`from_u8` persistence mapping (append after the existing 0–4
ids so saved configs keep meaning: sepia-dark=5, midnight-ink=6, graphite=7,
crt-violet=8).

| id | name | family | mood |
|---|---|---|---|
| 5 | `sepia-dark` | paper | dark coffee-brown page, warm cream ink |
| 6 | `midnight-ink` | paper | deep navy page, cool off-white ink |
| 7 | `graphite` | paper | neutral charcoal page, soft white ink — gentler paper-dark |
| 8 | `crt-violet` | CRT | neon violet phosphor on a near-black tube |

`ALL_THEMES` cycle order groups families:
`paper-dark, paper-light, sepia-dark, midnight-ink, graphite, crt-green,
crt-amber, crt-blue, crt-violet`.

Every new palette must pass the existing contrast gates in
`contrast_thresholds` (ink ≥ 10:1 on page, term_fg ≥ 10:1, text_muted ≥ 7:1,
legend_off ≥ 3:1, accent ≥ 3:1, border_focused ≥ 2.2, border_normal ≥ 1.45,
colour ANSI slots ≥ 3:1 on term_bg) plus `no_preset_uses_pure_black_or_white`
and `term_bg_equals_page_bg`. Exact RGB values are an implementation-plan
deliverable, authored against these gates. New paper-family darks follow
paper-dark's focus-led border hierarchy (unfocused borders sit back);
crt-violet follows the CRT presets' conventions (monochrome-tinted ANSI ramp,
border_thickness 2.5; paper darks use paper-dark's 2.5).

Everything downstream — Ctrl+Shift+L cycle, `/theme` value picker and ghost
completion, chat `/theme` list, config round-trip — derives from `ALL_THEMES`
and the name mapping, so no other surface needs bespoke changes beyond tests.

## 2. Random mode rotates dark themes only

In `crew-theme`:

- `random_pick(current, seed)` filters the pool to `t.is_dark() && t != current`.
- Behavior preserved: enabling random switches immediately to a (dark) theme
  and starts the 10-minute clock; `tick_random` cadence unchanged.
- Edge cases: enabling random while on `paper-light` picks from all 8 dark
  themes (current is not in the pool anyway since it isn't dark). The pool is
  never empty (8 dark themes; ≥ 1 even excluding current).
- `cycle_next` (Ctrl+Shift+L) is untouched — it still walks ALL themes
  including light, then random, then wraps.

Tests: for every starting theme and a seed sweep, `random_pick` never returns
a light theme and never returns `current`; determinism per seed retained.

## 3. Medium-weight text on light themes

In `crew-render`:

- `FontParams` (celltext.rs) gains `pub weight: u16` — the base weight for
  non-bold text (cosmic-text `Weight(n)`).
- `fill_rich_text` uses `Weight(params.weight)` for the default attrs and for
  non-bold styled runs; bold cells keep `Weight::BOLD`. (`RunKey` and span
  coalescing are unchanged — weight is uniform per frame, not per cell.)
- `pane_sig` (scenecache.rs) hashes the new field so a theme switch that
  changes weight rebuilds shaped buffers instead of serving stale glyphs.

`FontParams` is built per frame in `CellGrid::set_scene` (cellgrid.rs), which
lives in crew-render — a crate that already depends on crew-theme and reads
`crew_theme::theme()` per frame (renderer.rs reads `page_bg` there). The
weight follows that existing pattern: `weight = if crew_theme::theme().dark
{ 400 } else { 500 }` at `FontParams` construction. Random-rotation and live
`/theme` switches pick the weight up with no extra plumbing, and no crew-app
change is needed.

Font-fallback note: if the active family has no Medium face, cosmic-text
resolves to the nearest available weight — worst case identical to today,
never an error. Cell metrics are unaffected: glyph advances are snapped to the
fixed cell box (`set_monospace_width`), so a heavier face cannot shift the
grid.

## 4. Newsprint grain on light themes

- No shader changes. The existing `paperbg` pass's hybrid grain already scales
  multiplicatively on bright pages; only the amplitude fed to it changes.
- **Effective grain** = `config.paper_grain × theme().grain`, computed where
  the grain uniform is written each frame: `Renderer::frame` (renderer.rs)
  already reads `crew_theme::theme()` there for `page_bg`, and now passes
  `self.paper_grain * crew_theme::theme().grain` to the paperbg uniform. The
  app's resume-time `set_paper_grain(config.paper_grain)` call is unchanged
  (it stores the user knob); `/theme` switches and random rotation are picked
  up automatically frame-by-frame.
- `Theme.grain` values: 3.0 on `paper-light`; 1.0 on every dark theme.
- Config semantics preserved: `paper_texture=false` disables the whole pass;
  `paper_grain=0.0` disables grain everywhere (0 × anything = 0); the config
  clamp stays 0.0–2.0 on the **user** knob — the theme multiplier applies
  after the clamp by design (effective amplitude up to 6.0 at max user grain).

## File structure

`crates/crew-theme/src/lib.rs` (714 lines, ~250 of which are inline tests)
splits as part of this work:

- `lib.rs` — `Theme` struct, `ThemeId` enum + mappings, runtime state
  (current/random/cycle), re-exports of all presets.
- `presets_paper.rs` — PAPER_DARK, PAPER_LIGHT, SEPIA_DARK, MIDNIGHT_INK,
  GRAPHITE.
- `presets_crt.rs` — CRT_GREEN, CRT_AMBER, CRT_BLUE, CRT_VIOLET.
- `lib_tests.rs` — the test module, extended for the new behavior.

No new dependencies anywhere. `crew-render` already depends on `crew-theme`
(renderer.rs reads the active theme per frame for `page_bg`); the weight and
grain reads follow that established pattern rather than adding new plumbing
through crew-app.

## Error handling

There are no new runtime failure modes: all inputs are static data or already-
clamped config values. Unknown theme names in config/`/theme` fail exactly as
today (`from_name → None` → error status / default).

## Testing

- **crew-theme:** dark-flag/luminance consistency for every preset; contrast
  gates green across all 9 themes; random pool excludes light themes (seed
  sweep × every starting theme); u8 mapping round-trips all 9 ids; cycle
  walks all 9 then random then wraps; grain values (light=3.0, darks=1.0).
- **crew-render:** `pane_sig` differs when only `weight` differs; buffers
  built at weight 500 request Medium attrs (span attr assertion, mirroring
  the existing bold test); bold cells still map to 700 regardless of base.
- **crew-app:** weight/grain selection per theme (`dark → 400/1.0`,
  `light → 500/3.0×config`); suggest/chattheme test lists updated for the new
  names.
- **Visual:** screenshot harness pass on paper-light before/after for the
  grain and weight change (pixel-sample sanity, per the sRGB lesson).

## Out of scope

- No new light themes; no per-theme font families or sizes.
- No shader/noise-algorithm changes (fiber flecks were considered and
  declined).
- No change to the Ctrl+Shift+L cycle semantics or the 10-minute rotation
  interval.
