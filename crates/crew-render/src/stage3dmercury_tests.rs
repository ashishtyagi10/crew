use super::*;

fn palette() -> Palette {
    Palette::of(crew_theme::ThemeId::Mercury.theme())
}

/// Where lobe `i` of pool `k` lands on screen, in NDC, through the scene's
/// own camera.
fn ndc(b: &Mercury, k: usize, i: usize) -> (f32, f32) {
    let vp = b
        .camera
        .projection_matrix()
        .multiply(&b.camera.view_matrix());
    let p = b.lobe(k, i).position.apply_matrix4(&vp);
    (p.x, p.y)
}

fn every_lobe() -> impl Iterator<Item = (usize, usize)> {
    POOLS
        .iter()
        .enumerate()
        .flat_map(|(k, p)| (0..p.lobes).map(move |i| (k, i)))
}

#[test]
fn the_wrap_of_the_clock_never_jumps() {
    let mut b = Mercury::new(palette(), 1.6);
    let near = |a: f32, b: f32| (a - b).abs() < 1e-3;
    for (k, i) in every_lobe() {
        b.pose(0.0);
        let (p0, s0) = (b.lobe(k, i).position, b.lobe(k, i).scale);
        b.pose(1.0);
        let (p1, s1) = (b.lobe(k, i).position, b.lobe(k, i).scale);
        assert!(
            near(p0.x, p1.x) && near(p0.y, p1.y) && near(p0.z, p1.z),
            "lobe {k}.{i}'s place"
        );
        assert!(
            near(s0.x, s1.x) && near(s0.y, s1.y) && near(s0.z, s1.z),
            "lobe {k}.{i}'s wobble"
        );
    }
}

/// The lobes of a pool always overlap, so a pool reads as one blob folding
/// into itself, never as beads coming apart: at any point of the clock each
/// lobe is nearer the pool's centre than the largest lobe's radius.
#[test]
fn a_pool_never_comes_apart() {
    let mut b = Mercury::new(palette(), 1.6);
    for phase in [0.0, 0.13, 0.4, 0.77] {
        b.pose(phase);
        for (k, pool) in POOLS.iter().enumerate() {
            let c = b.centre(pool, phase);
            for i in 0..pool.lobes {
                let d = b.lobe(k, i).position - c;
                let len = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
                assert!(
                    len < lobe_radius(pool.size, 0),
                    "pool {k} lobe {i} strays {len:.2} at {phase}"
                );
            }
        }
    }
}

/// Every lobe stays in frame on a tall window and an ultrawide one, at any
/// point of the clock: pools are placed by the frustum, not in world units.
#[test]
fn every_pool_stays_in_frame_at_any_aspect() {
    for aspect in [0.6, 1.0, 1.6, 2.4, 3.0] {
        let mut b = Mercury::new(palette(), aspect);
        for phase in [0.0, 0.3, 0.7] {
            b.pose(phase);
            for (k, i) in every_lobe() {
                let (x, y) = ndc(&b, k, i);
                assert!(
                    x.abs() < 0.97 && y.abs() < 0.97,
                    "lobe {k}.{i} at aspect {aspect}, phase {phase}: ({x:.2}, {y:.2})"
                );
            }
        }
    }
}

/// It is chrome mirroring its studio in a fog of the page: the far pools
/// sink into the page, as prism's do.
#[test]
fn it_mirrors_its_studio_in_a_fog_of_the_page() {
    let t = crew_theme::ThemeId::Mercury.theme();
    let b = Mercury::new(palette(), 1.6);
    assert!(
        b.scene.environment.is_some(),
        "chrome needs a room to mirror"
    );
    let fog = b.scene.fog;
    assert_eq!(fog.mode, 1, "linear fog");
    assert_eq!(fog.color.r, color(t.page_bg).r);
    assert!(fog.near < EYE - 1.5 && fog.far > EYE + 5.0);
}
