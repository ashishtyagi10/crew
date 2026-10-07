// The BLACK HOLE an awake page wears on every theme but the tubes: a
// Schwarzschild hole in the middle of the page, still, with a thin disk of
// the theme's light turning round it — Interstellar's Gargantua.
//
// It never moves and neither does the eye, so how every ray bends is fixed:
// `holelut.rs` traced it once into `hole_lut`, u = 1/r as a ray at impact
// parameter b sweeps round by ψ (units of the hole's mass). Here a pixel's
// ray is followed to where it crosses the disk's plane — up to three times:
// the near side of the disk straight in front of the shadow, the far side
// lifted over the top of it, and its underside bent round below — and the
// disk is lit where those crossings land. Only the disk turns.

@group(0) @binding(1) var hole_lut: texture_2d<f32>;
@group(0) @binding(2) var hole_smp: sampler;

// The table's extent and size: `holelut.rs` says the same (a test holds the
// two to each other).
const LUT_B_MAX: f32 = 24.0;
const LUT_PSI_MAX: f32 = 9.424778;
const LUT_W: f32 = 384.0;
const LUT_H: f32 = 512.0;

const PI: f32 = 3.1415927;
const TAU: f32 = 6.2831853;
// The hole's size on the page: half-height units per unit of its mass. The
// disk's outer edge lands about 0.38 of the page's height out from the
// middle, the shadow about 0.11.
const HOLE_M: f32 = 0.022;
// Where light stops getting away: the shadow's edge, 3√3 masses out.
const B_CRIT: f32 = 5.196152;
// How far the disk tips toward us from edge-on: the cosine of the angle
// between its axis and the line of sight. Small, as in the film — the near
// side a thin band across the shadow, the far side a full arch over it.
const TILT: f32 = 0.15;
// The disk: from the innermost stable orbit out to its edge.
const R_IN: f32 = 6.0;
const R_OUT: f32 = 18.0;
// How far the disk's inner edge turns in one cycle of the spin clock (two
// a wash orbit); further out turns slower, as the orbits there are longer.
const SPIN_IN: f32 = 0.3;

struct Hole {
    // The disk's and the photon ring's light, already weighted by how much
    // of it shows (premultiplied).
    light: vec3<f32>,
    // How much of what lies behind the disk it hides.
    cover: f32,
    // 1 inside the shadow, where nothing gets out.
    shadow: f32,
    // Where the page behind this pixel really is, in px from it — the
    // lattice is drawn from there, so the weave bends round the hole.
    lens: vec2<f32>,
    // How far out this pixel's ray passes, in masses (the impact parameter).
    b: f32,
}

// `u` for a ray at impact parameter `b`, swept round by `psi`.
fn lut_u(b: f32, psi: f32) -> f32 {
    let t = vec2<f32>(
        (b / LUT_B_MAX * (LUT_W - 1.0) + 0.5) / LUT_W,
        (psi / LUT_PSI_MAX * (LUT_H - 1.0) + 0.5) / LUT_H,
    );
    return textureSampleLevel(hole_lut, hole_smp, t, 0.0).r;
}

// Value noise that wraps every `per` cells along x, so the disk's texture
// closes round its circle.
fn pnoise(p: vec2<f32>, per: f32) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let w = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let i0 = i.x - per * floor(i.x / per);
    let i1 = select(i0 + 1.0, 0.0, i0 + 1.0 >= per);
    let a = grain(vec2<f32>(i0, i.y));
    let b = grain(vec2<f32>(i1, i.y));
    let c = grain(vec2<f32>(i0, i.y + 1.0));
    let d = grain(vec2<f32>(i1, i.y + 1.0));
    return mix(mix(a, b, w.x), mix(c, d, w.x), w.y);
}

// The disk's streaks at radius `r`, angle `turns`: long along the orbit,
// fine across it, three octaves. `seed` picks a different pattern.
fn streaks(r: f32, turns: f32, seed: f32) -> f32 {
    let lr = log(r);
    var n = 0.0;
    var amp = 1.0;
    var per = 8.0;
    var rk = 11.0;
    for (var o = 0; o < 3; o++) {
        n += amp * pnoise(vec2<f32>(turns * per, lr * rk + seed + 7.3 * f32(o)), per);
        amp *= 0.55;
        per *= 2.0;
        rk *= 2.0;
    }
    return n / 1.8525;
}

