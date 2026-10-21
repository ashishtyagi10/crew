// Pictures: one textured rectangle each, sampled smoothly, handed to the
// blend premultiplied (`picture.rs`, `images.rs`).
struct Vp { size: vec2<f32>, _pad: vec2<f32> };
@group(0) @binding(0) var<uniform> vp: Vp;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct Inst {
    @location(0) rect: vec4<f32>,
    @location(1) uv: vec4<f32>,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32, inst: Inst) -> VsOut {
    var corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0)
    );
    let c = corners[vi];
    let px = inst.rect.xy + c * inst.rect.zw;
    var out: VsOut;
    out.pos = vec4(px.x / vp.size.x * 2.0 - 1.0, 1.0 - px.y / vp.size.y * 2.0, 0.0, 1.0);
    out.uv = mix(inst.uv.xy, inst.uv.zw, c);
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    // Already premultiplied (`Pixels::new`).
    return textureSample(tex, samp, in.uv);
}
