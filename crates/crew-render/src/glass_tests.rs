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
        &[-1.0, 0.35, 0.0, 0.0],
        "scan + edge_glow + pad"
    );
}

/// The vertex buffer stride must match what `pack` produces, or every
/// instance after the first reads misaligned data.
#[test]
fn stride_matches_the_packed_size() {
    assert_eq!(pack(&card()).len(), INSTANCE_FLOATS);
}

/// The quad is expanded by PAD in the shader so the blurred shadow is not
/// clipped to a hard square. Keep that padding ahead of the falloff it has
/// to contain (blur + drop), which this asserts against the shader source.
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
    let (pad, blur, drop) = (num("PAD"), num("SH_BLUR"), num("SH_DROP"));
    assert!(
        pad >= blur + drop,
        "PAD {pad} cannot contain a {blur}px blur dropped {drop}px"
    );
}
