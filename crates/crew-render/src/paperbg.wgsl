struct Uniform {
    page_bg: vec4<f32>,
    resolution: vec2<f32>,
    intensity: f32,
    grain_mul: f32,   // scales additive grain amplitude (0 = no grain, 1 = default)
    dot_a: vec4<f32>,    // pole A tint; a = lattice strength (0 = no dots)
    dot_b: vec4<f32>,    // pole B tint; a = dot radius px
    dot_grid: vec4<f32>, // xy = lattice pitch px; z = wash strength (0 = no wash); w = wash phase (turns)
    wash_focus: vec4<f32>, // xy = orbit centre in uv; z = how far it moves there (0 = page centre); w = wander clock (turns)
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

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    // UV in [0, 1] with (0,0) at top-left.
    let uv = in.pos.xy / u.resolution;

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

    // Aspect ratio, for the wash's round pools and the glint's 45° band.
    let asp = u.resolution.x / max(u.resolution.y, 1.0);
    // The wash's orbit, in radians — the clock every moving part of the
    // backdrop keys off, so the pools, the lattice's tint and the glint never
    // drift out of step with each other.
    let ang = 6.2831853 * u.dot_grid.w;

    // The modern family's gradient wash (dot_grid.z = 0 everywhere else):
    // two broad pools of pole light lying under the whole page — the aurora
    // the dot lattice is then woven on top of. Coordinates are aspect-
    // corrected (half-height units) so a pool is round on any window, and the
    // pair sits on an elliptical orbit that hugs the page: at phase 0 pole A
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
        // The orbit's centre. At pull 0 it is the page centre and the pools
        // sit either side of the middle, as they always have; as the app
        // raises it the whole pair slides toward the focused card, so the
        // page's light gathers where the work is. Same half-height units as
        // `p`, so a pull reads the same on any window shape.
        let fc = vec2<f32>(
            (u.wash_focus.x - 0.5) * asp,
            u.wash_focus.y - 0.5,
        ) * u.wash_focus.z;
        let p = vec2<f32>((uv.x - 0.5) * asp, uv.y - 0.5) - fc;
        let breath = sin(2.0 * ang);
        // Integer harmonics of the wander clock, so its wrap is seamless;
        // three and five never line up inside a cycle, so lean and reach
        // read as one unhurried drift rather than two metronomes.
        let wander = 6.2831853 * u.wash_focus.w;
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
        let ga = pow(1.0 - smoothstep(0.0, ra, length(p + orbit_a)), 3.0);
        let gb = pow(1.0 - smoothstep(0.0, rb, length(p - orbit_b)), 3.0);
        let trade = TRADE * (0.5 - 0.5 * cos(4.0 * ang));
        let col_a = mix(u.dot_a.rgb, u.dot_b.rgb, trade);
        let col_b = mix(u.dot_b.rgb, u.dot_a.rgb, trade);
        rgb = mix(rgb, col_a, ga * wash_amp * (1.0 + BREATH_AMP * breath));
        rgb = mix(rgb, col_b, gb * wash_amp * (1.0 - BREATH_AMP * breath));
    }

    // The lattice's gradient and the GLINT, shared by the sheen below and the
    // dots after it.
    //
    // The tint axis turns with the orbit — pole A's end starts at the top-left
    // corner and follows pool A round. Projection onto it is scaled by the
    // axis's reach to the farthest corner so every angle runs the full
    // pole_a→pole_b span (at 45° this is exactly the old `(uv.x + uv.y) / 2`).
    let axis = vec2<f32>(cos(ang + 0.78539816), sin(ang + 0.78539816));
    let span = abs(axis.x) + abs(axis.y);
    let diag = clamp(0.5 + dot(uv - vec2<f32>(0.5), axis) / span, 0.0, 1.0);
    let tint = mix(u.dot_a.rgb, u.dot_b.rgb, diag);
    // The glint: a band of light that sweeps the page corner to corner twice
    // a revolution, the weave catching the light as it turns. Its crest runs
    // −0.5 → 1.5 along the diagonal, so it enters and leaves fully off the
    // page, the wrap is never seen, a quarter of each sweep is rest, and a
    // page at rest (phase 0) wears none of it. Aspect-corrected, so the band
    // runs at 45° in pixels on any window.
    const GLINT_W: f32 = 0.22;
    let sweep = fract(2.0 * u.dot_grid.w) * 2.0 - 0.5;
    let along = (uv.x * asp + uv.y) / (asp + 1.0);
    let glint = pow(1.0 - smoothstep(0.0, GLINT_W, abs(along - sweep)), 2.0);
    // The SHEEN: the glint lights the page itself, faintly, in the lattice's
    // colour. Keyed to the wash's own strength — which the frame has already
    // scaled for the OS contrast setting — at half of it, so the band never
    // spends more of the text's headroom than a pool does.
    const SHEEN: f32 = 0.5;
    rgb = mix(rgb, tint, glint * wash_amp * SHEEN);

    // The modern family's dot lattice (dot_a.a = 0 everywhere else): soft
    // round dots on a grid whose pitch rides the text-cell metrics (set by
    // frame.rs), in the turning tint above so the backdrop carries the
    // theme's gradient identity. A mix toward the tint (not an add) so the
    // same strength reads on any page brightness. Under the glint's crest a
    // dot carries up to four times its resting strength.
    let dot_amp = u.dot_a.a;
    if (dot_amp > 0.0) {
        const GLINT_GAIN: f32 = 3.0;
        let pitch = u.dot_grid.xy;
        let off = (fract(in.pos.xy / pitch) - vec2<f32>(0.5)) * pitch; // px from dot centre
        let d = length(off);
        let r = u.dot_b.a;
        // ±0.8px feathered edge — soft, never a hard aliased circle.
        let mask = 1.0 - smoothstep(r - 0.8, r + 0.8, d);
        rgb = mix(rgb, tint, mask * min(dot_amp * (1.0 + GLINT_GAIN * glint), 1.0));
    }
    // Alpha comes from the page colour, not a hard 1.0: it carries the window
    // opacity, so a translucent window lets the desktop through the paper while
    // text and pane fills (which blend on top) stay solid.
    return vec4<f32>(rgb, u.page_bg.a);
}