// How brightly the disk shines at radius `r`, angle `turns`, with the spin
// clock at `spin` (cycles). Each radius turns at its own Keplerian rate, so
// the pattern shears; two layers half a cycle apart, each starting fresh
// while the other carries the picture, keep the shear from winding the
// streaks tighter forever.
fn disk_light(r: f32, turns: f32, spin: f32) -> f32 {
    let om = SPIN_IN * pow(R_IN / r, 1.5);
    let t1 = fract(spin);
    let t2 = fract(spin + 0.5);
    let w1 = 1.0 - abs(2.0 * t1 - 1.0);
    let w2 = 1.0 - w1;
    let s1 = streaks(r, turns - om * t1, 0.0) - 0.5;
    let s2 = streaks(r, turns - om * t2, 31.7) - 0.5;
    // Keep the contrast steady through the cross-fade.
    let tex = 0.5 + (w1 * s1 + w2 * s2) / sqrt(w1 * w1 + w2 * w2);
    let edge = smoothstep(R_IN, R_IN + 1.2, r) * (1.0 - smoothstep(R_OUT - 7.0, R_OUT, r));
    return edge * pow(R_IN / r, 1.2) * (0.45 + 1.25 * tex);
}

// The hole as seen from page point `c` (page-centred, half-height units, y
// down). `res_y` is the page's height in px, `spin` the disk's clock, `ping`
// the seconds since Enter (<0 none), `hot`/`cool` the disk's inner and outer
// colours.
fn black_hole(c: vec2<f32>, res_y: f32, spin: f32, ping: f32, hot: vec3<f32>, cool: vec3<f32>) -> Hole {
    var h: Hole;
    let hp = vec2<f32>(c.x, -c.y) / HOLE_M;
    let b = length(hp);
    let dir = hp / max(b, 1e-5);
    let sin_i = sqrt(1.0 - TILT * TILT);
    // The ray sweeps the plane through the line of sight and `dir`; it
    // crosses the disk's plane at psi0, then every half-turn after.
    let psi0 = atan2(-TILT, sin_i * dir.y) + PI;
    // A FLARE: every line the user sends drops a ring of light through the
    // disk from its rim to its inner edge, evenly in the log of the radius so
    // it quickens as it falls, and the photon ring flashes as it lands.
    const PING_FALL: f32 = 1.3;
    var ring_r = -1.0;
    var flash = 0.0;
    if (ping >= 0.0) {
        ring_r = R_OUT * pow(R_IN / R_OUT, min(ping / PING_FALL, 1.0));
        ring_r = select(ring_r, -1.0, ping > PING_FALL);
        flash = exp(-pow((ping - PING_FALL) / 0.35, 2.0));
    }
    for (var k = 0; k < 3; k++) {
        let psi = psi0 + f32(k) * PI;
        if (psi >= LUT_PSI_MAX || b >= LUT_B_MAX) {
            break;
        }
        let uu = lut_u(b, psi);
        if (uu < 0.9 / R_OUT || uu > 1.1 / R_IN) {
            continue;
        }
        let r = 1.0 / uu;
        let sp = sin(psi);
        let cp = cos(psi);
        let turns = atan2(TILT * sp * dir.y - sin_i * cp, sp * dir.x) / TAU;
        var lum = disk_light(r, turns, spin);
        if (ring_r > 0.0) {
            lum += 1.4 * exp(-pow((r - ring_r) / 0.8, 2.0)) * smoothstep(0.0, 0.15, ping);
        }
        let a = clamp(lum, 0.0, 1.0);
        let col = mix(hot, cool, smoothstep(R_IN, R_OUT, r));
        h.light += (1.0 - h.cover) * a * col;
        h.cover += (1.0 - h.cover) * a;
    }
    // The photon ring: light that orbited the hole before it got away, a
    // thin bright edge round the shadow and a faint glow beyond it.
    let out = max(b - B_CRIT, 0.0);
    let ring = (exp(-pow((b - 1.012 * B_CRIT) / 0.16, 2.0)) + 0.18 * exp(-out / 2.0))
        * step(B_CRIT, b) * (0.8 + 0.8 * flash);
    let ra = clamp(ring, 0.0, 1.0);
    h.light += (1.0 - h.cover) * ra * hot;
    h.cover += (1.0 - h.cover) * ra;
    // The shadow's edge, smoothed over a pixel.
    let px = 1.0 / (res_y * HOLE_M);
    h.shadow = 1.0 - smoothstep(B_CRIT - px, B_CRIT + px, b);
    // A point lens for the page behind: a ray passing at `b` comes from
    // E²/b nearer the middle — past the middle, inside the Einstein ring —
    // so the weave is pulled into a ring of arcs round the hole.
    const EINSTEIN: f32 = 1.5 * B_CRIT;
    // Inside the shadow nothing behind shows; the pull eases to nothing at
    // the middle there, so the weave never pinches to a point as it fades.
    let pull = EINSTEIN * EINSTEIN * select(1.0 / b, b / (B_CRIT * B_CRIT), b < B_CRIT);
    h.lens = vec2<f32>(-dir.x, dir.y) * pull * HOLE_M * res_y;
    h.b = b;
    return h;
}
