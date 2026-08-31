use super::*;

#[test]
fn readable_colours_pass_through_unchanged() {
    // Ink on paper and phosphor green on near-black both clear the floor.
    assert_eq!(
        ensure_min_contrast((22, 20, 18), (246, 243, 236)),
        (22, 20, 18)
    );
    assert_eq!(
        ensure_min_contrast((0, 255, 102), (3, 10, 5)),
        (0, 255, 102)
    );
}

#[test]
fn white_on_white_is_darkened_to_the_floor() {
    let bg = (246, 243, 236); // paper-light term_bg
    for fg in [(255, 255, 255), (235, 235, 235), (246, 243, 236)] {
        let fixed = ensure_min_contrast(fg, bg);
        assert!(
            ratio(fixed, bg) >= MIN_CONTRAST - 0.1,
            "{fg:?} → {fixed:?} ratio {}",
            ratio(fixed, bg)
        );
    }
}

#[test]
fn black_on_black_is_lightened_to_the_floor() {
    let bg = (8, 8, 8); // paper-dark term_bg
    for fg in [(0, 0, 0), (30, 30, 30), (8, 8, 8)] {
        let fixed = ensure_min_contrast(fg, bg);
        assert!(
            ratio(fixed, bg) >= MIN_CONTRAST - 0.1,
            "{fg:?} → {fixed:?} ratio {}",
            ratio(fixed, bg)
        );
    }
}

#[test]
fn hue_survives_the_nudge() {
    // A washed-out warm yellow on paper stays warm after darkening.
    let fixed = ensure_min_contrast((240, 220, 160), (246, 243, 236));
    assert!(
        fixed.0 > fixed.2,
        "red channel should stay dominant: {fixed:?}"
    );
    assert!(ratio(fixed, (246, 243, 236)) >= MIN_CONTRAST - 0.1);
}

#[test]
fn colorfgbg_matches_background_lightness() {
    assert_eq!(colorfgbg_for((246, 243, 236)), "0;15");
    assert_eq!(colorfgbg_for((8, 8, 8)), "15;0");
    assert_eq!(colorfgbg_for((3, 10, 5)), "15;0");
}
