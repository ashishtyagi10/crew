use super::*;

fn palette() -> Palette {
    Palette::of(crew_theme::theme())
}

#[test]
fn the_wrap_of_the_clock_never_jumps() {
    let mut b = Built::orbit(palette(), 1.6);
    let at = |b: &mut Built, p: f32| {
        b.pose(p);
        let k = b.scene.get(b.knot).unwrap();
        let c = b.scene.get(b.crystals[3]).unwrap();
        (k.quaternion, c.position, c.quaternion)
    };
    let (k0, p0, q0) = at(&mut b, 0.0);
    let (k1, p1, q1) = at(&mut b, 1.0);
    let near = |a: f32, b: f32| (a - b).abs() < 1e-3;
    // A quaternion and its negation are the same turn.
    let same_q = |a: threers::Quaternion, b: threers::Quaternion| {
        let d = a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w;
        near(d.abs(), 1.0)
    };
    assert!(same_q(k0, k1), "the knot");
    assert!(
        near(p0.x, p1.x) && near(p0.y, p1.y) && near(p0.z, p1.z),
        "a crystal's place"
    );
    assert!(same_q(q0, q1), "a crystal's spin");
}

#[test]
fn the_light_is_the_themes() {
    let p = palette();
    let t = crew_theme::theme();
    assert_eq!(
        (p.key, p.fill, p.page, p.dark),
        (t.accent_default, t.activity, t.page_bg, t.dark)
    );
    assert_eq!(color((255, 255, 255)).r, 1.0);
    assert!(color((128, 128, 128)).g < 0.5, "light is linear");
}
