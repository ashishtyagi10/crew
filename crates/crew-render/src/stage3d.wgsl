// The 3D backdrop (`stage3d`), laid down over the page's clear: the threers
// picture, at the backdrop's strength. threers draws premultiplied black at
// zero alpha where nothing is, so what it did not draw adds nothing.
// Output is premultiplied (the pipeline blends with `PREMUL_OVER`).

struct U { strength: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var pic: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
  var p = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
  var o: VsOut;
  o.pos = vec4<f32>(p[vi], 0.0, 1.0);
  o.uv = vec2<f32>(p[vi].x * 0.5 + 0.5, 0.5 - p[vi].y * 0.5);
  return o;
}

@fragment
fn fs(i: VsOut) -> @location(0) vec4<f32> {
  return textureSample(pic, samp, i.uv) * u.strength.x;
}
