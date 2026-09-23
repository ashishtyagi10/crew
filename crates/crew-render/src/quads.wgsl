struct Vp { size: vec2<f32>, _pad: vec2<f32> };
@group(0) @binding(0) var<uniform> vp: Vp;

struct Inst {
    @location(0) rect: vec4<f32>,
    @location(1) color: vec4<f32>,
    // Corner radii: top-left, top-right, bottom-right, bottom-left (px).
    @location(2) radii: vec4<f32>,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local: vec2<f32>,
    @location(2) hsz: vec2<f32>,
    @location(3) radii: vec4<f32>,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32, inst: Inst) -> VsOut {
    var corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0)
    );
    let c = corners[vi];
    let px = inst.rect.xy + c * inst.rect.zw;
    let ndc = vec2(px.x / vp.size.x * 2.0 - 1.0, 1.0 - px.y / vp.size.y * 2.0);
    var out: VsOut;
    out.pos = vec4(ndc, 0.0, 1.0);
    out.color = inst.color;
    out.hsz = inst.rect.zw * 0.5;
    out.local = (c - vec2(0.5, 0.5)) * inst.rect.zw;
    out.radii = inst.radii;
    return out;
}

// Rounded box with a radius per corner (tl, tr, br, bl), y pointing down.
fn sd_corners(p: vec2<f32>, b: vec2<f32>, r4: vec4<f32>) -> f32 {
    var r = select(r4.w, r4.z, p.x > 0.0);   // bottom: bl | br
    if (p.y < 0.0) {
        r = select(r4.x, r4.y, p.x > 0.0);   // top: tl | tr
    }
    let q = abs(p) - b + vec2(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0, 0.0))) - r;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    // Square quads — every rule, every chart fill, every run with no corner
    // on the page — take the exact old path: no coverage term at all.
    if (max(max(in.radii.x, in.radii.y), max(in.radii.z, in.radii.w)) <= 0.0) {
        return in.color;
    }
    let d = sd_corners(in.local, in.hsz, in.radii);
    let cover = clamp(0.5 - d, 0.0, 1.0);
    if (cover <= 0.0) { discard; }
    return vec4(in.color.rgb, in.color.a * cover);
}
