use super::*;

fn palette() -> Palette {
    Palette::of(crew_theme::ThemeId::Prism.theme())
}

/// Where solid `i` lands on screen, in NDC, through the scene's own camera.
fn ndc(b: &Prisms, i: usize) -> (f32, f32) {
    let vp = b
        .camera
        .projection_matrix()
        .multiply(&b.camera.view_matrix());
    let p = b.solid(i).position.apply_matrix4(&vp);
    (p.x, p.y)
}

#[test]
fn the_wrap_of_the_clock_never_jumps() {
    let mut b = Prisms::new(palette(), 1.6);
    let near = |a: f32, b: f32| (a - b).abs() < 1e-3;
    for i in 0..SEEDS.len() {
        b.pose(0.0);
        let (p0, q0) = (b.solid(i).position, b.solid(i).quaternion);
        b.pose(1.0);
        let (p1, q1) = (b.solid(i).position, b.solid(i).quaternion);
        assert!(
            near(p0.x, p1.x) && near(p0.y, p1.y) && near(p0.z, p1.z),
            "solid {i}'s place"
        );
        // A quaternion and its negation are the same turn.
        let d = q0.x * q1.x + q0.y * q1.y + q0.z * q1.z + q0.w * q1.w;
        assert!(near(d.abs(), 1.0), "solid {i}'s turn");
    }
}

/// Every solid stays in frame on a tall window and an ultrawide one, at any
/// point of the clock: they are placed by the frustum, not in world units.
#[test]
fn every_prism_stays_in_frame_at_any_aspect() {
    for aspect in [0.6, 1.0, 1.6, 2.4, 3.0] {
        let mut b = Prisms::new(palette(), aspect);
        for phase in [0.0, 0.3, 0.7] {
            b.pose(phase);
            for i in 0..SEEDS.len() {
                let (x, y) = ndc(&b, i);
                assert!(
                    x.abs() < 0.95 && y.abs() < 0.95,
                    "solid {i} at aspect {aspect}, phase {phase}: ({x:.2}, {y:.2})"
                );
            }
        }
    }
}

/// The scene fills the frame rather than one corner: solids on both sides
/// of the middle and above and below it, at three depths.
#[test]
fn the_prisms_fill_the_frame_at_three_depths() {
    let mut b = Prisms::new(palette(), 1.6);
    b.pose(0.0);
    let at: Vec<(f32, f32)> = (0..SEEDS.len()).map(|i| ndc(&b, i)).collect();
    assert!(at.iter().any(|p| p.0 < 0.0) && at.iter().any(|p| p.0 > 0.5));
    assert!(at.iter().any(|p| p.1 < -0.3) && at.iter().any(|p| p.1 > 0.3));
    let mut depths: Vec<i32> = SEEDS
        .iter()
        .filter(|s| !s.gem)
        .map(|s| s.z as i32)
        .collect();
    depths.dedup();
    assert_eq!(depths.len(), 3, "{depths:?}");
}

/// Its colours are the theme's: the solids take the accent, the activity and
/// the ring's poles, and the fog is the page, so the far ones sink into it.
#[test]
fn it_is_lit_and_fogged_in_the_themes_colours() {
    let t = crew_theme::ThemeId::Prism.theme();
    let p = palette();
    let m = t.modern.expect("prism has a ring");
    assert_eq!(p.ring, (m.pole_a, m.pole_b));
    let b = Prisms::new(p, 1.6);
    let fog = b.scene.fog;
    assert_eq!(fog.mode, 1, "linear fog");
    assert_eq!(
        (fog.color.r, fog.color.g),
        (color(t.page_bg).r, color(t.page_bg).g)
    );
    assert!(
        fog.near < EYE - 1.5 && fog.far > EYE + 5.0,
        "near ones clear, far ones fade"
    );
}
