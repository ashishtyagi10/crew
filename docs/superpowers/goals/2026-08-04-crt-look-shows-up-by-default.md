# Goal — the CRT theme LOOKS like a CRT the moment you pick it

**Status: COMPLETE** — shipped v0.12.6: a theme switch clears stale `crt`/`glass` pins and a
one-shot upgrade heals the configs that already had them.

**Set:** 2026-08-04 by the user, after reporting — for the third time — that the CRT theme is
"just a dark theme with some colors": no glow, no transparency, fonts not smooth.

Every one of those features EXISTS and SHIPPED (v0.12.2–v0.12.5: per-theme `CrtStyle`, half-res
bloom, luminous glass, unhinted glyphs + stem darkening). The user still saw none of them. That gap
— feature shipped, look absent — is the goal. Two root causes, both now understood:

**1. Stale pins outlive theme switches.** A bare `/crt` toggle persists `crt = false`; Settings can
persist `glass = "off"`. Neither is ever cleared again — `set_theme_cmd` switched the palette and
left the pins standing. `theme = "crt"` + `crt = false` + `glass = "off"` is *literally* "a dark
theme with some colors": the phosphor palette with the entire post-process and glass sheet gutted.
One accidental toggle months ago silently disabled every CRT feature forever, with nothing in the
UI saying so.

**2. The vendored glyphon patch covered one of TWO materialization sites.** The v0.12.5 smoothing
pass seeds stem-darkened bitmaps into `SwashCache.image_cache`, and `text_render.rs` was patched to
read through it — but `text_atlas.rs::grow()` still called `get_image_uncached`. The atlas STARTS
at 256² and a Retina terminal frame overflows that on the first frame, so the grow re-uploaded
every glyph UNSMOOTHED — and 2px smaller than its packed rect (smoothing pads 1px per side), so
also edge-clipped and 1px misplaced. Smoothing was silently reverted seconds into every session.
This is why "fix font rendering" kept being requested against a feature that kept being "done".

### The contract (shipped in 0.12.6)
- **Choosing a theme is a statement of intent.** `/theme <x>` and a composer theme switch clear the
  look-killing overrides: any `/crt` pin returns to auto (follow the theme), `glass = "off"`
  returns to the frosted default. A deliberate `low`/`high` glass strength is taste, not a kill
  switch — it survives. (`CrewConfig::reset_look_overrides`, wired in `set_theme_cmd` +
  `ChatAction::PersistTheme`.)
- **One-shot heal on upgrade across 0.12.6** (`handler.rs`, gated on `version_lt(last_seen,
  "0.12.6")`): existing configs carrying the pins are cleared once, before the first frame renders.
  Anyone who truly wants the effects off is one `/crt off` away — and now it only lasts until they
  next pick a theme.
- **The atlas grow path re-uploads the seeded bitmaps** (`vendor/glyphon/src/text_atlas.rs`,
  mirroring the `text_render.rs` read). Both patch sites are documented in the root `Cargo.toml`;
  a vendored-glyphon upgrade must re-apply BOTH.

### Still open (the Ghostty/Warp gap, ranked)
1. **Pixel-snap pane rects** (`layout.rs` computes raw-float origins): glyphs currently land in up
   to 4 subpixel bins per character — 4× atlas entries, 4× grow churn, and border strokes off the
   pixel grid. Snapping origins collapses the bins and sharpens every hairline.
2. **Settings pane has no Font Smoothing field** — the flagship v0.12.5 feature is `/smooth`-only.
3. **Real frosted glass is unimplemented**: crew's glass is tinted alpha, not a backdrop blur.
   Warp-style frost needs an `NSVisualEffectView` behind the wgpu layer (or a self-blur pass).
   Window opacity < 1.0 stays opt-in taste (Settings → WINDOW), and that default is correct.
4. **Atlas prewarm / larger `INITIAL_SIZE`** — 256² holds ~60 Retina glyphs; even correctly-patched
   grows are churn worth avoiding.
5. `curvature: 0.0, corner: 0.0` on all four CRT presets is deliberate (flat tube) — revisit only
   if the look still reads too subtle with everything above actually rendering.
