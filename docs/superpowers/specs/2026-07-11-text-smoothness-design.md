# Text Rendering Smoothness — Design

**Date:** 2026-07-11
**Status:** Approved (user: "fonts are smoother in ghostty")

## Problem

crew's text looks thinner/harsher than the same fonts in ghostty. Root cause
hypothesis (verified against glyphon 0.11 source): crew prefers an sRGB
surface (`gpu.rs`) and builds its glyph atlas with `TextAtlas::new`, whose
default `ColorMode::Accurate` blends glyph coverage in **linear** space. Gamma-
space blending — what CoreText, browsers, and most UI toolkits do — renders
antialiased edge pixels darker on light pages and less glowy on dark pages,
which the eye reads as heavier, smoother strokes. Ghostty rasterizes through
CoreText and inherits that look; crew does not.

## Change

Move the whole pipeline to web/native-style (gamma-space) blending:

1. **Surface preference flips** in `crates/crew-render/src/gpu.rs`: prefer a
   NON-sRGB format (`find(|f| !f.is_srgb())`), falling back to whatever the
   platform offers. glyphon's `ColorMode::Web` documents exactly this target:
   "a linear RGB texture containing sRGB colors".
2. **Atlas mode** in `crates/crew-render/src/cellgrid.rs`:
   `TextAtlas::with_color_mode(device, queue, &cache, format, ColorMode::Web)`
   — text colors pass through unconverted; blending happens on gamma-encoded
   values in the fixed-function blend.
3. **No color regressions by construction:** every color already crosses the
   GPU boundary through `color::target_rgba(rgb, a, srgb)` (quads, paper bg,
   clear color) keyed off `format.is_srgb()`, which handles both target kinds
   — flat theme colors stay byte-exact with the surface flag flipped. The plan
   audits the remaining shaders (`quads.wgsl`, `roundborder.wgsl`,
   `paperbg.wgsl`) for any hardcoded sRGB assumption.

No config knob: this replaces the rendering, it doesn't add a mode (YAGNI —
we add a knob only if someone misses linear blending, which nobody has seen
as better for text).

## Verification (the core of this work)

- **Screenshot A/B with the GUI harness** (.claude/skills/verify): same pane
  content, before/after builds, on paper-dark AND paper-light.
  - Flat-color guard: the page-bg pixel and a solid border pixel must be
    byte-identical to the theme's authored values in the after build.
  - Edge sampling: for a known glyph stem, the antialiased edge pixels must
    move toward the ink color (darker midtones on light bg, dimmer halo on
    dark bg) versus the before build.
- **Contrast floors hold:** crew-term's WCAG 3.0 fg/bg floor and the theme
  contrast tests are unaffected (they operate on sRGB values, which are
  unchanged) — run the suites to confirm.
- Unit-testable pieces: surface-format preference picks non-sRGB when the
  caps list offers both orders; `target_rgba` passthrough for srgb=false is
  already pinned by existing tests.

## Contingency (out of scope unless A/B disappoints)

If Web-mode blending alone doesn't close the gap to ghostty, the next
experiment is dark-theme stem weight (base_weight 400→450 where the family
has the axis). Not part of this change.

## Out of scope (YAGNI)

Subpixel (RGB) antialiasing, hinting changes inside cosmic-text/swash,
per-theme blend modes, a user-facing blending toggle.
