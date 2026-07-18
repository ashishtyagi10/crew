# CRT Look & Feel — Design

**Date:** 2026-07-17
**Status:** Implemented (feat/crt-look)

## Goal

A full-tube CRT effect over the terminal — barrel curvature, scanlines,
phosphor glow, corner darkening, and an activity-driven flicker — that turns on
automatically with the CRT phosphor themes and can be overridden with `/crt`.

## Decisions (from brainstorming)

- **Fidelity:** Full tube (curvature included), not just a scanline overlay →
  requires an off-screen render + post-process pass.
- **Trigger:** Auto with the `crt-*` themes via a new `Theme.crt` flag, plus a
  `/crt on|off|auto` override persisted in config.
- **Animation:** Flicker only while output is streaming (rides the existing
  busy-anim redraws); idle is a static tube at zero extra cost.
- **Glow:** Single-pass in-shader neighbour bloom (cheaper than a multi-pass
  bloom chain; reads convincingly for text). Chromatic aberration omitted.

## Architecture

### crew-render
- `SceneTarget` (`scenetarget.rs`): an off-screen colour texture + view, surface
  format, recreated on resize.
- `CrtPass` (`crt.rs` + `crt.wgsl`): fullscreen-triangle post-process. Bind group
  = scene texture + linear sampler + uniform; rebuilt via `set_source` on
  resize. Uniform = `[w, h, time, flicker, curvature, scanline, glow, corner]`.
- `frame.rs`: `encode()` runs the scene pass into `scene_view`, then — when CRT
  is on — the reprojection pass into the surface. CRT off ⇒ `scene_view ==
  surface_view` and no second pass (the original zero-overhead path).
- `Renderer`: holds the pass + target + `crt_on/time/flicker`; setters
  `set_crt`, `set_crt_anim`. Chooses the path each frame.

### Shader (`crt.wgsl`)
Barrel-warp UVs toward centre (outside `[0,1]` ⇒ black bezel) → 8-tap phosphor
glow → scanlines (3-px period; **not** 2 px, which aliases to flat 0.5 at pixel
centres) → corner darkening → flicker (`0` ⇒ static).

### crew-theme
`Theme.crt: bool` next to `dark`/`grain`. Only the `crt-*` presets set it.

### crew-app
- `config.crt: Option<bool>` (None = follow theme).
- `effective_crt() = config.crt.unwrap_or(theme().crt)`, read every frame in the
  `RedrawRequested` handler so it tracks live theme changes.
- Flicker lifts to `0.06` while any pane is animating (`pane_animating`), which
  already drives ~15 fps redraws; otherwise `0.0`.
- `/crt on|off|auto` (bare `/crt` toggles) in `dispatch.rs`, palette entry in
  `cmddefs.rs`, value picker in `suggest.rs`.

## Testing
- **Headless GPU** (`tests/crt_headless.rs`, skips without an adapter): renders a
  known source through `CrtPass` and asserts curvature blacks the corners,
  scanlines make adjacent centre rows differ, glow bleeds a bright block past
  its edge, and `flicker=0` is byte-for-byte static.
- **Theme:** only the `crt-*` presets carry `crt=true`.
- **Config:** `crt` round-trips through TOML.
- **Command:** `/crt on|off|auto`, bare-toggle, and unknown-arg behaviour.
