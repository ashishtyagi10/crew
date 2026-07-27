// CRT post-process: samples the off-screen scene texture and draws it onto a
// flat phosphor panel — a laptop LCD wearing the tube's phosphor glow (a cheap
// two-ring single-pass neighbour bloom), scanlines, and an activity-driven
// flicker. The barrel-curvature and corner-vignette math is kept but driven by
// uniforms that default to 0, so the geometry is flat and edge-to-edge
// (identity warp, no bezel). All amounts are uniforms so each theme can dial
// the look; flicker is 0 while idle, which makes the whole pass static (the app
// only advances `time` and lifts `flicker` while output is streaming).

struct U {
    resolution: vec2<f32>,
    time: f32,
    flicker: f32,
    curvature: f32,
    scanline: f32,
    glow: f32,
    corner: f32,
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

// Deterministic 0..1 hash of a scalar — drives the brightness flicker.
fn hash1(x: f32) -> f32 {
    return fract(sin(x * 12.9898) * 43758.5453);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    // Screen UV in [0, 1], origin top-left.
    let uv = in.pos.xy / u.resolution;

    // Barrel curvature: work in centered [-1, 1] coords and push the edges
    // outward by r^2, so the image bulges like a tube face.
    var c = uv * 2.0 - 1.0;
    let r2 = dot(c, c);
    c = c * (1.0 + u.curvature * r2);
    let warped = c * 0.5 + 0.5;

    // Anything past the glass edge after warping is the bezel — solid black.
    if (warped.x < 0.0 || warped.x > 1.0 || warped.y < 0.0 || warped.y > 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let scene = textureSample(tex, samp, warped);
    var col = scene.rgb;

    // Phosphor glow: a cheap two-ring neighbour bloom adds a fraction of
    // nearby brightness so bright glyphs bleed a soft halo (real bloom would
    // be a separate blur chain; this reads convincingly for text at one
    // pass). The inner ring (1.5px) gives the sharp near-edge lift; the outer
    // ring (3.5px, 0.45 weight) widens the reach so the halo carries several
    // pixels out instead of stopping dead at the block edge.
    if (u.glow > 0.0) {
        let o = (1.5 / u.resolution);
        var bloom = vec3<f32>(0.0);
        bloom += textureSample(tex, samp, warped + vec2<f32>( o.x, 0.0)).rgb;
        bloom += textureSample(tex, samp, warped + vec2<f32>(-o.x, 0.0)).rgb;
        bloom += textureSample(tex, samp, warped + vec2<f32>(0.0,  o.y)).rgb;
        bloom += textureSample(tex, samp, warped + vec2<f32>(0.0, -o.y)).rgb;
        bloom += textureSample(tex, samp, warped + vec2<f32>( o.x,  o.y)).rgb;
        bloom += textureSample(tex, samp, warped + vec2<f32>(-o.x,  o.y)).rgb;
        bloom += textureSample(tex, samp, warped + vec2<f32>( o.x, -o.y)).rgb;
        bloom += textureSample(tex, samp, warped + vec2<f32>(-o.x, -o.y)).rgb;
        let o2 = (3.5 / u.resolution);
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

    // Corner darkening on the curved coords — deeper than the background
    // vignette so the tube face falls off into its edges.
    col *= 1.0 - u.corner * r2;

    // Activity flicker: a small brightness wobble, exactly 0 when idle.
    col *= 1.0 + u.flicker * (hash1(u.time) - 0.5);

    // Carry the scene's alpha through untouched — the tube effects shape light,
    // not transparency, so a translucent window stays translucent under CRT.
    return vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), scene.a);
}
