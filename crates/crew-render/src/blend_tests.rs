use super::*;

/// Evaluate one blend component on scalar src/dst values.
fn apply(c: wgpu::BlendComponent, src: f32, src_a: f32, dst: f32) -> f32 {
    use wgpu::BlendFactor as F;
    let f = |f: F| match f {
        F::One => 1.0,
        F::Zero => 0.0,
        F::SrcAlpha => src_a,
        F::OneMinusSrcAlpha => 1.0 - src_a,
        other => panic!("unexpected factor {other:?}"),
    };
    src * f(c.src_factor) + dst * f(c.dst_factor)
}

#[test]
fn partial_coverage_never_makes_the_page_more_transparent() {
    for page in [0.5f32, 0.85, 0.94, 1.0] {
        for sa in [0.1f32, 0.5, 0.9] {
            let a = apply(STRAIGHT_OVER.alpha, sa, sa, page);
            assert!(a >= page - 1e-6, "page {page} coverage {sa} → alpha {a}");
        }
    }
}

#[test]
fn colour_still_blends_straight_alpha() {
    let c = apply(STRAIGHT_OVER.color, 1.0, 0.25, 0.0);
    assert!((c - 0.25).abs() < 1e-6);
}
