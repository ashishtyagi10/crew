use super::*;

fn palette() -> Palette {
    Palette::of(crew_theme::ThemeId::Mercury.theme())
}

/// The studio the chrome mirrors: a floor darker than the horizon, a
/// softbox overhead brighter than anything, and a warm strip on one side
/// and a cool one on the other, in the theme's own colours.
#[test]
fn the_studio_has_a_floor_a_softbox_and_two_tempered_strips() {
    let p = palette();
    let luma = |c: [f32; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    let floor = studio_at(&p, [0.0, -1.0, 0.0]);
    let horizon = studio_at(&p, [0.0, 0.0, 1.0]);
    let top = studio_at(&p, [0.0, 1.0, 0.0]);
    assert!(luma(floor) < luma(horizon) && luma(horizon) < luma(top));
    assert!(luma(top) > 0.95, "the softbox is white");
    let at = |az: f32| studio_at(&p, [az.sin() * 0.95, 0.3, az.cos() * 0.95]);
    let (key, warm) = (at(-0.9), at(1.1));
    assert!(warm[0] > warm[2] + 0.1, "the rim strip is warm: {warm:?}");
    assert!(key[2] >= key[0], "the key strip is cool: {key:?}");
    assert!(luma(key) > luma(horizon), "the strips are lights");
    let cube = studio(&p);
    assert_eq!(cube.size, STUDIO);
    assert!(cube
        .faces
        .iter()
        .all(|f| f.len() == (STUDIO * STUDIO * 4) as usize));
}
