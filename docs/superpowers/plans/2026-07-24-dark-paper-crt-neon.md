# Dark Paper & CRT Neon Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dark themes read as dark paper (coarse fiber grain + warm charcoal pages); CRT themes read as neon Tron (wider/stronger phosphor glow + hotter presets).

**Architecture:** All shader work is in `crew-render` (`paperbg.wgsl` second noise octave; `crt.wgsl` second glow ring + raised `GLOW` const), verified by the existing headless-GPU tests extended with new pixel-statistics assertions. All color work is data-only in `crew-theme` presets, vetoed by the existing contrast-threshold tests.

**Tech Stack:** Rust, wgpu/WGSL, headless Metal GPU tests (skip on GPU-less CI).

**Spec:** `docs/superpowers/specs/2026-07-24-dark-paper-crt-neon-design.md`

## Global Constraints

- Workspace root `/Users/atyagi/code/crew`.
- The render surface is NON-sRGB: all amplitudes are gamma-space values; the paper pass writes encoded values directly.
- Scanline period stays exactly 3.0 px (2 px aliases to flat — comment in `crt.wgsl:~95` explains; do not touch).
- Light-page appearance must not change: `paperbg_headless`'s existing Case 1–3 assertions must pass unmodified except where a step below explicitly edits them.
- `Theme.grain` stays `1.2` on every preset (`lib_tests.rs::grain_is_newsprint_on_every_theme` must keep passing).
- Contrast-threshold tests in `crew-theme` are the palette veto: if a retuned color trips one, adjust the color, never the threshold.
- Calibration discipline: shader amplitude constants are set by measuring (run the headless test's printed stats), not by taste; each calibrated constant gets a comment stating the measured value.
- GPU tests: `cargo test -p crew-render --test paperbg_headless -- --nocapture` and `cargo test -p crew-render --test crt_headless`. Theme tests: `cargo test -p crew-theme`.
- Pre-commit runs `cargo fmt --check` + `cargo check`; run `cargo fmt` before every commit.

---

### Task 1: Coarse fiber octave in the paper-grain shader

**Files:**
- Modify: `crates/crew-render/src/paperbg.wgsl`
- Test: `crates/crew-render/tests/paperbg_headless.rs`

**Interfaces:**
- Consumes: existing `Uniform` (unchanged — no new bindings), `grain(px)` hash.
- Produces: dark pages carry two-octave grain; light pages statistically unchanged. No Rust API change.

- [ ] **Step 1: Write the failing test additions**

In `paperbg_headless.rs`, add helpers after `pixel_r`:

```rust
/// Per-pixel R stddev over the flat 24..40 centre block (vignette-flat).
fn centre_stddev(buf: &[u8]) -> f64 {
    let mut vals = Vec::new();
    for y in 24..40 {
        for x in 24..40 {
            vals.push(pixel_r(buf, x, y) as f64);
        }
    }
    let mean = vals.iter().sum::<f64>() / vals.len() as f64;
    (vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / vals.len() as f64).sqrt()
}

/// Stddev of the same block after 2x2 box-downsampling: white noise
/// collapses (~/2), coarse structure survives — the "fiber present" signal.
fn downsampled_stddev(buf: &[u8]) -> f64 {
    let mut vals = Vec::new();
    for y in (24..40).step_by(2) {
        for x in (24..40).step_by(2) {
            let s = pixel_r(buf, x, y) as f64
                + pixel_r(buf, x + 1, y) as f64
                + pixel_r(buf, x, y + 1) as f64
                + pixel_r(buf, x + 1, y + 1) as f64;
            vals.push(s / 4.0);
        }
    }
    let mean = vals.iter().sum::<f64>() / vals.len() as f64;
    (vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / vals.len() as f64).sqrt()
}
```

Then, inside `paperbg_headless()` after the existing Case 3, add Case 4 (dark page, both statistics) — set a dark page the same way Case 1 sets the light one:

```rust
    // Case 4: dark page (8,8,8) at the app's default dark drive
    // (grain_mul = 1.56 = knob 1.3 × theme.grain 1.2): newsprint-band
    // per-pixel spread AND surviving coarse structure after 2x2 downsample.
    let dark_bg = [8.0_f32 / 255.0, 8.0 / 255.0, 8.0 / 255.0, 1.0];
    paper_bg.update(&queue, dark_bg, [64.0, 64.0], 1.0, 1.56);
    let dark_pixels = render_64x64(&device, &queue, &paper_bg);
    let dark_std = centre_stddev(&dark_pixels);
    let dark_coarse = downsampled_stddev(&dark_pixels);
    eprintln!("paperbg_headless dark: std={dark_std:.2} coarse={dark_coarse:.2}");
    assert!(
        (4.0..=9.0).contains(&dark_std),
        "dark grain out of newsprint band: {dark_std:.2}"
    );
    assert!(
        dark_coarse >= dark_std * 0.55,
        "no fiber structure: coarse {dark_coarse:.2} vs fine {dark_std:.2} — \
         pure white noise would collapse to ~0.5x under 2x2 downsampling"
    );

    // Light page must stay fiber-free: downsampled spread collapses like
    // white noise (octave is dark_weight-gated).
    let light_std = centre_stddev(&pixels);
    let light_coarse = downsampled_stddev(&pixels);
    eprintln!("paperbg_headless light: std={light_std:.2} coarse={light_coarse:.2}");
    assert!(
        light_coarse <= light_std * 0.7,
        "light page grew coarse structure: {light_coarse:.2} vs {light_std:.2}"
    );
```

IMPORTANT: `pixels` is Case 1's buffer (light page, grain_mul 1.0) — if it has moved out of scope by this point in the test body, re-render it with Case 1's exact parameters first. Check `PaperBgPass::update`'s real signature in `crates/crew-render/src/paperbg.rs` (the args above follow its doc: bg, resolution, intensity, grain_mul) and adapt argument order to match.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-render --test paperbg_headless -- --nocapture`
Expected: FAIL on the `dark_coarse >= dark_std * 0.55` assertion (current single-octave hash noise collapses under downsampling; observed ratio ≈ 0.5). The band assertion may pass (current calibration ≈ 5.7). If the run skips with "no GPU adapter", STOP and report BLOCKED — this machine should have Metal.

- [ ] **Step 3: Implement the second octave**

In `paperbg.wgsl`, replace the single-sample absolute-term block (the `let n = ...` line and the final `let rgb = ...` clamp) with:

```wgsl
    // Fine octave: per-pixel hash — the newsprint speckle.
    let n = (grain(in.pos.xy) - 0.5) * u.grain_mul * u.intensity;
    // Coarse octave: the same hash on 2.5px-quantized coords — value-noise
    // blotches at paper-fiber scale. Only the dark absolute term uses it
    // (dark_weight-gated below), so light pages keep their pure speckle.
    let n2 = (grain(floor(in.pos.xy / 2.5)) - 0.5) * u.grain_mul * u.intensity;
```

and the combine line becomes:

```wgsl
    let rgb = clamp(
        base * (1.0 + n * 0.05)
            + vec3<f32>((n * 0.5 + n2 * 0.7) * A_DARK * dark_weight),
        vec3<f32>(0.0), vec3<f32>(1.0));
```

with `A_DARK` a literal constant starting at `0.075`. Calibrate: run the headless test with `--nocapture`, read the printed `dark std/coarse`, and adjust `A_DARK` (and if needed the 0.5/0.7 octave weights) until `dark_std` lands in 5–7 and the coarse ratio clears 0.55 with margin (≥ 0.6 observed). Preserve the existing long calibration comment, updating its constants and adding one line stating the measured std/coarse values and the machine (Metal, 64×64 harness). The multiplicative light-page term (`n * 0.05`) must not change.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p crew-render --test paperbg_headless -- --nocapture`
Expected: ALL PASS including untouched Cases 1–3 (light path statistically unchanged; Case 1's exact-value assertions must not need edits — if they fail, the light path changed: fix the shader, not the test).
Then: `cargo test -p crew-render` (whole crate) — PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-render/src/paperbg.wgsl crates/crew-render/tests/paperbg_headless.rs
git commit -m "feat(render): coarse fiber octave gives dark pages a paper texture"
```

---

### Task 2: Warm charcoal dark pages

**Files:**
- Modify: `crates/crew-theme/src/presets_paper.rs` (the `dark: true` presets ONLY)
- Test: existing `crates/crew-theme/src/lib_tests.rs` suite (no new tests; the contrast tests are the gate) plus one new pinning test

**Interfaces:**
- Consumes/Produces: data-only preset edits; no API change. CRT presets and light presets untouched.

- [ ] **Step 1: Write the failing pinning test**

In `lib_tests.rs` add:

```rust
#[test]
fn dark_paper_pages_lean_warm() {
    // Dark non-CRT pages read as warm charcoal/kraft: R strictly above B.
    for id in ThemeId::ALL {
        let t = id.theme();
        if t.dark && !t.crt {
            assert!(
                t.page_bg.0 > t.page_bg.2,
                "{}: page_bg {:?} not warm (R must exceed B)",
                id.as_str(),
                t.page_bg
            );
        }
    }
}
```

Adapt the iteration idiom to whatever `grain_is_newsprint_on_every_theme` uses (same file) — copy its loop shape exactly.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-theme dark_paper_pages_lean_warm`
Expected: FAIL for at least one dark preset (any with R ≤ B page_bg).

- [ ] **Step 3: Retune the dark pages**

For each `dark: true` preset in `presets_paper.rs`: shift `page_bg` (and `term_bg` where it mirrors it) a few steps warm — roughly `+3..6` on R, `-2..4` on B, keeping overall luma within ±2 of the original so grain calibration holds. Example: `(12, 12, 14)` → `(16, 12, 10)`. Keep each preset's character (a bluish slate preset becomes warm slate, not brown). Update each preset's doc comment if it names the page hue.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p crew-theme`
Expected: ALL PASS — the new pinning test AND every contrast-threshold test. If a contrast test fails, reduce the tilt on that preset (the thresholds are the veto; never edit them).

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-theme/src/presets_paper.rs crates/crew-theme/src/lib_tests.rs
git commit -m "feat(theme): dark paper pages lean warm charcoal"
```

---

### Task 3: Wider, stronger phosphor glow

**Files:**
- Modify: `crates/crew-render/src/crt.wgsl`, `crates/crew-render/src/crt.rs` (`GLOW` const)
- Test: `crates/crew-render/tests/crt_headless.rs`

**Interfaces:**
- Consumes: existing `U` uniform (unchanged), existing test harness (`source`, `render`, `r_at`).
- Produces: glow bleeds farther and brighter; `GLOW` const `0.35` → `0.55`. No API change.

- [ ] **Step 1: Write the failing test**

In `crt_headless.rs`, inside the main test after the existing glow assertion (find it — it renders a bright block on dark and asserts neighbours lit), add a reach assertion. Use the existing source/render helpers and the test's existing uniform-setting call (mirror its exact `set_params`/`update` invocation, whatever it is named, with the same scanline/curvature values it already uses):

```rust
    // Neon reach: 3px from the bright block's edge must still visibly glow
    // (the old single 1.5px ring left it dark), while 8px out stays black —
    // the halo is wider, not a wash.
    let near = r_at(&glow_pixels, block_right_x + 3, block_mid_y);
    let far = r_at(&glow_pixels, block_right_x + 8, block_mid_y);
    assert!(near >= 8, "3px halo too weak: {near}");
    assert!(far <= 4, "glow washed out at 8px: {far}");
```

Derive `block_right_x`/`block_mid_y` from the existing test's bright-block geometry (its `source(...)` fill closure defines the block bounds — reuse its coordinates; if the existing test names them differently, use its names). Also update the existing near-neighbour glow assertion's expected minimum upward only if calibration shows it (see Step 3).

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-render --test crt_headless`
Expected: FAIL on `near >= 8` (single 1.5px ring leaves +3px dark). If it skips for lack of GPU, STOP and report BLOCKED.

- [ ] **Step 3: Implement**

In `crt.wgsl`, extend the glow block with a second ring at ~3.5px and rebalance:

```wgsl
    if (u.glow > 0.0) {
        let o = (1.5 / u.resolution);
        let o2 = (3.5 / u.resolution);
        var bloom = vec3<f32>(0.0);
        // inner ring (8 taps at 1.5px) — unchanged tap positions
        ...existing 8 taps into `bloom`...
        var bloom2 = vec3<f32>(0.0);
        bloom2 += textureSample(tex, samp, warped + vec2<f32>( o2.x, 0.0)).rgb;
        bloom2 += textureSample(tex, samp, warped + vec2<f32>(-o2.x, 0.0)).rgb;
        bloom2 += textureSample(tex, samp, warped + vec2<f32>(0.0,  o2.y)).rgb;
        bloom2 += textureSample(tex, samp, warped + vec2<f32>(0.0, -o2.y)).rgb;
        bloom2 += textureSample(tex, samp, warped + vec2<f32>( o2.x,  o2.y)).rgb;
        bloom2 += textureSample(tex, samp, warped + vec2<f32>(-o2.x,  o2.y)).rgb;
        bloom2 += textureSample(tex, samp, warped + vec2<f32>( o2.x, -o2.y)).rgb;
        bloom2 += textureSample(tex, samp, warped + vec2<f32>(-o2.x, -o2.y)).rgb;
        col += bloom * (u.glow / 8.0) + bloom2 * (u.glow * 0.45 / 8.0);
    }
```

(The `...existing 8 taps...` line means: keep the current eight `bloom +=` lines exactly as they are.) In `crt.rs`: `pub const GLOW: f32 = 0.55;` and extend its doc comment: the neon retune (2026-07-24) widened the halo with a second 3.5px ring at 0.45 weight. Update the module doc's "single-pass" wording ("two-ring single-pass bloom").

- [ ] **Step 4: Run the tests**

Run: `cargo test -p crew-render --test crt_headless` then `cargo test -p crew-render`.
Expected: ALL PASS — including the pre-existing scanline, flatness, and flicker-static assertions (the glow change must not disturb them; if the scanline assertion fails, the glow weight is bleeding into row deltas — lower the 0.45 ring weight, re-measure).

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-render/src/crt.wgsl crates/crew-render/src/crt.rs crates/crew-render/tests/crt_headless.rs
git commit -m "feat(render): two-ring phosphor bloom for the neon CRT halo"
```

---

### Task 4: Neon CRT preset retune

**Files:**
- Modify: `crates/crew-theme/src/presets_crt.rs` (all four presets)
- Test: existing `crew-theme` suite + one new pinning test in `lib_tests.rs`

**Interfaces:**
- Data-only. Each preset keeps its phosphor identity (GREEN, AMBER, VIOLET, BLUE).

- [ ] **Step 1: Write the failing pinning test**

In `lib_tests.rs`:

```rust
#[test]
fn crt_pages_are_deep_cool_black() {
    // Neon retune: CRT tubes sit on a darker, cooler near-black so the
    // phosphor halo pops — max page channel ≤ 8, and never warm (R ≤ B+2).
    for id in ThemeId::ALL {
        let t = id.theme();
        if t.crt {
            let (r, g, b) = t.page_bg;
            assert!(
                r.max(g).max(b) <= 8,
                "{}: page_bg {:?} too bright for a neon tube",
                id.as_str(),
                t.page_bg
            );
            assert!(
                r <= b.saturating_add(2),
                "{}: page_bg {:?} warm — CRT pages stay cool",
                id.as_str(),
                t.page_bg
            );
        }
    }
}
```

(Same loop idiom as Task 2's test. Note CRT_AMBER's current `page_bg (14, 8, 2)` violates both clauses — the test must fail before the retune.)

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-theme crt_pages_are_deep_cool_black`
Expected: FAIL (at least CRT_AMBER's `(14, 8, 2)`).

- [ ] **Step 3: Retune the four presets**

For each preset in `presets_crt.rs`:
- `page_bg`/`term_bg`: deep cool near-black satisfying the pinning test while keeping a whisper of the phosphor hue, e.g. GREEN `(3, 10, 5)` → `(2, 6, 5)`; AMBER `(14, 8, 2)` → `(6, 5, 6)`; VIOLET and BLUE analogous (≤ 8 max channel, R ≤ B+2).
- `border_focused`, `accent_default`, `activity`: push saturation/brightness toward full neon (at or near a 255 channel, low off-channels) while keeping the hue family.
- `ink`/`term_fg`: keep or slightly intensify the phosphor hue; do NOT dim.
- Leave `ansi` arrays, `grain: 1.2`, `crt: true`, `border_thickness` untouched unless a contrast test forces an ansi tweak.
- Update each preset's doc comment ("electrified" → describe the Tron-grid look).

- [ ] **Step 4: Run the tests**

Run: `cargo test -p crew-theme`
Expected: ALL PASS — pinning tests AND contrast thresholds (thresholds are the veto: adjust colors, never thresholds). Then `cargo test -p crew-render` once more (no interaction expected; cheap insurance).

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-theme/src/presets_crt.rs crates/crew-theme/src/lib_tests.rs
git commit -m "feat(theme): CRT presets go full neon over deep cool tubes"
```
