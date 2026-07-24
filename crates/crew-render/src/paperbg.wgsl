struct Uniform {
    page_bg: vec4<f32>,
    resolution: vec2<f32>,
    intensity: f32,
    grain_mul: f32,   // scales additive grain amplitude (0 = no grain, 1 = default)
}
@group(0) @binding(0) var<uniform> u: Uniform;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
}

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    // Fullscreen triangle — covers the entire NDC cube with 3 vertices, no VB.
    var pts = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var out: VsOut;
    out.pos = vec4<f32>(pts[vi], 0.0, 1.0);
    return out;
}

// Deterministic per-pixel luminance hash — pure function of pixel coordinates.
fn grain(px: vec2<f32>) -> f32 {
    return fract(sin(dot(px, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    // UV in [0, 1] with (0,0) at top-left.
    let uv = in.pos.xy / u.resolution;

    // Radial vignette: ~5% darker at corners (d2 = 0.5 at corner → 0.95).
    // Multiplicative on the page colour, so it scales with brightness.
    let d2 = dot(uv - vec2<f32>(0.5), uv - vec2<f32>(0.5));
    let vignette = 1.0 - d2 * 0.1;
    let base = u.page_bg.rgb * (vignette * u.intensity + (1.0 - u.intensity));

    // Fine octave: per-pixel hash — the newsprint speckle. Scaled by
    // grain_mul and gated by intensity.
    let n = (grain(in.pos.xy) - 0.5) * u.grain_mul * u.intensity;
    // Coarse octave: the same hash on 2.5px-quantized coords — value-noise
    // blotches at paper-fiber scale. Only the dark absolute term uses it
    // (dark_weight-gated below), so light pages keep their pure speckle and
    // the multiplicative light-page term (n * 0.05) is untouched.
    let n2 = (grain(floor(in.pos.xy / 2.5)) - 0.5) * u.grain_mul * u.intensity;
    // Hybrid grain so the texture reads on BOTH themes: a multiplicative term
    // gives the bright "paper" page its grain (an absolute term would be
    // imperceptible there), and a small absolute term gives the near-black
    // "newspaper" page visible texture (a purely multiplicative grain vanishes
    // on it).
    //
    // Gamma-space tuning: this pass now writes directly to a non-sRGB target
    // (see gpu.rs `pick_surface_format` — glyphon ColorMode::Web needs gamma-
    // space blending), so there is no sRGB encode gain on write. The old
    // 0.0015 absolute amplitude was tuned for a LINEAR page colour headed to
    // an sRGB target, where near-black values gained ~13x on encode; on the
    // non-sRGB path that gain is gone, so the same constant read as
    // essentially flat on dark pages.
    //
    // 0.048 gave the near-black "newspaper" page the SAME newsprint spread
    // the bright paper page has (light-page std ≈ 5.67), so the dark themes
    // read as textured newsprint rather than flat black — the deliberate
    // choice after dark themes previously kept a much subtler ~±3-level grain.
    // Dark themes now also carry theme.grain 1.2 (matching light), so the two
    // appearances share one grain identity. Measured by rendering page_bg
    // (8,8,8) at grain_mul 1.56 (knob default 1.3 × theme.grain 1.2) and
    // sampling per-pixel R stddev over a flat center region.
    //
    // The absolute term is weighted down by page brightness (`dark_weight`)
    // so it stays negligible on light pages — without this, the constant
    // would stack on the already-calibrated light-page multiplicative grain,
    // since the absolute and multiplicative terms share the same noise sample
    // and add. `dark_weight` ≈ 1 near black and ≈ 0.05 on the paper-light
    // page_bg, keeping the light-page spread essentially unchanged.
    //
    // Coarse fiber octave: the single-octave grain above read as pure
    // per-pixel speckle on dark pages, not paper fiber — `n2` above is the
    // same hash sampled on 2.5px-quantized coordinates, so it produces
    // value-noise blotches a few pixels wide instead of independent
    // per-pixel noise. It is blended into the SAME dark_weight-gated
    // absolute term as `n`, so light pages (dark_weight ≈ 0.05) stay
    // statistically unchanged; the light-page multiplicative term
    // (n * 0.05 above) is untouched. `A_DARK` replaces the old bare 0.048
    // absolute-amplitude literal; the fine/coarse weights 0.5/0.7 split the
    // dark-page grain energy between the two octaves, coarse weighted
    // slightly higher since it is what reads as "fiber" after downsampling.
    // Calibrated with the headless 64x64 Metal harness
    // (crates/crew-render/tests/paperbg_headless.rs), rendering the same
    // (8,8,8)/grain_mul-1.56 page as above and reading both per-pixel stddev
    // (`dark_std`, target band 5-7) and the stddev after a 2x2 box-downsample
    // (`dark_coarse`, target ratio dark_coarse/dark_std >= 0.6 — pure white
    // noise collapses to ~0.5x under 2x2 averaging, so clearing that floor
    // with margin is the "fiber survives downsampling" signal). Pre-octave
    // baseline (single hash sample, old 0.048 literal): dark_std=5.19,
    // dark_coarse=2.16 (ratio 0.42 — fails the floor, reads as speckle not
    // fiber). A_DARK=0.075 with fine/coarse weights 0.5/0.7 landed both
    // targets on the first try, so no further iteration was needed: measured
    // dark_std=6.40 (in the 5-7 band), dark_coarse=4.19 (ratio 0.65, clears
    // the 0.6 margin). Light page (Cases 1-3, grain_mul<=2.4) statistically
    // unaffected: light_std=3.56, light_coarse=1.49 (ratio 0.42, well under
    // the 0.7 white-noise ceiling — dark_weight keeps n2 negligible there).
    const A_DARK: f32 = 0.075;
    let page_luma = dot(u.page_bg.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let dark_weight = 1.0 - page_luma;
    let rgb = clamp(
        base * (1.0 + n * 0.05)
            + vec3<f32>((n * 0.5 + n2 * 0.7) * A_DARK * dark_weight),
        vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(rgb, 1.0);
}
