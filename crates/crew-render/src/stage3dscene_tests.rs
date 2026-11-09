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

/// Where `p` lands on screen, in NDC, through the scene's own camera.
fn ndc(b: &Built, p: threers::Vector3) -> (f32, f32) {
    let vp = b
        .camera
        .projection_matrix()
        .multiply(&b.camera.view_matrix());
    let q = p.apply_matrix4(&vp);
    (q.x, q.y)
}

/// The orbit is placed by the frustum, not in world units: its centre stays
/// right of the middle and in frame on a tall window and an ultrawide one
/// (at a fixed x = 2.3 a 0.5 window put it past the right edge and a 3.0 one
/// under the text, NDC 0.26), and every crystal of its ring stays in frame.
#[test]
fn the_orbit_stays_right_and_in_frame_at_any_aspect() {
    for aspect in [0.5, 0.6, 1.0, 1.6, 2.4, 3.0] {
        let mut b = Built::orbit(palette(), aspect);
        let (centre, _) = b.frame();
        let (x, y) = ndc(&b, centre);
        assert!(
            (0.35..0.85).contains(&x) && y < 0.0,
            "the knot at aspect {aspect}: ({x:.2}, {y:.2})"
        );
        for phase in [0.0, 0.2, 0.45, 0.7, 0.9] {
            b.pose(phase);
            for (i, id) in b.crystals.iter().enumerate() {
                let (x, y) = ndc(&b, b.scene.get(*id).unwrap().position);
                assert!(
                    x.abs() < 0.95 && y.abs() < 0.95,
                    "crystal {i} at aspect {aspect}, phase {phase}: ({x:.2}, {y:.2})"
                );
            }
        }
    }
}
