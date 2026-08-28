// The rects a sheer window keeps solid, written into the ALPHA channel alone.
//
// Colour is somebody else's job: this pass runs at the very end of the scene
// pass with `ColorWrites::ALPHA` and no blending, so every pixel it covers
// keeps exactly the colour the page, the wash, the lattice, the cell
// backgrounds and the text left there — and stops being see-through.
struct Uniform {
    // xy = the surface size in px. zw unused.
    res: vec4<f32>,
    // xy = a rect's top-left in physical px, zw = its size. One instance is
    // drawn per rect the frame asked for, so unused slots are never read.
    rects: array<vec4<f32>, 8>,
}
@group(0) @binding(0) var<uniform> u: Uniform;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
}

@vertex
fn vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VsOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    let r = u.rects[ii];
    let p = r.xy + corners[vi] * r.zw;
    var out: VsOut;
    // px → NDC, y down (the same convention every other pass here uses).
    out.pos = vec4<f32>(
        p.x / max(u.res.x, 1.0) * 2.0 - 1.0,
        1.0 - p.y / max(u.res.y, 1.0) * 2.0,
        0.0,
        1.0,
    );
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    // Only the `a` survives the write mask; the rgb is never read.
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
