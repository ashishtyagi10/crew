// Liquid-glass pane card: a two-layer shadow (a tight contact shadow plus a
// wide ambient one), a tinted fill with a top-lit vertical ramp, a specular
// rim lit from the upper left with a fainter refracted bounce along the lower
// right, and a whisper of frost grain. One instance per pane, one draw,
// composited in the fragment shader so the shadow and the sheet cost a single
// pass.

struct Vp { size: vec2<f32>, pad: vec2<f32> };
@group(0) @binding(0) var<uniform> vp: Vp;

// How far outside the card the quad is expanded to give the shadow room. Must
// stay >= the shadow's blur + offset or the falloff is visibly clipped square.
const PAD: f32 = 32.0;
// Width (px) of the specular rim just inside the card edge. The frame's stroke
// sits on the edge itself and covers the outer part of it, so what shows is a
// bright line tucked just inside the frame.
const HL_W: f32 = 3.5;
// The light: from the upper left, as every lit surface on a desktop is. The
// rim burns where the edge faces it and a refracted bounce — the look that
// reads as a thick lens rather than a pane of window glass — glows faintly on
// the opposite edge.
const LIGHT: vec2<f32> = vec2<f32>(-0.45, -0.89);
const BOUNCE: f32 = 0.35;
// Two shadows, as a real object casts: a tight, dark CONTACT shadow that says
// where the card touches the page, and a wide, faint AMBIENT one that says how
// far it has lifted. One wide shadow alone reads as a smudge around the card.
const CONTACT_BLUR: f32 = 4.0;
const CONTACT_DROP: f32 = 1.5;
const CONTACT_W: f32 = 0.55;
const AMBIENT_BLUR: f32 = 18.0;
const AMBIENT_DROP: f32 = 6.0;
const AMBIENT_W: f32 = 0.45;
// Edge antialiasing width.
const AA: f32 = 1.0;
// Inner edge-glow reach (px): how far the frame's light bleeds into the fill.
const GLOW_W: f32 = 24.0;

struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) local: vec2<f32>,   // px relative to the card centre
  @location(1) hsize: vec2<f32>,   // card half-extent
  @location(2) params: vec4<f32>,  // radius, alpha_top, alpha_bottom, noise
  @location(3) tint: vec4<f32>,    // tint.rgb, highlight_alpha
  @location(4) hl: vec4<f32>,      // highlight.rgb, shadow_alpha
  @location(5) extra: vec4<f32>,   // scan position, edge_glow, unused
};

// Half-height of the scan band, as a fraction of the card. Wide enough to read
// as a sweep of light rather than a line.
const SCAN_W: f32 = 0.18;
// How much the scan lifts the sheet at its centre. Deliberately slight — this
// runs while a pane is working, and a bright bar crossing the card every second
// would be the most annoying thing crew does.
const SCAN_GAIN: f32 = 0.35;

@vertex
fn vs(@builtin(vertex_index) vi: u32,
      @location(0) rect: vec4<f32>,
      @location(1) params: vec4<f32>,
      @location(2) tint: vec4<f32>,
      @location(3) hl: vec4<f32>,
      @location(4) extra: vec4<f32>) -> VsOut {
  var corners = array<vec2<f32>,6>(
    vec2<f32>(0.0,0.0), vec2<f32>(1.0,0.0), vec2<f32>(0.0,1.0),
    vec2<f32>(0.0,1.0), vec2<f32>(1.0,0.0), vec2<f32>(1.0,1.0));
  let c = corners[vi];
  // Expand the drawn quad by PAD on every side so the blurred shadow has room
  // to fall off outside the card itself.
  let origin = rect.xy - vec2<f32>(PAD, PAD);
  let size = rect.zw + vec2<f32>(PAD * 2.0, PAD * 2.0);
  let px = origin + c * size;
  let clip = vec2<f32>(px.x / vp.size.x * 2.0 - 1.0, 1.0 - px.y / vp.size.y * 2.0);
  let hs = rect.zw * 0.5;
  var out: VsOut;
  out.pos = vec4<f32>(clip, 0.0, 1.0);
  out.local = px - (rect.xy + hs);
  out.hsize = hs;
  out.params = params;
  out.tint = tint;
  out.hl = hl;
  out.extra = extra;
  return out;
}

fn sd_round_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
  let q = abs(p) - b + vec2<f32>(r, r);
  return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - r;
}

// A soft shadow's falloff past the edge: Gaussian-shaped, so it has no hard
// outer ring the way a linear or exponential tail does.
fn falloff(d: f32, blur: f32) -> f32 {
  let x = max(d, 0.0) / blur;
  return exp(-2.5 * x * x);
}

