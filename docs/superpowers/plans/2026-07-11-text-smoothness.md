# Text Rendering Smoothness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move crew's rendering to web/native-style (gamma-space) blending so text strokes read heavier and smoother, matching what CoreText apps (ghostty) look like.

**Architecture:** Flip the surface-format preference to non-sRGB in `gpu.rs` (a pure, testable picker), and key glyphon's atlas `ColorMode` off the actually-chosen format in `cellgrid.rs` (`Web` for non-sRGB targets, `Accurate` when only sRGB exists). All flat colors already route through `color::target_rgba(rgb, a, srgb)`, which handles both target kinds — no other color path changes.

**Tech Stack:** Rust, wgpu 27, glyphon 0.11 (`TextAtlas::with_color_mode`, `ColorMode::{Web,Accurate}`).

## Global Constraints

- Zero `cargo check` warnings; rustfmt clean (pre-commit hook enforces).
- No user-facing config knob: this replaces the rendering (spec: "No config knob").
- Flat theme colors must stay byte-exact on screen: every color entering the GPU goes through `crate::color::target_rgba(rgb, alpha, srgb)` with the real `format.is_srgb()` flag — never a raw `c/255` to an sRGB target, never a linearized value to a non-sRGB target.
- The shaders (`quads.wgsl`, `roundborder.wgsl`, `paperbg.wgsl`) contain no color-space math and must not gain any.

---

### Task 1: Non-sRGB surface preference + format-keyed atlas color mode

**Files:**
- Modify: `crates/crew-render/src/gpu.rs` (format selection, lines 29-38)
- Modify: `crates/crew-render/src/cellgrid.rs` (atlas construction, lines 1, 66)

**Interfaces:**
- Produces: `pub(crate) fn pick_surface_format(formats: &[wgpu::TextureFormat]) -> wgpu::TextureFormat` in `gpu.rs`; `pub(crate) fn atlas_color_mode(srgb: bool) -> glyphon::ColorMode` in `cellgrid.rs`. `CellGrid::new` and `Gpu::new` signatures unchanged.
- Consumes: nothing from other tasks.

- [ ] **Step 1: Write the failing tests**

In `crates/crew-render/src/gpu.rs`, add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::TextureFormat as F;

    #[test]
    fn prefers_a_non_srgb_format_for_gamma_space_blending() {
        // Whatever order the platform lists them, non-sRGB wins.
        assert_eq!(
            pick_surface_format(&[F::Bgra8UnormSrgb, F::Bgra8Unorm]),
            F::Bgra8Unorm
        );
        assert_eq!(
            pick_surface_format(&[F::Bgra8Unorm, F::Bgra8UnormSrgb]),
            F::Bgra8Unorm
        );
    }

    #[test]
    fn falls_back_to_the_first_format_when_all_are_srgb() {
        assert_eq!(
            pick_surface_format(&[F::Bgra8UnormSrgb, F::Rgba8UnormSrgb]),
            F::Bgra8UnormSrgb
        );
    }
}
```

In `crates/crew-render/src/cellgrid.rs`, add at the bottom (the file has no test module yet):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_color_mode_matches_the_target_kind() {
        // Non-sRGB target → Web (gamma-space blending, sRGB values pass through).
        // sRGB-only platform → Accurate (values linearized; never wash out).
        assert_eq!(atlas_color_mode(false), glyphon::ColorMode::Web);
        assert_eq!(atlas_color_mode(true), glyphon::ColorMode::Accurate);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-render pick_surface_format; cargo test -p crew-render atlas_color_mode`
Expected: compile FAIL — `pick_surface_format` / `atlas_color_mode` not found.

- [ ] **Step 3: Implement**

In `crates/crew-render/src/gpu.rs`, replace the format selection in `Gpu::new` (currently the `let format = caps.formats...find(|f| f.is_srgb())...` block, lines 33-38) with:

```rust
        let format = pick_surface_format(&caps.formats);
```

and add above `impl Gpu` (below the struct):

