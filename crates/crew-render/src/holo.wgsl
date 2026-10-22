// The holo grid (crew-theme `holo`): the neon page's backdrop, drawn over the
// page's clear and under everything else, so the panes' glass floats over it
// and shows it through. Fine lines a few cells apart, brighter every fourth,
// a crosshair at each bright crossing and a point at every other; a soft key
// light from above in the accent and a violet fill from the lower right;
// sparks drifting up through it, and now and then a packet of light running
// up a bright line. Everything that moves rides the clock in `clock.x`, which
// turns only while a pane works and holds at Motion off.
//
// Output is premultiplied (the pipeline blends with `PREMUL_OVER`).

struct U {
  size: vec2<f32>,
  cell: vec2<f32>,
  line: vec4<f32>,   // the grid's colour; a = strength
  spark: vec4<f32>,  // the cyan spark
  spark2: vec4<f32>, // the violet spark
  clock: vec4<f32>,  // x = phase in turns
};
@group(0) @binding(0) var<uniform> u: U;

struct VsOut { @builtin(position) pos: vec4<f32> };

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
  var p = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
  var o: VsOut;
  o.pos = vec4<f32>(p[vi], 0.0, 1.0);
  return o;
}

fn hash21(p: vec2<f32>) -> f32 {
  let h = dot(p, vec2<f32>(127.1, 311.7));
  return fract(sin(h) * 43758.5453123);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
  return vec2<f32>(hash21(p), hash21(p + vec2<f32>(17.31, 41.97)));
}

// A line `w` px wide at `f` px from it, antialiased over one px.
fn stroke(f: f32, w: f32) -> f32 {
  return 1.0 - smoothstep(w * 0.5, w * 0.5 + 1.0, f);
}

// One field of sparks: a lattice `pitch` px square, each cell lit with chance
// `p`, the field rising `n` cells a turn of the clock (whole cells, so the
// turn wraps without a seam), each spark about `r` px. Returns how bright,
// and which of the two colours.
fn sparks(px: vec2<f32>, pitch: f32, p: f32, n: f32, r: f32, seed: f32, ph: f32) -> vec2<f32> {
  let q = vec2<f32>(px.x, px.y + ph * n * pitch);
  let id = floor(q / pitch);
  let h = hash21(id + vec2<f32>(seed, seed * 1.7));
  if (h > p) { return vec2<f32>(0.0, 0.0); }
  let at = (hash22(id + vec2<f32>(seed * 3.1, seed)) * 0.5 + 0.25) * pitch;
  let d = length(q - id * pitch - at);
  let rr = r * (0.7 + 0.6 * hash21(id + vec2<f32>(seed * 7.7, 2.0)));
  let core = 1.0 - smoothstep(rr, rr + 1.0, d);
  let glow = exp(-d * d / (2.0 * rr * rr * 4.0));
  // Each twinkles on its own beat: three beats a turn, so it wraps too.
  let tw = 0.6 + 0.4 * sin(6.2831853 * (ph * 3.0 + h * 17.0));
  return vec2<f32>((core + 0.5 * glow) * tw, step(0.5, hash21(id + vec2<f32>(seed * 5.3, 9.0))));
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
  let px = in.pos.xy;
  let k = u.line.a;
  let ph = u.clock.x;
  // Four columns by two rows a square, so the lines keep to the text's grid.
  let pitch = max(vec2<f32>(u.cell.x * 4.0, u.cell.y * 2.0), vec2<f32>(8.0, 8.0));
  let big = pitch * 4.0;
  let fm = abs(fract(px / pitch + 0.5) - 0.5) * pitch;
  let fb = abs(fract(px / big + 0.5) - 0.5) * big;
  let minor = max(stroke(fm.x, 1.0), stroke(fm.y, 1.0));
  let major = max(stroke(fb.x, 1.0), stroke(fb.y, 1.0));
  let cross = max(stroke(fb.x, 1.5) * step(fb.y, 6.0), stroke(fb.y, 1.5) * step(fb.x, 6.0));
  let point = 1.0 - smoothstep(0.9, 1.9, length(fm));

  // The room's light: a key from above the centre, a violet fill from the
  // lower right, the corners falling away into the dark.
  let uv = px / max(u.size, vec2<f32>(1.0, 1.0));
  let kd = (uv - vec2<f32>(0.5, -0.15)) * vec2<f32>(1.0, 1.5);
  let key = exp(-dot(kd, kd) * 2.0);
  let fd = (uv - vec2<f32>(1.05, 1.1)) * vec2<f32>(1.2, 1.5);
  let fill = exp(-dot(fd, fd) * 2.5);
  let vd = (uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(1.4, 1.6);
  let vig = 1.0 - smoothstep(0.35, 1.25, length(vd));

  let lit = (0.55 + 0.45 * vig) * (0.75 + 0.5 * key);
  let a_grid = (0.05 * minor + 0.12 * major + 0.22 * cross + 0.16 * point) * lit;
  var col = u.line.rgb * a_grid + u.line.rgb * 0.045 * key + u.spark2.rgb * 0.06 * fill;
  var a = a_grid + 0.045 * key + 0.06 * fill;

  // Sparks: a fine field and a nearer, larger, slower one.
  let s1 = sparks(px, pitch.x * 0.75, 0.20, 3.0, 1.0, 1.0, ph);
  let s2 = sparks(px, pitch.x * 2.0, 0.16, 2.0, 1.7, 9.0, ph);
  let b1 = 0.55 * s1.x * (0.6 + 0.4 * vig);
  let b2 = 0.40 * s2.x * (0.6 + 0.4 * vig);
  col = col + mix(u.spark.rgb, u.spark2.rgb, s1.y) * b1 + mix(u.spark.rgb, u.spark2.rgb, s2.y) * b2;
  a = a + b1 + b2;

  // Packets of light running up some of the bright columns.
  let column = floor(px.x / big.x + 0.5);
  if (hash21(vec2<f32>(column, 3.0)) > 0.45) {
    let period = big.y * 1.5;
    let s = fract((px.y + ph * 2.0 * period) / period + hash21(vec2<f32>(column, 8.0)));
    let trail = (1.0 - smoothstep(0.0, 0.18, s)) * smoothstep(0.0, 0.01, s);
    let p = stroke(fb.x, 1.5) * trail * 0.5;
    col = col + u.spark.rgb * p;
    a = a + p;
  }

  a = clamp(a * k, 0.0, 1.0);
  return vec4<f32>(min(col * k, vec3<f32>(a)), a);
}
