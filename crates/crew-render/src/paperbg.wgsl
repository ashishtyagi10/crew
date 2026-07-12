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

    // One coherent noise sample, scaled by grain_mul and gated by intensity.
    let n = (grain(in.pos.xy) - 0.5) * u.grain_mul * u.intensity;
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
    // essentially flat on dark pages. 0.026 restores the pre-change ~±3-level
    // (std ≈ 2.6) spread on a near-black page — measured by rendering
    // page_bg (8,8,8) at grain_mul 1.3 (knob default 1.3 × theme.grain 1.0)
    // and sampling per-pixel R stddev over a flat center region.
    //
    // The absolute term is weighted down by page brightness (`dark_weight`)
    // so it stays negligible on light pages — without this, the larger
    // constant would roughly double the already-calibrated light-page grain
    // (theme.grain 1.2), since the absolute and multiplicative terms share
    // the same noise sample and add. `dark_weight` ≈ 1 near black and ≈ 0.05
    // on the paper-light page_bg, keeping light-page spread within ~1% of
    // its pre-F1 measurement (measured std 5.693 → 5.667).
    let page_luma = dot(u.page_bg.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let dark_weight = 1.0 - page_luma;
    let rgb = clamp(base * (1.0 + n * 0.05) + vec3<f32>(n * 0.026 * dark_weight),
                    vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(rgb, 1.0);
}