```rust
/// Prefer a NON-sRGB surface so alpha blending happens on gamma-encoded
/// values — the web/CoreText look; glyph antialiasing reads heavier and
/// smoother (glyphon's `ColorMode::Web` documents exactly this target).
/// Colours are still fed via `color::target_rgba`, keyed off the format, so
/// flat theme colours stay byte-exact either way. Falls back to whatever the
/// platform offers when everything is sRGB.
pub(crate) fn pick_surface_format(formats: &[wgpu::TextureFormat]) -> wgpu::TextureFormat {
    formats
        .iter()
        .copied()
        .find(|f| !f.is_srgb())
        .unwrap_or(formats[0])
}
```

Also update the now-stale comment above the old block (delete the "Prefer an sRGB surface..." comment — the new doc comment replaces it).

In `crates/crew-render/src/cellgrid.rs`:

Line 1, extend the glyphon import:

```rust
use glyphon::{Cache, ColorMode, FontSystem, Resolution, SwashCache, TextAtlas, TextRenderer, Viewport};
```

Replace line 66 (`let mut atlas = TextAtlas::new(device, queue, &cache, format);`) with:

```rust
        let mut atlas =
            TextAtlas::with_color_mode(device, queue, &cache, format, atlas_color_mode(format.is_srgb()));
```

and add near `default_bg()` (top of file, after imports):

```rust
/// Glyph blending mode for a target of the given sRGB-ness. Non-sRGB targets
/// get `Web`: sRGB text colours pass through unconverted and the fixed-function
/// blend operates on gamma-encoded values — the browser/CoreText look the
/// smoothness work targets. If a platform only offers sRGB surfaces, `Accurate`
/// keeps colours correct (Web mode on an sRGB target would double-encode).
pub(crate) fn atlas_color_mode(srgb: bool) -> ColorMode {
    if srgb {
        ColorMode::Accurate
    } else {
        ColorMode::Web
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-render`
Expected: PASS, including the two new tests and every existing crew-render test.

- [ ] **Step 5: Full gate**

Run: `cargo check -p crew-render -p crew-app 2>&1 | grep -c warning` → expected `0`; `cargo fmt --check`; `cargo test -p crew-term` (contrast-floor suite untouched but must stay green).
Expected: all clean.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-render/src/gpu.rs crates/crew-render/src/cellgrid.rs
git commit -m "feat(render): gamma-space text blending — non-sRGB surface + glyphon Web mode"
```

---

### Task 2: Screenshot A/B verification (controller-run, GUI harness)

**Files:**
- None modified — verification only, using `.claude/skills/verify` (isolated-HOME dev instance, PID guard, `screencapture` + pixel sampling).

**Interfaces:**
- Consumes: the Task 1 commit (after build) and the pre-change build (`git stash` / previous binary) for the A/B.

This task is executed by the session controller (not a subagent) because it drives the live GUI harness:

- [ ] **Step 1: Build both binaries** — `cargo build -p crew-app` at Task 1's commit and at its parent; keep both `target/debug/crew` copies in the scratchpad.
- [ ] **Step 2: For each build × {paper-dark, paper-light}:** launch isolated (`HOME="$SCRATCH/home"`, fonts + `.local/bin` symlinks per the verify skill), open a pane with known text, screenshot.
- [ ] **Step 3: Flat-color guard:** sample the page-bg pixel (empty area) in the after build — must equal the theme's authored `page_bg` bytes exactly (paper-dark `(8,8,8)`, paper-light `(246,243,236)`).
- [ ] **Step 4: Edge sampling:** for the same glyph stem in before/after, sample the antialiased edge pixels — after-build midtones must sit closer to the ink colour (darker on paper-light, less bright halo on paper-dark).
- [ ] **Step 5: Eyeball** the paper grain + vignette (space change shifts them subtly; confirm no banding or washout), and record the verdict + screenshot paths in the progress ledger.

---

## Self-Review Notes

- Spec coverage: surface flip (Task 1/gpu.rs), atlas mode (Task 1/cellgrid.rs), "no config knob" (none added), shader audit (done at plan time: no color-space math in any .wgsl — constraint pins it), flat-color guard + edge sampling + contrast suite (Task 2 / Task 1 Step 5).
- The `ColorMode` import must not break the `glyphon::cosmic_text` import in celltext.rs — untouched.
- Fallback correctness: sRGB-only platforms keep today's exact behavior (`Accurate` + linearized colors).
