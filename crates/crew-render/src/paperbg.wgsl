struct Uniform {
    page_bg: vec4<f32>,
    resolution: vec2<f32>,
    intensity: f32,
    grain_mul: f32,   // replaces pad; scales grain amplitude (0 = no grain, 1 = default ~3%)
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
    let d2 = dot(uv - vec2<f32>(0.5), uv - vec2<f32>(0.5));
    let vignette = 1.0 - d2 * 0.1;

    // Grain: ±3% amplitude around neutral, scaled by grain_mul.
    let g = (grain(in.pos.xy) - 0.5) * 0.06 * u.grain_mul;

    // Blend between full effect (intensity=1) and plain bg (intensity=0).
    let mod_factor = (vignette + g) * u.intensity + (1.0 - u.intensity);
    let rgb = u.page_bg.rgb * clamp(mod_factor, 0.0, 1.0);
    return vec4<f32>(rgb, 1.0);
}
