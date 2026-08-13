// Bloom chain for the CRT pass: three fullscreen-triangle passes over
// HALF-resolution targets. Pass 1 (fs_bright) downsamples the scene and
// keeps only what clears the brightness threshold; passes 2/3 (fs_blur_h,
// fs_blur_v) run a separable 9-tap blur whose reach scales with the theme's
// `glow_radius`. The result is what crt.wgsl adds back over the scene —
// half-res keeps the chain O(quarter the pixels) per pass, and bilinear
// upsampling in the composite hides the resolution drop inside the blur.

struct U {
    // One HALF-RES texel in UV space (1 / half-res size). The blur reads and
    // writes half-res, and the bright pass writes half-res, so this is the
    // fragment→UV scale for every pass.
    texel: vec2<f32>,
    // Blur radius in half-res pixels; taps land at i · radius/4 texels.
    radius: f32,
    // Bright-pass floor: only light above this blooms, so the near-black
    // tube page never hazes over.
    threshold: f32,
    // 0 = LIGHT bloom (dark pages): what is brighter than the floor glows.
    // 1 = INK bloom (light pages): brightness is useless there — the page is
    //     already brighter than everything on it, so a brightness pass blooms
    //     the PAGE and the whole frame clips to white. Colour is what still
    //     stands out on paper, so the pass keeps each pixel's colourfulness
    //     instead and the composite SUBTRACTS the blur (see crt.wgsl's
    //     signed glow), laying a soft coloured shadow around the ring the way
    //     the dark pages lay a soft light halo around a stroke.
    ink: f32,
}
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> u: U;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
}

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    // Fullscreen triangle — no vertex buffer.
    var pts = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var out: VsOut;
    out.pos = vec4<f32>(pts[vi], 0.0, 1.0);
    return out;
}

// Colourfulness gate for the ink pass: below CHROMA_DEAD nothing blooms, at
// CHROMA_FULL it blooms fully. Tuned against the light modern palettes so the
// gradient ring (chroma ≈ 0.78) and coloured terminal text carry a halo while
// the slate ink (≈ 0.09), the page (≈ 0.02) and the page's own wash and dot
// lattice (≈ 0.11 at the wash's strongest) carry none — body text on paper
// must stay crisp, and a halo around the backdrop would just be a smudge.
const CHROMA_DEAD: f32 = 0.25;
const CHROMA_FULL: f32 = 0.50;

@fragment
fn fs_bright(in: VsOut) -> @location(0) vec4<f32> {
    // Half-res fragment sampling the full-res scene with a bilinear sampler:
    // the fetch averages a 2×2 block, so this is the downsample too.
    let uv = in.pos.xy * u.texel;
    let c = textureSample(tex, samp, uv).rgb;
    if (u.ink > 0.5) {
        let chroma = max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b));
        let mask = smoothstep(CHROMA_DEAD, CHROMA_FULL, chroma);
        // The pixel's COMPLEMENT: subtracting it from a white page leaves the
        // pixel's own hue behind, so a blue ring bleeds blue rather than gray.
        return vec4<f32>((vec3<f32>(1.0) - c) * mask, 1.0);
    }
    return vec4<f32>(max(c - vec3<f32>(u.threshold), vec3<f32>(0.0)), 1.0);
}

// 9-tap kernel, gaussian-shaped but with lifted tails summing to 1.6 per
// pass (2.56 over both). A normalized kernel starves the far halo: a glyph
// stroke is a few texels wide, so by 8+ half-res texels out only the tail
// weights ever touch it and the reach the goal demands (light ≥ 16 full-res
// px from a stroke) rounds to black. The lift is the phosphor's energy —
// bounded, because the composite clamps before the scanlines multiply.
fn blur(uv: vec2<f32>, dir: vec2<f32>) -> vec3<f32> {
    let step = dir * u.texel * (u.radius / 4.0);
    var w = array<f32, 4>(0.24, 0.18, 0.13, 0.10);
    var acc = textureSample(tex, samp, uv).rgb * 0.30;
    for (var i = 1; i <= 4; i++) {
        let o = step * f32(i);
        acc += textureSample(tex, samp, uv + o).rgb * w[i - 1];
        acc += textureSample(tex, samp, uv - o).rgb * w[i - 1];
    }
    return acc;
}

@fragment
fn fs_blur_h(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(blur(in.pos.xy * u.texel, vec2<f32>(1.0, 0.0)), 1.0);
}

@fragment
fn fs_blur_v(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(blur(in.pos.xy * u.texel, vec2<f32>(0.0, 1.0)), 1.0);
}
