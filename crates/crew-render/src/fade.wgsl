// Theme-switch crossfade: the LAST presented frame (snapshotted by
// `fadepass.rs`) drawn over the freshly-rendered new-theme frame at a
// decaying opacity. The snapshot is surface-sized and pixel-aligned, so the
// fragment reads its own texel directly — no sampler math, no filtering, the
// old frame sits exactly where it was.

struct U {
    // How strongly the old frame still covers the new one (1 → all old).
    fade: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler; // layout companion; unused (texel load)
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

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    // The snapshot's own alpha carries the window opacity; the fade scales it
    // so a translucent window crossfades without ever going more opaque.
    return vec4<f32>(c.rgb, c.a * u.fade);
}
