// Liquid-glass pane card: a two-layer shadow (a tight contact shadow plus a
// wide ambient one), a tinted fill with a top-lit vertical ramp, a specular
// rim lit from the upper left with a fainter refracted bounce along the lower
// right, and a whisper of frost grain. One instance per pane, one draw,
// composited in the fragment shader so the shadow and the sheet cost a single
// pass.

// `tilt`: where the pointer sits in the window, -1..=1 per axis (0 at rest).
struct Vp { size: vec2<f32>, tilt: vec2<f32> };
@group(0) @binding(0) var<uniform> vp: Vp;

// How far outside the card the quad is expanded to give the shadow room. Must
// stay >= the shadow's blur + offset or the falloff is visibly clipped square.
const PAD: f32 = 52.0;
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
// How far the pointer leans that light: at a window corner the light swings
// well round toward it, but never all the way — the look stays the look.
const TILT: f32 = 0.6;
// Two shadows, as a real object casts: a tight, dark CONTACT shadow that says
// where the card touches the page, and a wide, faint AMBIENT one that says how
// far it has lifted. One wide shadow alone reads as a smudge around the card.
const CONTACT_BLUR: f32 = 4.0;
const CONTACT_DROP: f32 = 1.5;
const CONTACT_W: f32 = 0.55;
const AMBIENT_BLUR: f32 = 18.0;
const AMBIENT_DROP: f32 = 6.0;
const AMBIENT_W: f32 = 0.45;
// What a full lift adds: the ambient shadow drops further and spreads wider
// (a card higher off the page throws a softer, more distant shadow), the whole
// shadow darkens, and the rim catches more light. The contact shadow stays
// put — it is where the card's edge meets its own shadow, lifted or not.
const LIFT_DROP: f32 = 4.0;
const LIFT_BLUR: f32 = 6.0;
const LIFT_SHADOW: f32 = 0.8;
const LIFT_RIM: f32 = 0.35;
// The highest a card rides: 1 is the focused pane, 2 a floating card (a
// pop-up, `/keys`, a toast) over everything.
const MAX_LIFT: f32 = 2.0;
// A NEGATIVE lift sinks the card into the page instead: a well, the way a
// text field is pressed into a sheet of glass. No shadow falls outside it;
// one falls INSIDE, under its top lip, and the light catches the lower lip
// instead of the upper one. -1 is a full well.
const WELL_DROP: f32 = 2.0;
const WELL_BLUR: f32 = 7.0;
const WELL_SHADOW: f32 = 1.4;
// The focus glint: a soft specular GLINT_PX wide that runs once along the top
// rim as focus lands, fading in and out over its run so it never pops.
const GLINT_PX: f32 = 90.0;
const GLINT_GAIN: f32 = 0.85;
// The bevel's shade: the edges facing AWAY from the light darken a touch, as
// the underside of a thick lens does. A white rim on a white sheet is all but
// invisible, so on a light page this is what says the top is lit and the
// bottom is not. Scaled by the card's shadow, so it deepens with the lift and
// never exists where no shadow does (the tubes, a floating card's shadow-only
// overlay has no rim at all).
const SHADE_GAIN: f32 = 0.6;
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
  @location(5) extra: vec4<f32>,   // scan position, edge_glow, lift, glint
};

