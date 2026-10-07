struct Uniform {
    page_bg: vec4<f32>,
    resolution: vec2<f32>,
    intensity: f32,
    grain_mul: f32,   // scales additive grain amplitude (0 = no grain, 1 = default)
    dot_a: vec4<f32>,    // pole A tint; a = lattice strength (0 = no dots)
    dot_b: vec4<f32>,    // pole B tint; a = dot radius px
    dot_grid: vec4<f32>, // xy = lattice pitch px; z = wash strength (0 = no wash); w = wash phase (turns)
    wash_focus: vec4<f32>, // xy = orbit centre in uv; z = how far it moves there (0 = page centre); w = wander clock (turns)
    motion: vec4<f32>,     // x = how awake the page is (0 = the still page); y = eddy clock (turns); z = 1 for liquid glass's wallpaper; w spare
}
@group(0) @binding(0) var<uniform> u: Uniform;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
}

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    // Fullscreen triangle — covers the entire NDC cube with 3 vertices, no VB.
    var pts = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var out: VsOut;
    out.pos = vec4<f32>(pts[vi], 0.0, 1.0);
    return out;
}

// Deterministic per-pixel luminance hash — pure function of pixel coordinates.
fn grain(px: vec2<f32>) -> f32 {
    return fract(sin(dot(px, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

// Smooth value noise on the grain hash: 0..1, one feature per unit. Quintic
// easing between the lattice points, so a warp built on it bends smoothly
// instead of creasing where the cubic's curvature jumps.
fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let w = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = grain(i);
    let b = grain(i + vec2<f32>(1.0, 0.0));
    let c = grain(i + vec2<f32>(0.0, 1.0));
    let d = grain(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, w.x), mix(c, d, w.x), w.y);
}

// One soft field of colour on the wallpaper: Gaussian, so fields melt into
// each other with no edge anywhere.
fn field(p: vec2<f32>, c: vec2<f32>, r: f32) -> f32 {
    let d = length(p - c) / r;
    return exp(-d * d);
}

// Liquid glass's wallpaper: the iPhone's kind — a few broad, saturated fields
// of colour melting into one another over a deep page, so whatever a pane is
// laid over, its glass has colour to bend. Pole A and pole B hold opposite
// corners, their blends the other two; each drifts round a small orbit on the
// wash's clocks, so the page is still when they are and slides as a lit room
// would when they run. Coverage tops out at the wash's strength, so the page
// colour always shows a little — the field the glass's legibility guard
// (`liquid_text_reads_on_its_glass`) is measured against.
fn wallpaper(uv: vec2<f32>) -> vec3<f32> {
    let asp = u.resolution.x / max(u.resolution.y, 1.0);
    let p = vec2<f32>((uv.x - 0.5) * asp, uv.y - 0.5);
    let ang = 6.2831853 * u.dot_grid.w;
    let wnd = 6.2831853 * u.wash_focus.w;
    let a_at = vec2<f32>(-0.42 * asp, -0.28) + 0.10 * vec2<f32>(cos(ang), sin(ang));
    let b_at = vec2<f32>(0.40 * asp, 0.30) + 0.10 * vec2<f32>(cos(ang + 2.1), sin(ang + 2.1));
    let c_at = vec2<f32>(0.34 * asp, -0.34) + 0.08 * vec2<f32>(cos(wnd), sin(1.3 * wnd));
    let d_at = vec2<f32>(-0.28 * asp, 0.38) + 0.08 * vec2<f32>(sin(wnd), cos(ang));
    let wa = field(p, a_at, 0.62);
    let wb = field(p, b_at, 0.62);
    let wc = 0.8 * field(p, c_at, 0.48);
    let wd = 0.7 * field(p, d_at, 0.44);
    let ca = u.dot_a.rgb;
    let cb = u.dot_b.rgb;
    let cc = mix(ca, cb, 0.55);
    let cd = mix(cb, ca, 0.30);
    let w = wa + wb + wc + wd;
    let col = (ca * wa + cb * wb + cc * wc + cd * wd) / max(w, 1e-4);
    // The legibility gradient the iPhone lays behind its status bar: the
    // wallpaper deepens toward the window's top and bottom edges, where pane
    // titles and the input bar's tag stand on it rather than on glass.
    let edge = min(uv.y, 1.0 - uv.y);
    let shade = mix(0.4, 1.0, smoothstep(0.0, 0.07, edge));
    let cover = clamp(w, 0.0, 1.0) * u.dot_grid.z * shade;
    return mix(u.page_bg.rgb, col, cover);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    // UV in [0, 1] with (0,0) at top-left.
    let uv = in.pos.xy / u.resolution;
    if (u.motion.z > 0.5) {
        return vec4<f32>(wallpaper(uv), u.page_bg.a);
    }

    // Radial vignette: ~5% darker at corners (d2 = 0.5 at corner → 0.95).
    // Multiplicative on the page colour, so it scales with brightness.
    let d2 = dot(uv - vec2<f32>(0.5), uv - vec2<f32>(0.5));
    let vignette = 1.0 - d2 * 0.1;
    let base = u.page_bg.rgb * (vignette * u.intensity + (1.0 - u.intensity));

    // Fine octave: per-pixel hash — the newsprint speckle. Scaled by
    // grain_mul and gated by intensity.
    let n = (grain(in.pos.xy) - 0.5) * u.grain_mul * u.intensity;
    // Coarse octave: the same hash on 2.5px-quantized coords — value-noise
    // blotches at paper-fiber scale. Only the dark absolute term uses it
    // (dark_weight-gated below), so light pages keep their pure speckle and
    // the multiplicative light-page term (n * 0.05) is untouched.
    let n2 = (grain(floor(in.pos.xy / 2.5)) - 0.5) * u.grain_mul * u.intensity;
    // Hybrid grain so the texture reads on BOTH themes: a multiplicative term
    // gives the bright "paper" page its grain (an absolute term would be
    // imperceptible there), and a small absolute term gives the near-black
    // "newspaper" page visible texture (a purely multiplicative grain vanishes
    // on it).
    //
    // Gamma-space tuning: this pass now writes directly to a non-sRGB target
    // (see gpu.rs `pick_surface_format` — glyphon ColorMode::Web needs gamma-
    // space blending), so there is no sRGB encode gain on write. The old
    // 0.0015 absolute amplitude was tuned for a LINEAR page colour headed to
    // an sRGB target, where near-black values gained ~13x on encode; on the
    // non-sRGB path that gain is gone, so the same constant read as
    // essentially flat on dark pages.
    //
    // 0.048 gave the near-black "newspaper" page the SAME newsprint spread
    // the bright paper page has (light-page std ≈ 5.67 (measured at the 1.56 dark drive)), so the dark themes
    // read as textured newsprint rather than flat black — the deliberate
    // choice after dark themes previously kept a much subtler ~±3-level grain.
    // Dark themes now also carry theme.grain 1.2 (matching light), so the two
    // appearances share one grain identity. Measured by rendering page_bg
    // (8,8,8) at grain_mul 1.56 (knob default 1.3 × theme.grain 1.2) and
    // sampling per-pixel R stddev over a flat center region.
    //
    // The absolute term is weighted down by page brightness (`dark_weight`)
    // so it stays negligible on light pages — without this, the constant
    // would stack on the already-calibrated light-page multiplicative grain,
    // since the absolute and multiplicative terms share the same noise sample
    // and add. `dark_weight` ≈ 1 near black and ≈ 0.05 on the paper-light
    // page_bg, keeping the light-page spread essentially unchanged.
    //
    // Coarse fiber octave: the single-octave grain above read as pure
    // per-pixel speckle on dark pages, not paper fiber — `n2` above is the
    // same hash sampled on 2.5px-quantized coordinates, so it produces
    // value-noise blotches a few pixels wide instead of independent
    // per-pixel noise. It is blended into the SAME dark_weight-gated
    // absolute term as `n`, so light pages (dark_weight ≈ 0.05) stay
    // statistically unchanged; the light-page multiplicative term
    // (n * 0.05 above) is untouched. `A_DARK` replaces the old bare 0.048
    // absolute-amplitude literal; the fine/coarse weights 0.5/0.7 split the
    // dark-page grain energy between the two octaves, coarse weighted
    // slightly higher since it is what reads as "fiber" after downsampling.
    // Calibrated with the headless 64x64 Metal harness
    // (crates/crew-render/tests/paperbg_headless.rs), rendering the same
    // (8,8,8)/grain_mul-1.56 page as above and reading both per-pixel stddev
    // (`dark_std`, target band 5-7) and the stddev after a 2x2 box-downsample
    // (`dark_coarse`, target ratio dark_coarse/dark_std >= 0.6 — pure white
    // noise collapses to ~0.5x under 2x2 averaging, so clearing that floor
    // with margin is the "fiber survives downsampling" signal). Pre-octave
    // baseline (single hash sample, old 0.048 literal): dark_std=5.19,
    // dark_coarse=2.16 (ratio 0.42 — fails the floor, reads as speckle not
    // fiber). A_DARK=0.075 with fine/coarse weights 0.5/0.7 landed both
    // targets on the first try, so no further iteration was needed: measured
    // dark_std=6.40 (in the 5-7 band), dark_coarse=4.19 (ratio 0.65, clears
    // the 0.6 margin). Light page (Cases 1-3, grain_mul<=2.4) statistically
    // unaffected: light_std=3.56 (at grain_mul 1.0), light_coarse=1.49 (ratio 0.42, well under
    // the 0.7 white-noise ceiling — dark_weight keeps n2 negligible there).
    const A_DARK: f32 = 0.075;
    let page_luma = dot(u.page_bg.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let dark_weight = 1.0 - page_luma;
    var rgb = clamp(
        base * (1.0 + n * 0.05)
            + vec3<f32>((n * 0.5 + n2 * 0.7) * A_DARK * dark_weight),
        vec3<f32>(0.0), vec3<f32>(1.0));

    // Aspect ratio, for the wash's round pools.
    let asp = u.resolution.x / max(u.resolution.y, 1.0);
    // The wash's orbit, in radians — the clock every moving part of the
    // backdrop keys off, so the pools, the lattice's tint and the glow never
    // drift out of step with each other.
    let ang = 6.2831853 * u.dot_grid.w;
    // The second, slower clock (the hue breath's), in radians.
    let wander = 6.2831853 * u.wash_focus.w;

    // The orbit's centre. At pull 0 it is the page centre and the pools sit
    // either side of the middle, as they always have; as the app raises it
    // the whole pair slides toward the focused card, so the page's light
    // gathers where the work is. Half-height units (y spans ±0.5, x ±asp/2),
    // so a pull reads the same on any window shape.
    let fc = vec2<f32>(
        (u.wash_focus.x - 0.5) * asp,
        u.wash_focus.y - 0.5,
    ) * u.wash_focus.z;
    let c = vec2<f32>((uv.x - 0.5) * asp, uv.y - 0.5) - fc;

    // The FLOW: the page's light moves like liquid, not like a rigid card.
    // Every moving part below — the pools, the lattice's tint and the glow —
    // is drawn on these bent coordinates rather than the page's straight
    // ones, so nothing in the backdrop moves in a straight line.
    //
    // Two bends, both spread evenly over the page. A CURRENT: two travelling waves
    // crossing at right angles nudge every point along the other axis, so a
    // pool's edge never sits still. Then EDDIES: a slow drift of noise —
    // smoke, not sine — so the pools' edges and the glow's ring wisp rather
    // than rippling in step. Two octaves, the finer one warped by the coarser (a domain warp:
    // eddies inside eddies). The noise is sampled through a window that
    // circles round the noise plane on the third clock, so its loop is
    // seamless, and that clock runs at an irrational ratio to the other two,
    // so the page never comes back to a frame it has drawn.
    //
    // `flow` is how far the page has come from its still geometry: how AWAKE
    // it is (`motion.x`), which the app eases from 0 to 1 over the first
    // seconds of drift and then holds. It never dips back on a clock, so a
    // page that has never drifted is exactly the still orbit, and the
    // orbit's own effects (breath, trade, tint) measure on a sleeping page's
    // straight coordinates. The waves' phases are whole multiples of the
    // clocks, so every wrap is seamless.
    const CURRENT: f32 = 0.06;
    const CURRENT_K: f32 = 3.5;
    let flow = u.motion.x;
    var q = c;
    q += CURRENT * flow * vec2<f32>(
        sin(CURRENT_K * q.y + ang + wander),
        sin(CURRENT_K * q.x - 2.0 * ang),
    );
    const EDDY: f32 = 0.10;
    const EDDY_K: f32 = 2.0;
    let ed = 6.2831853 * u.motion.y;
    let lap = 1.7 * vec2<f32>(cos(ed), sin(ed));
    let p1 = q * EDDY_K + lap;
    let e1 = vec2<f32>(vnoise(p1), vnoise(p1 + vec2<f32>(5.2, 1.3))) - vec2<f32>(0.5);
    let p2 = q * (2.1 * EDDY_K) + 2.5 * e1 - lap.yx;
    let e2 = vec2<f32>(vnoise(p2 + vec2<f32>(1.7, 9.2)), vnoise(p2 + vec2<f32>(8.3, 2.8)))
        - vec2<f32>(0.5);
    q += EDDY * flow * (e1 + 0.4 * e2);
    // The same bend in uv, for the lattice's tint. Added as a
    // displacement so a page at rest keeps its exact uv.
    let dq = q - c;
    let fuv = uv + vec2<f32>(dq.x / asp, dq.y);

    // The modern family's gradient wash (dot_grid.z = 0 everywhere else):
    // two broad pools of pole light lying under the whole page — the aurora
    // the dot lattice is then woven on top of. Drawn on the flow's bent
    // coordinates (aspect-corrected, half-height units), so a still pool is
    // round on any window and a moving one curls; the pair sits on an
    // elliptical orbit that hugs the page: at phase 0 pole A
    // is at the left edge and pole B at the right, and a quarter turn later
    // they have swung clockwise to the top and the bottom.
    //
    // On that orbit the pools also BREATHE and WANDER. Breath is two swells
    // a revolution, in counter-phase: as one pool widens and brightens the
    // other narrows and dims, so the light moves between the poles rather
    // than the page pulsing as a whole. Wander rides the second, slower clock
    // (`wash_focus.w`, the hue breath's): the pools lean toward each other on
    // one side of the orbit and apart on the other, and reach in and out
    // from the centre, so they meet, mix and part instead of turning as one
    // rigid bar. And they TRADE colour: between the cardinal points of the
    // orbit each pool leans toward the other's pole, so the gradient itself
    // keeps changing, not just where it lies. Every term is a sine of a phase
    // that is 0 at rest, so a page that has never drifted is exactly the
    // still orbit — and the phase comes from the app, so a held frame is
    // still a pure function of position.
    let wash_amp = u.dot_grid.z;
    if (wash_amp > 0.0) {
        // Pool radius, in half-height units: wide enough that a pool covers
        // its own side of the page with no visible edge, short enough to fall
        // to nothing before the far side.
        const WASH_R: f32 = 0.95;
        // How far out the pools orbit, as a fraction of each half-axis.
        const WASH_ORBIT: f32 = 0.45;
        // How much a breath widens a pool's radius and lifts its strength at
        // the top of the swell (and narrows/dims the other pool as much).
        const BREATH_R: f32 = 0.10;
        const BREATH_AMP: f32 = 0.25;
        // The wander's reach: how far (radians) each pool leans off the
        // straight line through the centre, and how far (fraction of the
        // orbit) the pair reaches in and out.
        const WANDER_LEAN: f32 = 0.45;
        const WANDER_REACH: f32 = 0.15;
        // How far toward the other pole a pool's colour leans at the peak of
        // the trade (four a revolution, back to its own at every quarter).
        const TRADE: f32 = 0.35;
        let breath = sin(2.0 * ang);
        // Integer harmonics of the wander clock, so its wrap is seamless;
        // three and five never line up inside a cycle, so lean and reach
        // read as one unhurried drift rather than two metronomes.
        let lean = WANDER_LEAN * sin(3.0 * wander);
        let reach = WASH_ORBIT * (1.0 + WANDER_REACH * sin(5.0 * wander));
        let ang_a = ang + lean;
        let ang_b = ang - lean;
        let orbit_a = vec2<f32>(cos(ang_a) * reach * asp, sin(ang_a) * reach);
        let orbit_b = vec2<f32>(cos(ang_b) * reach * asp, sin(ang_b) * reach);
        // Cubic falloff — a soft shoulder rather than smoothstep's linear
        // middle, so each pool reads as light with a core rather than a
        // painted disc, and the page between them dips to roughly a third of
        // a pool's lift instead of washing out flat.
        let ra = WASH_R * (1.0 + BREATH_R * breath);
        let rb = WASH_R * (1.0 - BREATH_R * breath);
        let ga = pow(1.0 - smoothstep(0.0, ra, length(q + orbit_a)), 3.0);
        let gb = pow(1.0 - smoothstep(0.0, rb, length(q - orbit_b)), 3.0);
        let trade = TRADE * (0.5 - 0.5 * cos(4.0 * ang));
        let col_a = mix(u.dot_a.rgb, u.dot_b.rgb, trade);
        let col_b = mix(u.dot_b.rgb, u.dot_a.rgb, trade);
        rgb = mix(rgb, col_a, ga * wash_amp * (1.0 + BREATH_AMP * breath));
        rgb = mix(rgb, col_b, gb * wash_amp * (1.0 - BREATH_AMP * breath));
    }

    // The lattice's gradient, shared by the glow below and the dots.
    //
    // The tint axis turns with the orbit — pole A's end starts at the top-left
    // corner and follows pool A round. Projection onto it is scaled by the
    // axis's reach to the farthest corner so every angle runs the full
    // pole_a→pole_b span (at 45° this is exactly the old `(uv.x + uv.y) / 2`).
    // Read off the flow's bent uv, so the tint's bands bow with the current.
    let axis = vec2<f32>(cos(ang + 0.78539816), sin(ang + 0.78539816));
    let span = abs(axis.x) + abs(axis.y);
    let diag = clamp(0.5 + dot(fuv - vec2<f32>(0.5), axis) / span, 0.0, 1.0);
    let tint = mix(u.dot_a.rgb, u.dot_b.rgb, diag);

    // The GLOW: a soft light at the middle of the page that beats slowly,
    // like a resting pulse, and radiates. On each beat its CORE swells and
    // brightens, and a soft RING of light leaves it and travels outward,
    // widening and fading as it goes, until it is gone near the page's rim —
    // then the core rests a few seconds before the next. A quiet HALO
    // round the core holds the light between beats.
    //
    // It replaced first a spiral that poured into the middle (hypnotic, and
    // so distracting) and then folds of silk across the page (lines, still
    // too busy). There are no lines in it: every part is a Gaussian of the
    // distance from the middle, so it reads as light, not as a pattern.
    //
    // It sits at the PAGE's middle — not the pools' orbit centre, which
    // leans toward the focused card — and the eddies bend it only a little
    // (a third as far as the pools), so the ring's edge is never a
    // compass-drawn circle but the glow never wanders. Its colour runs from
    // pole A at the core to pole B at the rim, so a ring changes colour as it
    // travels. BEATS whole beats per orbit, so the orbit's wrap is seamless:
    // six seconds a beat on a quiet page, three while a pane works. It wakes
    // with the page (`flow`), so a still page wears none of it.
    const BEATS: f32 = 4.0;
    const CORE_R: f32 = 0.13; // the core's radius, in page heights
    const HALO_R: f32 = 0.42;
    const HALO: f32 = 0.25;
    const RING: f32 = 0.75;
    const RING_W: f32 = 0.08; // the ring's width as it leaves the core
    const REACH: f32 = 1.1; // how far out a ring has gone when it is spent
    let gp = vec2<f32>((uv.x - 0.5) * asp, uv.y - 0.5) + 0.3 * (q - c);
    let gr = length(gp);
    let t = fract(BEATS * u.dot_grid.w);
    let beat = 0.5 + 0.5 * cos(6.2831853 * t);
    let core = exp(-pow(gr / CORE_R, 2.0)) * mix(0.55, 1.0, beat);
    let halo = HALO * exp(-pow(gr / HALO_R, 2.0)) * mix(0.8, 1.0, beat);
    // The ring is born inside the core (faded in, so it never pops) and
    // dies at REACH (faded out, so the wrap is seamless).
    let ring_w = RING_W * (1.0 + 2.0 * t);
    let ring = RING * exp(-pow((gr - REACH * t) / ring_w, 2.0))
        * smoothstep(0.0, 0.12, t) * pow(1.0 - t, 1.2);
    let glint = flow * min(core + halo + ring, 1.0);
    let glow_col = mix(u.dot_a.rgb, u.dot_b.rgb, smoothstep(0.0, 0.7, gr));
    // The light the glow casts: its own colour on an awake page, the
    // lattice's tint on a still one.
    let glow = mix(tint, glow_col, flow);
    // The SHEEN: the glow lights the page itself, in its own colours. Keyed
    // to the wash's own strength — which the frame has already scaled for
    // the OS contrast setting — and below it, so even the core at the top of
    // its beat never spends more of the text's headroom than a pool does.
    const SHEEN: f32 = 0.9;
    rgb = mix(rgb, glow, glint * wash_amp * SHEEN);

    // The modern family's dot lattice (dot_a.a = 0 everywhere else): soft
    // round dots on a grid whose pitch rides the text-cell metrics (set by
    // frame.rs), in the turning tint above so the backdrop carries the
    // theme's gradient identity. A mix toward the tint (not an add) so the
    // same strength reads on any page brightness. Under the glow a dot
    // carries up to about three times its resting strength, takes the glow's
    // colour and swells a little, so the lattice is a halftone of the light
    // and a ring passes across the weave as a wave of brightening dots. Half
    // as hard on a light page, where darkened dots sit right under dark ink.
    let dot_amp = u.dot_a.a;
    if (dot_amp > 0.0) {
        let gain = 2.2 * mix(0.5, 1.0, dark_weight);
        let swell = 0.6 * mix(0.6, 1.0, dark_weight);
        let pitch = u.dot_grid.xy;
        let off = (fract(in.pos.xy / pitch) - vec2<f32>(0.5)) * pitch; // px from dot centre
        let d = length(off);
        let r = u.dot_b.a * (1.0 + swell * glint);
        // ±0.8px feathered edge — soft, never a hard aliased circle.
        let mask = 1.0 - smoothstep(r - 0.8, r + 0.8, d);
        let ink = mix(tint, glow, min(2.0 * glint, 1.0));
        rgb = mix(rgb, ink, mask * min(dot_amp * (1.0 + gain * glint), 1.0));
    }
    // Alpha comes from the page colour, not a hard 1.0: it carries the window
    // opacity, so a translucent window lets the desktop through the paper while
    // text and pane fills (which blend on top) stay solid.
    return vec4<f32>(rgb, u.page_bg.a);
}
