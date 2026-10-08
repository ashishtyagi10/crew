struct Vp { size: vec2<f32>, pad: vec2<f32> };
@group(0) @binding(0) var<uniform> vp: Vp;

struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) local: vec2<f32>,
  @location(1) hsize: vec2<f32>,
  @location(2) radius: f32,
  @location(3) thickness: f32,
  @location(4) color: vec4<f32>,
  @location(5) crisp: f32,
  @location(6) color_h: vec4<f32>,
};

// `rect` is what is drawn — the whole ring, or the clip a corner's arc is
// cut from (`corners.rs`); `shape` is the ring's rounded box either way.
@vertex
fn vs(@builtin(vertex_index) vi: u32,
      @location(0) rect: vec4<f32>,
      @location(1) shape: vec4<f32>,
      @location(2) params: vec4<f32>,
      @location(3) color: vec4<f32>,
      @location(4) color_h: vec4<f32>) -> VsOut {
  var corners = array<vec2<f32>,6>(
    vec2<f32>(0.0,0.0), vec2<f32>(1.0,0.0), vec2<f32>(0.0,1.0),
    vec2<f32>(0.0,1.0), vec2<f32>(1.0,0.0), vec2<f32>(1.0,1.0));
  let c = corners[vi];
  let px = rect.xy + c * rect.zw;
  let clip = vec2<f32>(px.x / vp.size.x * 2.0 - 1.0, 1.0 - px.y / vp.size.y * 2.0);
  let hs = shape.zw * 0.5;
  var out: VsOut;
  out.pos = vec4<f32>(clip, 0.0, 1.0);
  out.local = px - (shape.xy + hs);
  out.hsize = hs;
  out.radius = params.x;
  out.thickness = params.y;
  out.crisp = params.z;
  out.color = color;
  out.color_h = color_h;
  return out;
}

fn sd_round_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
  let q = abs(p) - b + vec2<f32>(r, r);
  return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - r;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
  let d = sd_round_box(in.local, in.hsize, in.radius);
  var alpha = 0.0;
  if (in.crisp > 0.5) {
    // Box-filtered at the pixel centre: a straight edge on a pixel boundary
    // is exactly on or off, like the rule glyphs an arc's tails continue,
    // and the curve between them antialiases over one pixel.
    alpha = clamp(0.5 - d, 0.0, 1.0) * clamp(0.5 + d + in.thickness, 0.0, 1.0);
  } else {
    let aa = 1.5;
    let outer = 1.0 - smoothstep(0.0, aa, d);
    let inner = smoothstep(0.0, aa, d + in.thickness);
    alpha = outer * inner;
  }
  if (alpha <= 0.001) { discard; }
  // The vertical runs wear `color`, the horizontal ones `color_h`, and round
  // a corner the one fades into the other by angle about the arc's centre.
  let q = abs(in.local) - (in.hsize - vec2<f32>(in.radius));
  let turn = atan2(max(q.y, 0.0), max(q.x, 0.0) + 1e-4) / 1.5707963;
  let c = mix(in.color, in.color_h, clamp(turn, 0.0, 1.0));
  return vec4<f32>(c.rgb, c.a * alpha);
}
