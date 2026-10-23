use super::*;

fn card() -> GlassCard {
    GlassCard {
        x: 10.0,
        y: 20.0,
        w: 300.0,
        h: 200.0,
        radius: 10.0,
        alpha_top: 0.2,
        alpha_bottom: 0.09,
        noise: 0.012,
        tint: [0.1, 0.2, 0.3, 1.0],
        highlight: [1.0, 1.0, 1.0, 1.0],
        highlight_alpha: 0.22,
        shadow_alpha: 0.3,
        scan: -1.0,
        edge_glow: 0.35,
        gloss: 0.14,
        glow: 1.0,
        etch: 0.0,
        lift: 0.75,
        glint: 0.5,
        notch: Default::default(),
        see: 0.0,
        liquid: false,
    }
}

/// The shader reads these by fixed offset; a reordering here would show up
/// as garbled cards rather than a compile error, so pin the layout.
#[test]
fn packing_matches_the_shader_layout() {
    let p = pack(&card());
    assert_eq!(&p[0..4], &[10.0, 20.0, 300.0, 200.0], "rect");
    assert_eq!(&p[4..8], &[10.0, 0.2, 0.09, 0.012], "radius/alphas/noise");
    assert_eq!(
        &p[8..12],
        &[0.1, 0.2, 0.3, 0.22],
        "tint.rgb + highlight_alpha"
    );
    assert_eq!(
        &p[12..16],
        &[1.0, 1.0, 1.0, 0.3],
        "highlight.rgb + shadow_alpha"
    );
    assert_eq!(
        &p[16..20],
        &[-1.0, 0.35, 0.75, 0.5],
        "scan + edge_glow + lift + glint"
    );
    let mut c = card();
    c.notch.depth = 9.0;
    c.notch.top[1] = [3.0, 4.0];
    c.notch.bottom[3] = [5.0, 6.0];
    let p = pack(&c);
    assert_eq!(
        &p[20..24],
        &[9.0, 0.14, 1.0, 0.0],
        "notch depth + gloss + glow + etch"
    );
    assert_eq!(&p[26..28], &[3.0, 4.0], "second top span");
    assert_eq!(&p[38..40], &[5.0, 6.0], "last bottom span");
    (c.see, c.liquid) = (0.55, true);
    assert_eq!(
        &pack(&c)[40..44],
        &[0.55, 1.0, 0.0, 0.0],
        "the liquid body, last"
    );
}

/// The vertex buffer stride must match what `pack` produces, or every
/// instance after the first reads misaligned data.
#[test]
fn stride_matches_the_packed_size() {
    assert_eq!(pack(&card()).len(), INSTANCE_FLOATS);
}

/// The quad is expanded by PAD in the shader so the blurred shadow is not
/// clipped to a hard square. Keep that padding ahead of the falloff each
/// shadow layer has to contain at FULL lift — its drop plus a quarter past its
/// blur, where the Gaussian is down to under 2% — asserted against the shader
/// source.
#[test]
fn shadow_padding_covers_its_falloff() {
    let src = include_str!("glass.wgsl");
    let num = |name: &str| -> f32 {
        let at = src
            .find(&format!("const {name}: f32 = "))
            .unwrap_or_else(|| panic!("{name} missing from glass.wgsl"));
        let rest = &src[at + format!("const {name}: f32 = ").len()..];
        let end = rest.find(';').expect("unterminated const");
        rest[..end].trim().parse().expect("non-numeric const")
    };
    let pad = num("PAD");
    let lifted = [
        ("CONTACT", 0.0, 0.0),
        ("AMBIENT", num("LIFT_BLUR"), num("LIFT_DROP")),
    ];
    for (layer, more_blur, more_drop) in lifted {
        let blur = num(&format!("{layer}_BLUR")) + more_blur;
        let drop = num(&format!("{layer}_DROP")) + more_drop;
        assert!(
            pad >= 1.25 * blur + drop,
            "PAD {pad} cannot contain the {layer} layer's {blur}px blur dropped {drop}px"
        );
    }
}

/// The veil the shader thickens behind a legend is the one crew-theme floors
/// the legend inks against: a thinner veil here would leave every glass
/// legend under the floor its colour was chosen for.
#[test]
fn veil_frost_is_the_one_the_legend_inks_assume() {
    let src = include_str!("glass.wgsl");
    let at = src
        .find("const VEIL_FROST: f32 = ")
        .expect("VEIL_FROST missing");
    let rest = &src[at + "const VEIL_FROST: f32 = ".len()..];
    let frost: f32 = rest[..rest.find(';').expect("unterminated")]
        .trim()
        .parse()
        .expect("non-numeric");
    assert_eq!(frost, crew_theme::glasslegend::LEGEND_FROST);
}

/// Nothing lights the patch behind a legend: the frame's holographic light
/// and the gloss lip both ran under a title's lower half, inside the sheet's
/// top edge, and on neon (2026-10-23) put it at 2.5:1 against its upper
/// half's 8:1 (the glass family at 3.4–5.2:1). With both kept off the patch,
/// neon 5.4:1 and the glass family 6.5–7.3:1 over a dark desktop.
#[test]
fn the_frame_light_and_the_gloss_stop_at_a_legend() {
    let src = include_str!("glass.wgsl");
    let line = |start: &str| {
        let at = src.find(start).unwrap_or_else(|| panic!("{start} missing"));
        &src[at..at + src[at..].find(';').expect("unterminated")]
    };
    let bleed = line("let bleed = (1.0 - smoothstep(0.0, GLOW_W, -d)) * inside");
    assert!(bleed.contains("(1.0 - thick)"), "{bleed}");
    let gloss = line("let g = clamp(gloss");
    assert!(gloss.contains("(1.0 - thick)"), "{gloss}");
}

/// No shadow under the words a card's legend puts outside it: a pop-up has
/// no veil there, and its shadow drew a dark rule through the `/keys` hint.
#[test]
fn no_shadow_falls_under_a_legends_outer_half() {
    let src = include_str!("glass.wgsl");
    let at = src
        .find("shadow = clamp(sh_alpha")
        .expect("the shadow term");
    let term = &src[at..at + src[at..].find(';').expect("unterminated")];
    assert!(term.contains("max(body_in, words_out)"), "{term}");
}

/// A sheetless card (a pop-up) has its page laid in under the words on its
/// rule, by the veil's feathered shape, not by a box drawn under them.
#[test]
fn a_pop_ups_rule_words_stand_on_its_page_feathered() {
    let src = include_str!("glass.wgsl");
    assert!(src.contains("let pop = words_out * (1.0 - step(1e-4, a_top + a_bot));"));
    assert!(
        src.contains("vp.page.rgb * pop, out_a)"),
        "the page is mixed in by it"
    );
}