// Half-width of the busy sheen, as a fraction of the card's diagonal. Wide
// enough to read as light rather than a line.
const SCAN_W: f32 = 0.12;
// How much the sheen lifts the sheet at its centre. Deliberately slight — it
// runs while a pane is working, and a bright bar crossing the card would be
// the most annoying thing crew does.
const SCAN_GAIN: f32 = 0.28;

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
  let lift = clamp(in.extra.z, 0.0, MAX_LIFT);
  let well = clamp(-in.extra.z, 0.0, 1.0);
  var shadow = 0.0;
  if (sh_alpha > 0.0 && well == 0.0) {
    let dc = sd_round_box(in.local - vec2<f32>(0.0, CONTACT_DROP), in.hsize, radius);
    let da = sd_round_box(in.local - vec2<f32>(0.0, AMBIENT_DROP + LIFT_DROP * lift),
                          in.hsize, radius);
    let s = CONTACT_W * falloff(dc, CONTACT_BLUR)
      + AMBIENT_W * falloff(da, AMBIENT_BLUR + LIFT_BLUR * lift);
    shadow = clamp(sh_alpha * (1.0 + LIFT_SHADOW * lift), 0.0, 1.0) * s * (1.0 - inside);
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

  // --- busy sheen -----------------------------------------------------------
  // A soft diagonal band of extra fill crossing the card once, upper left to
  // lower right — the way the light falls — while the pane works; the app
  // rests it between passes. It starts and ends off the card, so it never
  // pops in, and it rides the fill alpha rather than adding a colour of its
  // own, so it stays in whatever palette the theme declared.
  let scan_pos = in.extra.x;
  if (scan_pos >= 0.0) {
    let across = clamp((in.local.x + in.hsize.x) / max(in.hsize.x * 2.0, 1.0), 0.0, 1.0);
    let centre = -SCAN_W + scan_pos * (1.0 + 2.0 * SCAN_W);
    let d_scan = abs((across + t) * 0.5 - centre);
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

  // --- well: the inner shadow -----------------------------------------------
  // The page's edge, dropped a couple of px and blurred, seen from inside: a
  // band hugging the top lip that fades down into the field. Black over the
  // fill, so it deepens whatever tint the sheet has.
  if (well > 0.0 && sh_alpha > 0.0) {
    let lip = sd_round_box(in.local - vec2<f32>(0.0, WELL_DROP), in.hsize, radius);
    let a = clamp(sh_alpha * WELL_SHADOW * well, 0.0, 1.0)
      * smoothstep(-WELL_BLUR, WELL_DROP, lip) * inside;
    let out_a = a + alpha * (1.0 - a);
    if (out_a > 0.0001) {
      rgb = rgb * alpha * (1.0 - a) / out_a;
    }
    alpha = out_a;
  }

  if (hl_alpha > 0.0) {
    let band = smoothstep(-HL_W, 0.0, d) * inside;
    let e = 0.5;
    let n = normalize(vec2<f32>(
      sd_round_box(in.local + vec2<f32>(e, 0.0), in.hsize, radius)
        - sd_round_box(in.local - vec2<f32>(e, 0.0), in.hsize, radius),
      sd_round_box(in.local + vec2<f32>(0.0, e), in.hsize, radius)
        - sd_round_box(in.local - vec2<f32>(0.0, e), in.hsize, radius)) + vec2<f32>(1e-5, 1e-5));
    // A well's lit wall is the one facing AWAY from the light: its lower lip.
    let light = normalize(LIGHT + TILT * vp.tilt);
    let facing = dot(n, light) * select(1.0, -1.0, well > 0.0);
    let key = pow(clamp(facing, 0.0, 1.0), 1.5);
    let bounce = BOUNCE * pow(clamp(-facing, 0.0, 1.0), 2.0);
    var a = clamp(hl_alpha * (1.0 + LIFT_RIM * lift), 0.0, 1.0) * band * (key + bounce);
    let glint = in.extra.w;
    if (glint >= 0.0) {
      let at = -in.hsize.x + glint * 2.0 * in.hsize.x;
      let dx = (in.local.x - at) / GLINT_PX;
      let up = clamp(-n.y, 0.0, 1.0);
      let envelope = sin(3.14159265 * clamp(glint, 0.0, 1.0));
      a = clamp(a + GLINT_GAIN * envelope * exp(-dx * dx * 2.0) * band * up, 0.0, 1.0);
    }
    // Composite the highlight over the fill (both are "source" here).
    let out_a = a + alpha * (1.0 - a);
    if (out_a > 0.0001) {
      rgb = (in.hl.xyz * a + rgb * alpha * (1.0 - a)) / out_a;
    }
    alpha = out_a;

    // The shade, black over all of that, on the far side from the light.
    let shade = clamp(sh_alpha * SHADE_GAIN * (1.0 + LIFT_SHADOW * lift), 0.0, 1.0)
      * band * pow(clamp(-facing, 0.0, 1.0), 1.5);
    if (shade > 0.0) {
      let sa = shade + alpha * (1.0 - shade);
      rgb = rgb * alpha * (1.0 - shade) / max(sa, 0.0001);
      alpha = sa;
    }
  }

  // --- fill over shadow -----------------------------------------------------
  let out_a = alpha + shadow * (1.0 - alpha);
  if (out_a <= 0.0015) { discard; }
  // The shadow is pure black, so it contributes no colour — only weight.
  let out_rgb = rgb * alpha / out_a;
  return vec4<f32>(out_rgb, out_a);
}
