struct Vp { size: vec2<f32>, pad: vec2<f32> };
@group(0) @binding(0) var<uniform> vp: Vp;

struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) local: vec2<f32>,
  @location(1) hsize: vec2<f32>,
  @location(2) radius: f32,
  @location(3) thickness: f32,
  @location(4) color: vec4<f32>,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32,
      @location(0) rect: vec4<f32>,
      @location(1) params: vec4<f32>,
      @location(2) color: vec4<f32>) -> VsOut {
  var corners = array<vec2<f32>,6>(
    vec2<f32>(0.0,0.0), vec2<f32>(1.0,0.0), vec2<f32>(0.0,1.0),
    vec2<f32>(0.0,1.0), vec2<f32>(1.0,0.0), vec2<f32>(1.0,1.0));
  let c = corners[vi];
  let px = rect.xy + c * rect.zw;
  let clip = vec2<f32>(px.x / vp.size.x * 2.0 - 1.0, 1.0 - px.y / vp.size.y * 2.0);
  let hs = rect.zw * 0.5;
  var out: VsOut;
  out.pos = vec4<f32>(clip, 0.0, 1.0);
  out.local = px - (rect.xy + hs);
  out.hsize = hs;
  out.radius = params.x;
  out.thickness = params.y;
  out.color = color;
  return out;
}

fn sd_round_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
  let q = abs(p) - b + vec2<f32>(r, r);
  return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - r;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
  let d = sd_round_box(in.local, in.hsize, in.radius);
  let aa = 1.5;
  let outer = 1.0 - smoothstep(0.0, aa, d);
  let inner = smoothstep(0.0, aa, d + in.thickness);
  let alpha = outer * inner;
  if (alpha <= 0.001) { discard; }
  return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
