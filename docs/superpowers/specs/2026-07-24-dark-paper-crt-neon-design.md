# Dark paper & CRT neon — design

**Date:** 2026-07-24
**User ask:** Light themes read as paper (visible grain); dark themes read as
speckled black, not "dark paper"; CRT themes should lean harder into a
neon/Tron look.

## Background

- Grain lives in `crew-render/src/paperbg.wgsl`: one per-pixel hash octave,
  hybrid multiplicative + absolute term. v0.5.58 calibrated the dark
  absolute amplitude (0.048) so dark per-pixel stddev matches light
  (≈ 5.67). Equal amplitude ≠ equal look: 1px hash noise reads as speckle;
  paper reads as multi-scale fiber.
- CRT look is a post-pass (`crew-render/src/crt.wgsl`): curvature, 3px
  scanlines (2px aliases — keep 3px), single 8-tap glow ring, flicker.
  Four CRT presets in `crew-theme/src/presets_crt.rs`.
- `contrast_thresholds` in crew-theme is the palette arbiter; any color
  retune must keep its tests green. The surface is NON-sRGB (gamma-space
  blending) — all amplitudes are gamma-space values.

## Part 1: dark paper

1. **Second noise octave** in `paperbg.wgsl`: the same hash sampled on
   `floor(px / 2.5)` coordinates — 2–3px value-noise blotches, no textures,
   no extra bindings. Blended into the absolute term only (dark-weighted):
   fine speckle stays, fiber-scale structure appears on dark pages; light
   pages unchanged (`dark_weight ≈ 0.05` there).
2. **Amplitude split:** absolute term becomes
   `(0.5 * n_fine + 0.7 * n_coarse) * A * dark_weight` with `A` recalibrated
   so the dark page's per-pixel stddev stays in the newsprint band
   (≈ 5–7), while a 2×2 box-downsampled stddev — which collapses white
   noise but preserves coarse structure — lands meaningfully above the
   light page's (the "fiber present" signal). Exact constants come from the
   pixel-sampling harness, not from taste.
3. **Warm dark pages:** the dark (non-CRT) presets' `page_bg` tilt a few
   steps toward warm charcoal/kraft (+R, −B, small). CRT pages are NOT
   warmed (Part 2 pushes them cooler). Contrast-threshold tests are the
   veto: a tilt that trips a floor is reduced, not forced.

## Part 2: CRT neon

1. **Glow:** second sample ring (8 more taps at ~2× offset, lower weight)
   and a higher glow coefficient, per-theme-controllable via the existing
   glow uniform. Scanlines stay 3px. Flicker untouched.
2. **Preset retune** (`presets_crt.rs`, all four — GREEN, AMBER, VIOLET,
   BLUE): each preset keeps its phosphor identity but intensifies —
   borders/accents pushed hotter and more saturated, `page_bg` pushed to a
   darker, cooler near-black so halos pop. Grain stays 1.2. Glow strength
   itself is the pass's global `GLOW` const (raised in §1) — no new
   per-theme fields (YAGNI).

## Testing

- Headless GPU harness (existing pattern): render a flat dark page →
  assert per-pixel stddev in band AND 2×2-downsampled stddev above a floor;
  render the light page → assert its downsampled stddev stays below that
  floor (no fiber on light) and its raw stddev is unchanged from today's
  calibration.
- CRT: extend the existing headless CRT test — a lit glyph's surrounding
  ring luminance must exceed the pre-change pass's (wider halo), scanline
  period still 3px.
- crew-theme: preset contrast tests unchanged and green after retunes;
  grain field stays 1.2 everywhere (existing test).

## Non-goals

Border/ribbon glow (the "full Tron kit" option was declined), light-theme
changes, new config knobs, texture assets.

## Sequencing

Queued behind `feat/smith-attach-popup`; implemented on its own branch
(`feat/dark-paper-crt-neon`) after that merges.
