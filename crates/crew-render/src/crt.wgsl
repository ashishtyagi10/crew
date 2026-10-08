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
    core: f32,
    shade: f32,
    // 1: leave the frame premultiplied (crtchain's `premultiplies`), 0: straight.
    premul: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
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

// Most glow one pixel takes (see the composite).
const GLOW_CAP: f32 = 0.18;

// The shadow's ramp over the blurred lit-ink mask (bloom's alpha; the blur's
// lifted kernel sums to 2.56, so a solid field reads 2.56). Measured on the
// crt-green window: a lone frame line reads ~0.2 (0.3 where two cross), so
// it peaks under SHADE_FROM — a frame needs no shadow, and its sparse-tap
// echoes would stripe the wallpaper — while a run of text reads 0.45-0.85.
const SHADE_FROM: f32 = 0.25;
const SHADE_FULL: f32 = 0.7;
// The mask is the bloom's half-res blur, whose sparse taps leave copies of
// each stroke a few px apart and whose texels are 2 px square: read raw, the
// shadow is a striped, blocky smudge. So the composite reads it through a
// 3×3 tent this many full-res px apart, which spans both and leaves a soft
// cloud behind the text.
const SHADE_SPAN: f32 = 3.0;

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
    //
    // Capped per pixel. The kernel sums well past 1 so a stroke's halo
    // reaches far, which over a wide FILL is the fill's light several times
    // over — poured onto the dark letters of a selected row until they read
    // as the bar's own colour. A stroke's halo sits well under the cap; a
    // flooded field does not.
    let bloom = textureSample(bloom_tex, samp, warped).rgb;
    col += clamp(bloom * u.glow, vec3<f32>(-GLOW_CAP), vec3<f32>(GLOW_CAP));

    // The FILAMENT: a lit stroke burns white at its core inside its coloured
    // halo, as a neon tube's current does inside its glass — the halo above
    // keeps the phosphor's colour, the core goes white-hot. Only THIN bright
    // strokes burn: the bloom says how much light surrounds a pixel, and a
    // glyph or a frame line has little around it while a wide bright fill (a
    // selected row, a bar) is drowning in its own, so a fill keeps its
    // colour and the dark letters on it stay dark. Zero on every theme that
    // is not a tube.
    if (u.core > 0.0) {
        let peak = max(scene.r, max(scene.g, scene.b));
        let around = dot(bloom, vec3<f32>(0.2126, 0.7152, 0.0722));
        let hot = smoothstep(0.5, 0.9, peak) * (1.0 - smoothstep(0.3, 0.8, around));
        col += (vec3<f32>(1.0) - col) * (u.core * hot);
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

    // Activity flicker: a small brightness wobble, exactly 0 when idle.
    col *= 1.0 + u.flicker * (hash1(u.time) - 0.5);

    // The SHADOW (a tube on a see-through window): around what is solid —
    // the text above all — the window takes a little more of the desktop's
    // light away, softly, out to the blur's reach. Only the alpha moves: the
    // window server composites crew premultiplied, so raising alpha without
    // adding colour dims the wallpaper behind a line of text and leaves the
    // glow, the frost and the gaps between the lines exactly as they were.
    // An opaque window is alpha 1 already, so this is a no-op there.
    var ink = 0.0;
    if (u.shade > 0.0) {
        let d = SHADE_SPAN / u.resolution;
        for (var j = -1; j <= 1; j++) {
            for (var i = -1; i <= 1; i++) {
                let w = f32((2 - abs(i)) * (2 - abs(j))) / 16.0;
                ink += w * textureSample(bloom_tex, samp, warped + d * vec2<f32>(f32(i), f32(j))).a;
            }
        }
    }
    let shadow = u.shade * smoothstep(SHADE_FROM, SHADE_FULL, ink);
    let a = scene.a + (1.0 - scene.a) * shadow;

    // Otherwise the scene's alpha passes through untouched — the tube effects
    // shape light, not transparency, so a translucent window stays
    // translucent under CRT.
    //
    // See-through glass leaves PREMULTIPLIED: the window server adds rgb to
    // what the alpha leaves of the desktop, so a light page written straight
    // is the desktop plus near-white — a white-out, not glass.
    let out = clamp(col, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(out * mix(1.0, a, u.premul), a);
}