// Cheap value hash for the frost grain.
fn hash21(p: vec2<f32>) -> f32 {
  let h = dot(p, vec2<f32>(127.1, 311.7));
  return fract(sin(h) * 43758.5453123);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
  let radius = in.params.x;
  let a_top = in.params.y;
  let a_bot = in.params.z;
  let noise_amt = in.params.w;
  let hl_alpha = in.tint.w;
  let sh_alpha = in.hl.w;

  let d = sd_round_box(in.local, in.hsize, radius);
  let inside = 1.0 - smoothstep(0.0, AA, d);

  // --- soft drop shadow -----------------------------------------------------
  // Offset downward and smeared: an exponential falloff outside the shape gives
  // a far softer edge than a smoothstep of the same width.
  //
  // Masked by `1 - inside` because the card OCCLUDES ITS OWN SHADOW. Without
  // that, the shadow shows through the translucent sheet and darkens the card's
  // interior — enough that a white sheet over a grey page came out DARKER than
  // the page it was supposed to be lifting off (caught by `glass_headless`).
  var shadow = 0.0;
  if (sh_alpha > 0.0) {
    let dc = sd_round_box(in.local - vec2<f32>(0.0, CONTACT_DROP), in.hsize, radius);
    let da = sd_round_box(in.local - vec2<f32>(0.0, AMBIENT_DROP), in.hsize, radius);
    let s = CONTACT_W * falloff(dc, CONTACT_BLUR) + AMBIENT_W * falloff(da, AMBIENT_BLUR);
    shadow = sh_alpha * s * (1.0 - inside);
  }

  // --- frosted fill ---------------------------------------------------------
  // 0 at the card's top edge, 1 at the bottom: the ramp that makes the sheet
  // read as lit from above rather than a flat wash of colour.
  let t = clamp((in.local.y + in.hsize.y) / max(in.hsize.y * 2.0, 1.0), 0.0, 1.0);
  var fill_a = mix(a_top, a_bot, t) * inside;

  // Frost grain, signed so it neither only-lightens nor only-darkens.
  if (noise_amt > 0.0) {
    fill_a = fill_a + (hash21(floor(in.local)) - 0.5) * noise_amt * inside;
  }
  fill_a = clamp(fill_a, 0.0, 1.0);

  // --- inner edge-glow --------------------------------------------------------
  // CRT sheets read as lit by their own frame: the fill brightens at the card
  // border and fades over GLOW_W px inward. Reuses the SDF distance the corner
  // rounding already computes (-d = px inside the shape). The zero-strength
  // guard keeps every paper theme's pixels bit-identical to a build without
  // this term.
  let edge_glow = in.extra.y;
  if (edge_glow > 0.0) {
    let bleed = 1.0 - smoothstep(0.0, GLOW_W, -d);
    fill_a = clamp(fill_a + edge_glow * bleed * inside, 0.0, 1.0);
  }

  // --- scan sweep -----------------------------------------------------------
  // A soft band of extra fill travelling down the card while the pane works.
  // It rides the fill alpha rather than adding a colour of its own, so it stays
  // in whatever palette the theme declared.
  let scan_pos = in.extra.x;
  if (scan_pos >= 0.0) {
    let d_scan = abs(t - scan_pos);
    let band = 1.0 - smoothstep(0.0, SCAN_W, d_scan);
    fill_a = clamp(fill_a + band * SCAN_GAIN * mix(a_top, a_bot, t) * inside, 0.0, 1.0);
  }

  // --- specular rim --------------------------------------------------------
  // A band hugging the inside of the border, weighted by how squarely the edge
  // faces the light — so it burns along the top and the left, rounds the
  // upper-left corner, and dies out down the far sides, where a faint bounce
  // picks it up again. The normal is the SDF's own gradient: the old
  // `local / hsize` normal pointed at the corners of a wide card and lit the
  // whole top edge unevenly.
  var rgb = in.tint.xyz;
  var alpha = fill_a;
  if (hl_alpha > 0.0) {
    let band = smoothstep(-HL_W, 0.0, d) * inside;
    let e = 0.5;
    let n = normalize(vec2<f32>(
      sd_round_box(in.local + vec2<f32>(e, 0.0), in.hsize, radius)
        - sd_round_box(in.local - vec2<f32>(e, 0.0), in.hsize, radius),
      sd_round_box(in.local + vec2<f32>(0.0, e), in.hsize, radius)
        - sd_round_box(in.local - vec2<f32>(0.0, e), in.hsize, radius)) + vec2<f32>(1e-5, 1e-5));
    let facing = dot(n, LIGHT);
    let key = pow(clamp(facing, 0.0, 1.0), 1.5);
    let bounce = BOUNCE * pow(clamp(-facing, 0.0, 1.0), 2.0);
    let a = hl_alpha * band * (key + bounce);
    // Composite the highlight over the fill (both are "source" here).
    let out_a = a + alpha * (1.0 - a);
    if (out_a > 0.0001) {
      rgb = (in.hl.xyz * a + rgb * alpha * (1.0 - a)) / out_a;
    }
    alpha = out_a;
  }

  // --- fill over shadow -----------------------------------------------------
  let out_a = alpha + shadow * (1.0 - alpha);
  if (out_a <= 0.0015) { discard; }
  // The shadow is pure black, so it contributes no colour — only weight.
  let out_rgb = rgb * alpha / out_a;
  return vec4<f32>(out_rgb, out_a);
}
