// CRT composite: samples the off-screen scene texture and draws it onto a
// flat phosphor panel — the tube's phosphor glow (a real half-res gaussian
// bloom, blurred in bloom.wgsl and added back here), scanlines, and an
// activity-driven flicker. The panel is flat and edge-to-edge: the barrel
// curvature and corner vignette this pass once carried were set to 0 by every
// theme after the flat-tube decree, so the arithmetic ran per pixel per frame
// and produced an identity warp and a multiply by one. Both are gone. All
// remaining amounts are uniforms so each theme can dial the look; flicker is 0
// while idle, which makes the whole pass static (the app only advances `time`
// and lifts `flicker` while output is streaming).

struct U {
    resolution: vec2<f32>,
    time: f32,
    flicker: f32,
    scanline: f32,
    glow: f32,
}
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> u: U;
// The blurred bright-pass from bloom.wgsl — half the scene's resolution, so
// the bilinear fetch below is also the upsample.
@group(0) @binding(3) var bloom_tex: texture_2d<f32>;

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

// Deterministic 0..1 hash of a scalar — drives the brightness flicker.
fn hash1(x: f32) -> f32 {
    return fract(sin(x * 12.9898) * 43758.5453);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    // Screen UV in [0, 1], origin top-left.
    let uv = in.pos.xy / u.resolution;

    // Flat panel: the sample coordinate is the screen coordinate. There is no
    // warp to push pixels past the glass edge, so there is no bezel test.
    let warped = uv;

    let scene = textureSample(tex, samp, warped);
    var col = scene.rgb;

    // Phosphor glow: add the pre-blurred bright-pass (bloom.wgsl's half-res
    // gaussian chain) scaled by the theme's glow. This is what replaced the
    // old two-ring neighbour tap — the halo now carries tens of pixels with a
    // gaussian falloff instead of dying ~8px from the stroke.
    col += textureSample(bloom_tex, samp, warped).rgb * u.glow;

    // Glow can push col past 1.0 on bright/saturated fields (e.g. a uniform
    // bright field with two rings summing in). Clamp here, before the
    // scanline multiply, so the darkened rows are always a fraction of a
    // bounded value — otherwise every row clips to the same ceiling and the
    // scanline's row-to-row delta washes out (the pre-fix failure mode: a
    // stronger glow silently erased the scanlines it was drawn on top of).
    // The final return-clamp still applies after flicker, which can push
    // values out of range again post-scanline.
    col = clamp(col, vec3<f32>(0.0), vec3<f32>(1.0));

    // Scanlines: a cosine keyed to physical rows darkens a line every
    // SCANLINE_PERIOD pixels, the signature horizontal texture of a raster tube.
    // The period must NOT be 2 px: at exactly one cycle per 2 px the cosine is
    // sampled at its zero crossing on every pixel centre (cos((y+0.5)π)=0) and
    // aliases to a flat 0.5 — no visible lines, worse on hi-DPI. A 3-px period
    // both reads as scanlines and survives upscaled displays.
    let scanline_period = 3.0;
    let line = 0.5 + 0.5 * cos(warped.y * u.resolution.y * (6.2831853 / scanline_period));
    col *= 1.0 - u.scanline * line;

    // Activity flicker: a small brightness wobble, exactly 0 when idle.
    col *= 1.0 + u.flicker * (hash1(u.time) - 0.5);

    // Carry the scene's alpha through untouched — the tube effects shape light,
    // not transparency, so a translucent window stays translucent under CRT.
    return vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), scene.a);
}
